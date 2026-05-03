//! Compositional object intersection. Two paths:
//!
//! - Different classes (or one with no shared `name`): glue them as a
//!   single intersection-bearing object (`Foo & Bar`), choosing the
//!   canonical-smallest participant as the head.
//! - Same class, different generic arguments: merge args pointwise
//!   under the world-declared variance. Invariant args meet (must
//!   agree); covariant args meet; contravariant args join. If any
//!   invariant slot meets to `never`, the whole intersection is
//!   uninhabitable and we return `None`.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::TypeListId;
use crate::element::payload::DefiningEntity;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::prelude::TYPE_NEVER;
use crate::world::TemplateParameter;
use crate::world::Variance;
use crate::world::World;

pub(in crate::meet) fn compose_object_intersection<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    let mut participants: Vec<ElementId> = Vec::new();
    participants.push(i.intern_object(ObjectInfo { intersections: None, ..a_info }));
    if let Some(id) = a_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    participants.push(i.intern_object(ObjectInfo { intersections: None, ..b_info }));
    if let Some(id) = b_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    let same_class_merged = merge_same_class_participants(participants, world, options, report)?;
    let reconciled = reconcile_descendant_participants(same_class_merged, world, options, report)?;

    finalize_object_composition(reconciled, world)
}

/// Reconcile pairs of object participants where one nominally
/// descends the other. The descendant's view of the ancestor (via
/// `World::inherited_template_argument`) must be compatible with the
/// ancestor's args under the ancestor's variance; if not, the
/// intersection is uninhabited (`None`). When compatible, the
/// ancestor is redundant (the descendant is strictly more specific)
/// and we drop it from the merged list.
#[inline]
fn reconcile_descendant_participants<W: World>(
    mut merged: Vec<ElementId>,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    let i = interner();
    let mut keep: Vec<bool> = vec![true; merged.len()];

    for descendant_idx in 0..merged.len() {
        if !keep[descendant_idx] || merged[descendant_idx].kind() != ElementKind::Object {
            continue;
        }
        let descendant_info = *i.get_object(merged[descendant_idx]);

        for ancestor_idx in 0..merged.len() {
            if descendant_idx == ancestor_idx
                || !keep[ancestor_idx]
                || merged[ancestor_idx].kind() != ElementKind::Object
            {
                continue;
            }
            let ancestor_info = *i.get_object(merged[ancestor_idx]);
            if descendant_info.name == ancestor_info.name {
                continue;
            }
            if !world.descends_from(descendant_info.name, ancestor_info.name) {
                continue;
            }
            if !descendant_args_satisfy_ancestor(descendant_info, ancestor_info, world, options, report) {
                return None;
            }
            // Splice the ancestor's `Negated` conjuncts into the
            // descendant before dropping the ancestor. A negation
            // that covers the descendant's class collapses the
            // meet to `None`.
            if let Some(ancestor_intersections) = ancestor_info.intersections {
                let mut new_conjuncts: Vec<ElementId> =
                    descendant_info.intersections.map(|id| i.get_element_list(id).to_vec()).unwrap_or_default();
                for &conjunct in i.get_element_list(ancestor_intersections) {
                    if conjunct.kind() == ElementKind::Negated
                        && negation_excludes_class(conjunct, descendant_info.name, world)
                    {
                        return None;
                    }
                    if !new_conjuncts.contains(&conjunct) {
                        new_conjuncts.push(conjunct);
                    }
                }
                new_conjuncts.sort();
                let new_id = i.intern_element_list(&new_conjuncts);
                if Some(new_id) != descendant_info.intersections {
                    merged[descendant_idx] =
                        i.intern_object(ObjectInfo { intersections: Some(new_id), ..descendant_info });
                }
            }
            keep[ancestor_idx] = false;
        }
    }

    Some(merged.into_iter().zip(keep).filter_map(|(elem, k)| k.then_some(elem)).collect())
}

/// `true` iff `negated_atom` (a `Negated` conjunct) excludes every
/// instance of class `class_name`. Today this fires for negations
/// of a bare-named ancestor of `class_name`: every instance of
/// `class_name` is also an instance of the ancestor, so the
/// negation rules them all out.
#[inline]
fn negation_excludes_class<W: World>(negated_atom: ElementId, class_name: mago_atom::Atom, world: &W) -> bool {
    let i = interner();
    let neg_info = *i.get_negated(negated_atom);
    let elements = neg_info.inner.as_ref().elements;
    if !crate::element::simd::any_of_kind(elements, ElementKind::Object) {
        return false;
    }

    elements.iter().any(|&inner| {
        if inner.kind() != ElementKind::Object {
            return false;
        }
        let inner_info = *i.get_object(inner);
        if inner_info.intersections.is_some() {
            return false;
        }
        world.descends_from(class_name, inner_info.name)
    })
}

/// Project `descendant`'s view of `ancestor` through the world's
/// inherited-template-argument rule and substitute `descendant`'s
/// actual args, then check each position against `ancestor`'s args
/// under `ancestor`'s variance.
#[inline]
fn descendant_args_satisfy_ancestor<W: World>(
    descendant: ObjectInfo,
    ancestor: ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let arity = world.template_parameter_arity(ancestor.name);
    if arity == 0 {
        return true;
    }

    let ancestor_args: Vec<TypeId> = match ancestor.type_args {
        Some(id) => i.get_type_list(id).to_vec(),
        None => return true,
    };
    if ancestor_args.len() != arity {
        return true;
    }

    let descendant_actuals: Vec<TypeId> =
        descendant.type_args.map(|id| i.get_type_list(id).to_vec()).unwrap_or_default();

    for (position, &ancestor_arg) in ancestor_args.iter().enumerate() {
        let Some(inherited) = world.inherited_template_argument(descendant.name, ancestor.name, position) else {
            return true;
        };
        let resolved = crate::template::substitute(inherited, &|info: &GenericParameterInfo| -> Option<TypeId> {
            let defining = *i.get_defining_entity(info.defining_entity);
            if defining != DefiningEntity::ClassLike(descendant.name) {
                return None;
            }
            let pos = world.template_parameter_index(descendant.name, info.name)?;
            descendant_actuals.get(pos).copied()
        });

        let variance = world
            .template_parameter_at(ancestor.name, position)
            .map(|p: TemplateParameter| p.variance)
            .unwrap_or_default();

        let compatible = match variance {
            Variance::Invariant => {
                crate::lattice::refines(resolved, ancestor_arg, world, options, report)
                    && crate::lattice::refines(ancestor_arg, resolved, world, options, report)
            }
            Variance::Covariant => crate::lattice::refines(resolved, ancestor_arg, world, options, report),
            Variance::Contravariant => crate::lattice::refines(ancestor_arg, resolved, world, options, report),
        };
        if !compatible {
            return false;
        }
    }
    true
}

/// `object{...} ∩ has-method<m>` (or `has-property<p>` /
/// `ObjectShape`): the shape never guarantees the structural,
/// and its known properties may be optional or the shape unsealed,
/// so the intersection adds the structural to the shape's
/// `intersections` list rather than dropping it.
pub(in crate::meet) fn compose_shape_with_structural(shape: ElementId, structural: ElementId) -> Option<ElementId> {
    let i = interner();
    let shape_info = *i.get_object_shape(shape);
    let mut conjuncts: Vec<ElementId> =
        shape_info.intersections.map(|id| i.get_element_list(id).to_vec()).unwrap_or_default();

    if !conjuncts.contains(&structural) {
        conjuncts.push(structural);
    }

    conjuncts.sort();
    let intersections = Some(i.intern_element_list(&conjuncts));
    Some(i.intern_object_shape(ObjectShapeInfo { intersections, ..shape_info }))
}

/// Compose a nominal object atom with a structural conjunct
/// (`HasMethod`, `HasProperty`, `ObjectShape`). An unknown class
/// might gain the structural feature via a subclass, so the
/// intersection stays alive. A final class that doesn't satisfy
/// the structural collapses to `None`. When the world already
/// records that a positive class in the intersection has the
/// method/property, the redundant conjunct is dropped.
pub(in crate::meet) fn compose_object_with_structural<W: World>(
    object: ElementId,
    structural: ElementId,
    world: &W,
) -> Option<ElementId> {
    let i = interner();
    let object_info = *i.get_object(object);

    let mut nominal_classes: Vec<mago_atom::Atom> = vec![object_info.name];
    if let Some(id) = object_info.intersections {
        for &conjunct in i.get_element_list(id) {
            if conjunct.kind() == ElementKind::Object {
                nominal_classes.push(i.get_object(conjunct).name);
            }
        }
    }

    if structural_uninhabited_under_finality(&nominal_classes, structural, world) {
        return None;
    }

    let drop_as_redundant = matches!(structural.kind(), ElementKind::HasMethod | ElementKind::HasProperty)
        && nominal_classes.iter().any(|&c| class_satisfies_structural(c, structural, world));
    if drop_as_redundant {
        return Some(object);
    }

    let mut participants: Vec<ElementId> = Vec::new();
    participants.push(i.intern_object(ObjectInfo { intersections: None, ..object_info }));
    if let Some(id) = object_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    participants.push(structural);

    finalize_object_composition(participants, world)
}

/// `final C & HasMethod(m)` is uninhabited when `C` is final and the
/// world says it lacks `m`: a final class admits no subclass that
/// could add the member. The check fires only for nominal classes
/// the world declares final; open-world classes always keep the
/// structural intersection.
#[inline]
fn structural_uninhabited_under_finality<W: World>(
    classes: &[mago_atom::Atom],
    structural: ElementId,
    world: &W,
) -> bool {
    classes.iter().any(|&class| world.is_final(class) && !class_satisfies_structural(class, structural, world))
}

#[inline]
fn class_satisfies_structural<W: World>(class: mago_atom::Atom, structural: ElementId, world: &W) -> bool {
    let i = interner();
    let mut conjuncts: Vec<ElementId> = vec![structural];
    let nested = match structural.kind() {
        ElementKind::HasMethod => i.get_has_method(structural).intersections,
        ElementKind::HasProperty => i.get_has_property(structural).intersections,
        _ => None,
    };

    if let Some(id) = nested {
        conjuncts.extend_from_slice(i.get_element_list(id));
    }

    conjuncts.iter().all(|&c| match c.kind() {
        ElementKind::HasMethod => world.class_has_method(class, i.get_has_method(c).method_name),
        ElementKind::HasProperty => world.class_has_property(class, i.get_has_property(c).property_name),
        _ => true,
    })
}

#[inline]
fn finalize_object_composition<W: World>(merged: Vec<ElementId>, world: &W) -> Option<ElementId> {
    let i = interner();

    let mut object_parts: Vec<ElementId> = Vec::new();
    let mut other_parts: Vec<ElementId> = Vec::new();
    for elem in merged {
        if elem.kind() == ElementKind::Object {
            object_parts.push(elem);
        } else {
            other_parts.push(elem);
        }
    }

    if !single_inheritance_consistent(&object_parts, world) {
        return None;
    }

    let has_negated = crate::element::simd::any_of_kind(&other_parts, ElementKind::Negated);
    if has_negated {
        for &neg in other_parts.iter().filter(|e| e.kind() == ElementKind::Negated) {
            for &obj in &object_parts {
                if negation_excludes_class(neg, i.get_object(obj).name, world) {
                    return None;
                }
            }

            // `(X & !X)` shape: a `Negated` whose inner accepts another
            // conjunct in the same intersection is contradictory.
            let neg_inner = i.get_negated(neg).inner;
            for &positive in &other_parts {
                if positive == neg {
                    continue;
                }

                let pos_t = i.intern_type(&[positive], FlowFlags::EMPTY);
                if crate::lattice::refines(
                    pos_t,
                    neg_inner,
                    world,
                    crate::lattice::LatticeOptions::default(),
                    &mut crate::lattice::LatticeReport::new(),
                ) {
                    return None;
                }
            }
        }
    } // end has_negated guard

    object_parts.sort();
    object_parts.dedup();
    other_parts.sort();
    other_parts.dedup();

    let head_elem = object_parts.remove(0);
    let head_info = *i.get_object(head_elem);
    let mut conjuncts = object_parts;
    conjuncts.extend(other_parts);
    let intersections = if conjuncts.is_empty() { None } else { Some(i.intern_element_list(&conjuncts)) };
    let result = i.intern_object(ObjectInfo { intersections, ..head_info });
    Some(result)
}

/// `Foo & Bar & …` is inhabitable when no finality witness rules it
/// out. A `final` class in the intersection has only itself as a
/// possible witness, so it must descend every other class in the
/// intersection. When that fails, the type is uninhabited and
/// compose collapses to `None`. Without a final witness we
/// optimistically allow the composition (PHP's open world might
/// supply a common subclass via interfaces / traits).
#[inline]
fn single_inheritance_consistent<W: World>(objects: &[ElementId], world: &W) -> bool {
    let i = interner();
    let names: Vec<mago_atom::Atom> = objects.iter().map(|o| i.get_object(*o).name).collect();
    for &final_candidate in &names {
        if !world.is_final(final_candidate) {
            continue;
        }
        for &other in &names {
            if other == final_candidate {
                continue;
            }
            if !world.descends_from(final_candidate, other) && !world.descends_from(other, final_candidate) {
                return false;
            }
        }
    }
    true
}

#[inline]
fn merge_same_class_participants<W: World>(
    participants: Vec<ElementId>,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    let i = interner();
    let mut out: Vec<ElementId> = Vec::with_capacity(participants.len());

    for elem in participants {
        if elem.kind() != ElementKind::Object {
            out.push(elem);
            continue;
        }

        let info = *i.get_object(elem);
        let mut absorbed = false;
        for slot in &mut out {
            if slot.kind() != ElementKind::Object {
                continue;
            }

            let existing = *i.get_object(*slot);
            if existing.name != info.name {
                continue;
            }

            let merged_args = merge_args(existing, info, world, options, report)?;
            *slot = i.intern_object(ObjectInfo {
                name: info.name,
                type_args: merged_args,
                intersections: None,
                flags: info.flags,
            });
            absorbed = true;
            break;
        }

        if !absorbed {
            out.push(elem);
        }
    }

    Some(out)
}

#[inline]
fn merge_args<W: World>(
    a: ObjectInfo,
    b: ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Option<TypeListId>> {
    let i = interner();
    let arity = world.template_parameter_arity(a.name);

    // Arity-0 classes can carry meaningless explicit args; collapse
    // both sides to the bare nominal form so the same atom is
    // produced regardless of how each side was constructed.
    if arity == 0 {
        return Some(None);
    }

    match (a.type_args, b.type_args) {
        (None, None) => Some(None),
        (Some(id), None) | (None, Some(id)) => {
            let all_contravariant = (0..arity).all(|idx| {
                matches!(world.template_parameter_at(a.name, idx).map(|t| t.variance), Some(Variance::Contravariant))
            });

            if all_contravariant { Some(None) } else { Some(Some(id)) }
        }
        (Some(a_id), Some(b_id)) => {
            // Normalize both sides to exactly `arity` positions:
            // truncate over-supply, default-fill under-supply.
            let a_supplied: &[TypeId] = i.get_type_list(a_id);
            let b_supplied: &[TypeId] = i.get_type_list(b_id);
            let fill = |idx: usize| -> TypeId {
                world
                    .template_parameter_at(a.name, idx)
                    .and_then(|p| p.upper_bound)
                    .unwrap_or(crate::prelude::TYPE_MIXED)
            };
            let a_args: Vec<TypeId> =
                (0..arity).map(|idx| a_supplied.get(idx).copied().unwrap_or_else(|| fill(idx))).collect();
            let b_args: Vec<TypeId> =
                (0..arity).map(|idx| b_supplied.get(idx).copied().unwrap_or_else(|| fill(idx))).collect();

            let mut merged: Vec<TypeId> = Vec::with_capacity(arity);
            for (idx, (&a_arg, &b_arg)) in a_args.iter().zip(b_args.iter()).enumerate() {
                let variance = world.template_parameter_at(a.name, idx).map_or(Variance::Invariant, |t| t.variance);
                let arg = match variance {
                    Variance::Covariant => crate::meet::compute(a_arg, b_arg, world, options, report),
                    Variance::Invariant => {
                        // Invariant slots require args to be mutually
                        // refining (i.e. value-equal). Mere non-empty
                        // intersection isn't enough: `B<int|enum>` and
                        // `B<int>` admit no shared B-instances when T is
                        // pinned exactly.
                        let a_refines_b = crate::lattice::refines(a_arg, b_arg, world, options, report);
                        let b_refines_a = crate::lattice::refines(b_arg, a_arg, world, options, report);
                        if !a_refines_b || !b_refines_a {
                            return None;
                        }
                        a_arg
                    }
                    Variance::Contravariant => {
                        let mut elems: Vec<ElementId> = a_arg.as_ref().elements.to_vec();
                        elems.extend_from_slice(b_arg.as_ref().elements);
                        i.intern_type(&elems, FlowFlags::EMPTY)
                    }
                };

                if matches!(variance, Variance::Covariant) && arg == TYPE_NEVER {
                    return None;
                }

                merged.push(arg);
            }

            Some(Some(i.intern_type_list(&merged)))
        }
    }
}
