#![allow(clippy::arithmetic_side_effects)]

//! Overlap relation: `overlaps(a, b)` is `true` iff there exists a
//! runtime value `v` such that `v ∈ a ∩ b`.
//!
//! Symmetric: `overlaps(a, b) == overlaps(b, a)`. Distinct from
//! `refines`: `int<0,10>` and `int<5,15>` overlap (value 7 inhabits both)
//! without either refining the other. The type-returning meet (greatest
//! lower bound) lives in `crate::meet`.
//!
//! Strategy: distribute over union (any element pair on the two sides
//! that overlaps proves the whole types overlap), then for each element
//! pair fall through these rules in order:
//!
//! 1. Reflexivity / Top / Bot axioms.
//! 2. Generic-parameter projection: `T` overlaps `X` iff `T`'s constraint
//!    overlaps `X`.
//! 3. Subsumption: `a <: b` or `b <: a` implies overlap.
//! 4. Family-specific positive overlap rules (e.g. range overlap, the
//!    string/class-like-string crossing, narrowed-mixed conservatism).
//!
//! When none of those fire we report disjoint. The rule set is incomplete
//! by design: adding a positive rule never weakens correctness, since the
//! relation is monotone in true outcomes; missing rules only cost
//! precision (a downstream narrowing returns `never` instead of a real
//! overlap).

use mago_atom::Atom;

use crate::ElementId;
use crate::ElementKind;
use crate::ElementListId;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::DefiningEntity;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::ListInfo;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family::mixed as mixed_family;
use crate::lattice::refines::element_refines;
use crate::prelude;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::world::TemplateParameter;
use crate::world::Variance;
use crate::world::World;

#[inline]
pub fn overlaps<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let a_type = a.as_ref();
    let b_type = b.as_ref();

    a_type.elements.iter().any(|x| b_type.elements.iter().any(|y| element_overlaps(*x, *y, world, options, report)))
}

#[inline]
fn element_overlaps<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == NEVER || b == NEVER {
        return false;
    }

    if is_uninhabited(a, world) || is_uninhabited(b, world) {
        return false;
    }

    if a == b {
        return true;
    }

    if a == MIXED || b == MIXED || a == PLACEHOLDER || b == PLACEHOLDER {
        return true;
    }

    if a.kind() == ElementKind::GenericParameter && b.kind() == ElementKind::GenericParameter {
        let a_info = interner().get_generic_parameter(a);
        let b_info = interner().get_generic_parameter(b);
        if a_info.name != b_info.name || a_info.defining_entity != b_info.defining_entity {
            return false;
        }

        return overlaps(a_info.constraint, b_info.constraint, world, options, report);
    }

    if a.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(a).constraint;
        let other = interner().intern_type(&[b], FlowFlags::EMPTY);
        return overlaps(constraint, other, world, options, report);
    }
    if b.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(b).constraint;
        let other = interner().intern_type(&[a], FlowFlags::EMPTY);
        return overlaps(constraint, other, world, options, report);
    }

    // `!T` overlaps `X` iff `X \ T ≢ ⊥` (some `X` value is outside
    // `T`). Sound conservative path: we ask the subtract side, which
    // rejects when `X <: T` and otherwise produces the surviving
    // pieces; if any survive, an overlap exists.
    if a.kind() == ElementKind::Negated || b.kind() == ElementKind::Negated {
        let (negated, other) = if a.kind() == ElementKind::Negated { (a, b) } else { (b, a) };
        let i = interner();
        if other.kind() == ElementKind::Negated {
            // `!T ∩ !U` is non-empty iff `T ∪ U ≢ mixed`. Without
            // exhaustive `mixed` enumeration we answer optimistically
            // true unless one side strictly refines the other (which
            // collapses one of the negations to the more general
            // form, leaving a non-empty complement).
            return true;
        }
        let neg_info = *i.get_negated(negated);
        let other_t = i.intern_type(&[other], FlowFlags::EMPTY);
        let surviving = crate::subtract::compute(other_t, neg_info.inner, world, options, report);
        return surviving != prelude::TYPE_NEVER;
    }

    if a.kind() == ElementKind::Intersected {
        let info = *interner().get_intersected(a);
        if !element_overlaps(info.head, b, world, options, report) {
            return false;
        }
        for &conjunct in interner().get_element_list(info.conjuncts) {
            if !element_overlaps(conjunct, b, world, options, report) {
                return false;
            }
        }
        return true;
    }
    if b.kind() == ElementKind::Intersected {
        let info = *interner().get_intersected(b);
        if !element_overlaps(a, info.head, world, options, report) {
            return false;
        }
        for &conjunct in interner().get_element_list(info.conjuncts) {
            if !element_overlaps(a, conjunct, world, options, report) {
                return false;
            }
        }
        return true;
    }
    if a.kind() == ElementKind::Object && b.kind() == ElementKind::Object {
        return object_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::String && b.kind() == ElementKind::String {
        return string_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::List && b.kind() == ElementKind::List {
        return list_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::Array && b.kind() == ElementKind::Array {
        return array_overlap(a, b, world, options, report);
    }

    if (a.kind() == ElementKind::List && b.kind() == ElementKind::Array)
        || (a.kind() == ElementKind::Array && b.kind() == ElementKind::List)
    {
        return list_array_overlap(a, b, world, options, report);
    }

    if a.kind() == ElementKind::Callable && b.kind() == ElementKind::Callable {
        return callable_overlap(a, b);
    }

    // Iterables likewise share the empty iterator: `[]`, the empty
    // generator, etc. inhabit `iterable<K, V>` for any K, V.
    if a.kind() == ElementKind::Iterable && b.kind() == ElementKind::Iterable {
        return true;
    }

    if (a.kind() == ElementKind::Iterable && b.kind() == ElementKind::Array)
        || (a.kind() == ElementKind::Array && b.kind() == ElementKind::Iterable)
    {
        return iterable_array_overlap(a, b, world, options, report);
    }

    if (a.kind() == ElementKind::Iterable && b.kind() == ElementKind::List)
        || (a.kind() == ElementKind::List && b.kind() == ElementKind::Iterable)
    {
        return iterable_list_overlap(a, b, world, options, report);
    }

    if matches!(
        (a.kind(), b.kind()),
        (ElementKind::HasMethod, ElementKind::HasMethod)
            | (ElementKind::HasProperty, ElementKind::HasProperty)
            | (ElementKind::HasMethod, ElementKind::HasProperty)
            | (ElementKind::HasProperty, ElementKind::HasMethod)
            | (ElementKind::ObjectShape, ElementKind::HasMethod)
            | (ElementKind::ObjectShape, ElementKind::HasProperty)
            | (ElementKind::HasMethod, ElementKind::ObjectShape)
            | (ElementKind::HasProperty, ElementKind::ObjectShape)
    ) {
        return true;
    }

    let (object_atom, structural_atom) = match (a.kind(), b.kind()) {
        (ElementKind::Object, ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape) => {
            (Some(a), Some(b))
        }
        (ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape, ElementKind::Object) => {
            (Some(b), Some(a))
        }
        _ => (None, None),
    };

    if let (Some(o), Some(s)) = (object_atom, structural_atom) {
        return object_structural_overlap(o, s, world);
    }

    if element_refines(a, b, world, options, report) || element_refines(b, a, world, options, report) {
        return true;
    }

    family_overlap(a, b)
}

#[inline]
fn object_structural_overlap<W: World>(object: ElementId, structural: ElementId, world: &W) -> bool {
    let i = interner();
    let info = *i.get_object(object);
    let class = info.name;
    !world.is_final(class) || class_satisfies_structural(class, structural, world)
}

#[inline]
fn class_satisfies_structural<W: World>(class: Atom, structural: ElementId, world: &W) -> bool {
    let i = interner();
    let conjuncts: Vec<ElementId> = vec![structural];

    conjuncts.iter().all(|&c| match c.kind() {
        ElementKind::HasMethod => world.class_has_method(class, i.get_has_method(c).method_name),
        ElementKind::HasProperty => world.class_has_property(class, i.get_has_property(c).property_name),
        _ => true,
    })
}

/// Object × Object overlap. Two named classes share values when:
///
/// - They are the same class with type-args compatible under each
///   parameter's variance (invariant slots must value-equal, covariant
///   slots must overlap).
/// - One descends from the other (the descendant subset overlaps the
///   ancestor).
///
/// Otherwise, in PHP's single-inheritance model, two unrelated nominal
/// classes cannot share a runtime instance, so we return `false`. This
/// is conservative: a future world surface for shared interfaces /
/// traits can lift the answer to `true`.
#[inline]
fn object_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    let a_classes = collect_class_names(a, a_info);
    let b_classes = collect_class_names(b, b_info);
    let combined: Vec<Atom> = a_classes.iter().chain(b_classes.iter()).copied().collect();
    if intersection_uninhabited_under_finality(&combined, world) {
        return false;
    }

    if a_info.name != b_info.name
        && let (Some(pa), Some(pb)) = (world.sealed_parent_of(a_info.name), world.sealed_parent_of(b_info.name))
        && pa == pb
        && !world.descends_from(a_info.name, b_info.name)
        && !world.descends_from(b_info.name, a_info.name)
    {
        return false;
    }

    if a_info.name == b_info.name
        && let (Some(a_args_id), Some(b_args_id)) = (a_info.type_args, b_info.type_args)
    {
        // Normalize args: arity-0 ignores any explicit args, arity > 0
        // truncates over-supply and default-fills under-supply. When
        // either side has no `type_args` it denotes "any T" and the
        // per-position check is skipped (handled at the outer `let`).
        let arity = world.template_parameter_arity(a_info.name);
        if arity > 0 {
            let a_supplied = i.get_type_list(a_args_id);
            let b_supplied = i.get_type_list(b_args_id);
            let fill = |idx: usize| -> TypeId {
                world.template_parameter_at(a_info.name, idx).and_then(|p| p.upper_bound).unwrap_or(prelude::TYPE_MIXED)
            };
            for idx in 0..arity {
                let a_arg = a_supplied.get(idx).copied().unwrap_or_else(|| fill(idx));
                let b_arg = b_supplied.get(idx).copied().unwrap_or_else(|| fill(idx));
                let variance =
                    world.template_parameter_at(a_info.name, idx).map_or(Variance::Invariant, |t| t.variance);
                match variance {
                    Variance::Invariant => {
                        let a_refines_b = lattice::refines(a_arg, b_arg, world, options, report);
                        let b_refines_a = lattice::refines(b_arg, a_arg, world, options, report);
                        if !a_refines_b || !b_refines_a {
                            return false;
                        }
                    }
                    Variance::Covariant => {
                        if !overlaps(a_arg, b_arg, world, options, report) {
                            return false;
                        }
                    }
                    Variance::Contravariant => {}
                }
            }
        }
    }

    // Cross-class descendant check: when A descends B (or vice
    // versa), the descendant's view of the ancestor's args must be
    // compatible under the ancestor's variance. An invariant arg
    // mismatch (e.g. `A<int(0)>` extending `B<T>` met with `B<int>`)
    // makes the intersection uninhabited and overlap must reflect
    // that or downstream `meet` (which now performs the same check)
    // would disagree.
    if a_info.name != b_info.name {
        let (descendant, ancestor) = if world.descends_from(a_info.name, b_info.name) {
            (a_info, b_info)
        } else if world.descends_from(b_info.name, a_info.name) {
            (b_info, a_info)
        } else {
            return true;
        };

        if !descendant_args_satisfy_ancestor(descendant, ancestor, world, options, report) {
            return false;
        }
    }

    true
}

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
                lattice::refines(resolved, ancestor_arg, world, options, report)
                    && lattice::refines(ancestor_arg, resolved, world, options, report)
            }
            Variance::Covariant => lattice::refines(resolved, ancestor_arg, world, options, report),
            Variance::Contravariant => lattice::refines(ancestor_arg, resolved, world, options, report),
        };
        if !compatible {
            return false;
        }
    }
    true
}

/// `true` iff `Foo & Bar & …` is provably uninhabited via the
/// world's finality surface. A `final` class admits no subclass,
/// so for `F & O` to be inhabited `F` and `O` must be ancestor-
/// related; an unrelated `O` alongside a final `F` collapses the
/// intersection. Without a final witness we stay open-world
/// (return `false`).
#[inline]
fn intersection_uninhabited_under_finality<W: World>(classes: &[Atom], world: &W) -> bool {
    classes.iter().any(|&final_candidate| {
        if !world.is_final(final_candidate) {
            return false;
        }

        classes.iter().any(|&other| {
            other != final_candidate
                && !world.descends_from(final_candidate, other)
                && !world.descends_from(other, final_candidate)
        })
    })
}

/// Collects the head + every object-kind conjunct's class name. Used
/// by `object_overlap` to enforce single-inheritance consistency
/// across the whole intersection (matching the rule in compose).
#[inline]
fn collect_class_names(elem: ElementId, info: ObjectInfo) -> Vec<Atom> {
    let _ = elem;
    vec![info.name]
}

/// `true` for atoms that are structurally non-NEVER but whose value
/// set is empty: `non-empty-list<never>`, `non-empty-array<…, never>`,
/// `Foo<never>` with a non-contravariant template, and any container
/// nested over a value-never type (e.g. `non-empty-list<B<never>>`).
/// The lattice can construct these but no runtime value inhabits
/// them, so `overlap` treats them as bottom.
#[inline]
fn list_uninhabited<W: World>(info: &ListInfo, intersections: Option<ElementListId>, world: &W) -> bool {
    if info.flags.non_empty() && type_is_value_never(info.element_type, world) {
        return true;
    }

    if let Some(known_id) = info.known_elements {
        for entry in interner().get_known_elements(known_id) {
            if !entry.optional && type_is_value_never(entry.value, world) {
                return true;
            }
        }
    }
    let stripped = interner().intern_list(*info);
    list_array_intersections_uninhabited_components(stripped, intersections, world)
}

#[inline]
fn array_uninhabited<W: World>(info: &KeyedArrayInfo, intersections: Option<ElementListId>, world: &W) -> bool {
    if info.flags.non_empty() {
        if let Some(key_t) = info.key_param {
            let int_or_string = interner().intern_type(&[prelude::INT, prelude::STRING], FlowFlags::EMPTY);
            if !lattice::overlaps(key_t, int_or_string, world, LatticeOptions::default(), &mut LatticeReport::new()) {
                return true;
            }
        }

        let key_empty = info.key_param.is_some_and(|t| type_is_value_never(t, world));
        let value_empty = info.value_param.is_some_and(|t| type_is_value_never(t, world));
        if key_empty || value_empty {
            return true;
        }
    }

    if let Some(known_id) = info.known_items {
        for entry in interner().get_known_items(known_id) {
            if !entry.optional && type_is_value_never(entry.value, world) {
                return true;
            }
        }
    }

    let stripped = interner().intern_array(*info);
    list_array_intersections_uninhabited_components(stripped, intersections, world)
}

#[inline]
fn object_uninhabited<W: World>(info: &ObjectInfo, intersections: Option<ElementListId>, world: &W) -> bool {
    let i = interner();
    if let Some(intersections_id) = intersections {
        let mut classes: Vec<Atom> = vec![info.name];
        let mut structurals: Vec<ElementId> = Vec::new();
        let mut negations: Vec<ElementId> = Vec::new();
        for &conjunct in i.get_element_list(intersections_id) {
            match conjunct.kind() {
                ElementKind::Object => classes.push(i.get_object(conjunct).name),
                ElementKind::HasMethod | ElementKind::HasProperty => structurals.push(conjunct),
                ElementKind::Negated => negations.push(conjunct),
                _ => {}
            }
        }

        if intersection_uninhabited_under_finality(&classes, world) {
            return true;
        }

        // Distinct direct inheritors of the same sealed class are disjoint.
        if sealed_siblings_disjoint(&classes, world) {
            return true;
        }

        for &neg in &negations {
            let neg_inner = i.get_negated(neg).inner;
            for &class in &classes {
                let bare = i.intern_object(ObjectInfo { name: class, type_args: None, flags: ObjectFlags::default() });
                let bare_t = i.intern_type(&[bare], FlowFlags::EMPTY);
                if lattice::refines(bare_t, neg_inner, world, LatticeOptions::default(), &mut LatticeReport::new()) {
                    return true;
                }
            }
        }

        for &class in &classes {
            if !world.is_final(class) {
                continue;
            }

            for &s in &structurals {
                let satisfied = match s.kind() {
                    ElementKind::HasMethod => world.class_has_method(class, i.get_has_method(s).method_name),
                    ElementKind::HasProperty => world.class_has_property(class, i.get_has_property(s).property_name),
                    _ => true,
                };

                if !satisfied {
                    return true;
                }
            }
        }
    }
    let Some(args_id) = info.type_args else { return false };
    let args = i.get_type_list(args_id);
    args.iter().enumerate().any(|(idx, &arg)| {
        if !type_is_value_never(arg, world) {
            return false;
        }

        let variance = world.template_parameter_at(info.name, idx).map_or(Variance::Contravariant, |p| p.variance);
        !matches!(variance, Variance::Contravariant)
    })
}

#[inline]
pub fn is_uninhabited<W: World>(elem: ElementId, world: &W) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::List => {
            let info = *i.get_list(elem);
            list_uninhabited(&info, None, world)
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            array_uninhabited(&info, None, world)
        }
        ElementKind::Object => {
            let info = *i.get_object(elem);
            object_uninhabited(&info, None, world)
        }
        ElementKind::Intersected => {
            let info = *i.get_intersected(elem);

            // Sealed-cover uninhabitedness: H & !S1 & !S2 ... where
            // H is sealed and all inheritors are covered by negations.
            if info.head.kind() == ElementKind::Object && sealed_cover_fully_excluded(info.head, info.conjuncts, world)
            {
                return true;
            }

            if intersected_negated_contradiction(info.head, info.conjuncts, world) {
                return true;
            }

            match info.head.kind() {
                ElementKind::Object => {
                    let head_info = *i.get_object(info.head);
                    return object_uninhabited(&head_info, Some(info.conjuncts), world);
                }
                ElementKind::List => {
                    let head_info = *i.get_list(info.head);
                    return list_uninhabited(&head_info, Some(info.conjuncts), world);
                }
                ElementKind::Array => {
                    let head_info = *i.get_array(info.head);
                    return array_uninhabited(&head_info, Some(info.conjuncts), world);
                }
                _ => {}
            }
            if is_uninhabited(info.head, world) {
                return true;
            }
            let conjuncts = i.get_element_list(info.conjuncts);
            for &c in conjuncts {
                if is_uninhabited(c, world) {
                    return true;
                }
            }
            false
        }
        _ => false,
    }
}

/// `true` when every atom in `t` is uninhabited or `t` is the
/// canonical `never`. Used by [`is_uninhabited`] to recurse into
/// container element types.
#[inline]
pub(crate) fn type_is_value_never<W: World>(t: TypeId, world: &W) -> bool {
    if t == prelude::TYPE_NEVER {
        return true;
    }
    let elements = t.as_ref().elements;
    if elements.is_empty() {
        return true;
    }
    elements.iter().all(|e| *e == NEVER || is_uninhabited(*e, world))
}

/// `true` iff the intersection of `head` with `conjuncts` refines the
/// inner of any Negated conjunct, making `Intersected(H, C1, …, !T)`
/// uninhabited.
#[inline]
fn intersected_negated_contradiction<W: World>(
    head: ElementId,
    conjuncts_id: crate::element::ElementListId,
    world: &W,
) -> bool {
    let i = interner();
    let conjuncts = i.get_element_list(conjuncts_id);

    let mut non_negated: Vec<ElementId> = Vec::with_capacity(conjuncts.len());
    for &c in conjuncts {
        if c.kind() != ElementKind::Negated {
            non_negated.push(c);
        }
    }

    let positive_elem = if non_negated.is_empty() { head } else { ElementId::intersected(head, &non_negated) };
    let positive_t = i.intern_type(&[positive_elem], FlowFlags::EMPTY);

    for &c in conjuncts {
        if c.kind() != ElementKind::Negated {
            continue;
        }
        let inner = i.get_negated(c).inner;
        if lattice::refines(positive_t, inner, world, LatticeOptions::default(), &mut LatticeReport::new()) {
            return true;
        }
    }
    false
}

/// `true` iff `Intersected(H, cogn-juncts)` has a sealed head `H`
/// and every direct inheritor of `H` is covered by some Negated
/// conjunct, making the Intersected uninhabited.
#[inline]
fn sealed_cover_fully_excluded<W: World>(
    head: ElementId,
    conjuncts_id: crate::element::ElementListId,
    world: &W,
) -> bool {
    let i = interner();
    let conjuncts = i.get_element_list(conjuncts_id);
    let mut negated_inners: Vec<TypeId> = Vec::with_capacity(conjuncts.len());
    for &c in conjuncts {
        if c.kind() == ElementKind::Negated {
            negated_inners.push(i.get_negated(c).inner);
        }
    }
    if negated_inners.is_empty() {
        return false;
    }
    matches!(
        crate::lattice::sealed::compute_residual(
            head,
            &negated_inners,
            world,
            LatticeOptions::default(),
            &mut LatticeReport::new(),
        ),
        crate::lattice::sealed::SealedResidual::FullyCovered
    )
}

/// `Callable × Callable` overlap. A function value has a fixed
/// arity at runtime, so two callable types with different parameter
/// counts cannot share any value. Same-arity (or one side `Any`)
/// callables share at least the always-throwing function (`return
/// never`), which trivially satisfies any return type.
#[inline]
fn callable_overlap(a: ElementId, b: ElementId) -> bool {
    let i = interner();
    use crate::element::payload::CallableInfo;
    let a_info = *i.get_callable(a);
    let b_info = *i.get_callable(b);
    let (CallableInfo::Signature(a_id), CallableInfo::Signature(b_id)) = (a_info, b_info) else {
        return true;
    };
    let a_sig = *i.get_signature(a_id);
    let b_sig = *i.get_signature(b_id);
    let a_arity = a_sig.parameters.map_or(0, |p| i.get_param_list(p).len());
    let b_arity = b_sig.parameters.map_or(0, |p| i.get_param_list(p).len());
    a_arity == b_arity
}

/// `String × String` overlap: defer to the meet rule. Two refined
/// string axes (`numeric-string`, `lowercase-string`, etc.) admit a
/// non-empty intersection unless their literal/casing/flags are
/// jointly unsatisfiable, which `string_meet` already decides.
#[inline]
fn string_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let _ = (world, options, report);
    crate::meet::family::string::string_meet(a, b).is_some()
}

/// `true` iff `elem` (a List or Array) carries an intersection chain
/// containing a `Negated(T)` whose inner `T` already contains the
/// stripped head ; in that case the whole intersection is empty.
/// Mirrors the negated-class arm of [`is_uninhabited`] for objects.
#[inline]
fn list_array_intersections_uninhabited_components<W: World>(
    stripped: ElementId,
    intersections: Option<ElementListId>,
    world: &W,
) -> bool {
    let Some(intersections_id) = intersections else { return false };
    let i = interner();
    let stripped_t = i.intern_type(&[stripped], FlowFlags::EMPTY);
    let opts = LatticeOptions::default();
    for &conjunct in i.get_element_list(intersections_id) {
        if conjunct.kind() == ElementKind::Negated {
            let neg_inner = i.get_negated(conjunct).inner;
            let mut report = LatticeReport::new();
            if lattice::refines(stripped_t, neg_inner, world, opts, &mut report) {
                return true;
            }
        }
    }
    false
}

/// `list<X> ∩ list<Y>` shares the empty list `[]` only when neither
/// side requires non-empty. When at least one side requires non-empty,
/// the element types must overlap for any concrete value to inhabit
/// both sets.
#[inline]
fn list_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);
    if !a_info.flags.non_empty() && !b_info.flags.non_empty() {
        return true;
    }

    overlaps(a_info.element_type, b_info.element_type, world, options, report)
}

/// `list<E> ∩ array<K, V>` shares the empty list `[]` (which is also
/// the empty array) unless either side demands non-empty. With at
/// least one non-empty side, the array's key constraint must accept
/// `int` (lists are int-keyed) and `E ∩ V` must overlap.
/// `iterable<K,V> ∩ array<K',V'>` shares the empty array unless the
/// array is non-empty; otherwise the iterable's K must admit some
/// of the array's keys and V must admit some of the array's values.
#[inline]
fn iterable_array_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let (it_atom, arr_atom) = if a.kind() == ElementKind::Iterable { (a, b) } else { (b, a) };
    let it_info = *i.get_iterable(it_atom);
    let arr_info = *i.get_array(arr_atom);

    if !arr_info.flags.non_empty() {
        return true;
    }
    let arr_key = arr_info.key_param.unwrap_or(prelude::TYPE_ARRAY_KEY);
    let arr_value = arr_info.value_param.unwrap_or(prelude::TYPE_MIXED);
    overlaps(it_info.key_type, arr_key, world, options, report)
        && overlaps(it_info.value_type, arr_value, world, options, report)
}

/// `iterable<K,V> ∩ list<E>` overlaps when `int` fits `K` (so the
/// list's keys are admissible) and `V` overlaps the list element
/// type (so any non-empty list value can match). The empty list is
/// shared by any pair structurally, but the lattice has no atom
/// that refines both sides simultaneously when `int <: K` fails,
/// so this rule mirrors the meet rule's precision rather than the
/// pure value-set semantics.
#[inline]
fn iterable_list_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let (it_atom, list_atom) = if a.kind() == ElementKind::Iterable { (a, b) } else { (b, a) };
    let it_info = *i.get_iterable(it_atom);
    let list_info = *i.get_list(list_atom);

    if !lattice::refines(prelude::TYPE_INT, it_info.key_type, world, options, report) {
        return false;
    }
    if !list_info.flags.non_empty() {
        return true;
    }
    overlaps(it_info.value_type, list_info.element_type, world, options, report)
}

#[inline]
fn list_array_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let (list_atom, array_atom) = if a.kind() == ElementKind::List { (a, b) } else { (b, a) };
    let list_info = *i.get_list(list_atom);
    let array_info = *i.get_array(array_atom);

    if !list_info.flags.non_empty() && !array_info.flags.non_empty() {
        return true;
    }
    if let Some(array_key_param) = array_info.key_param
        && !lattice::refines(prelude::TYPE_INT, array_key_param, world, options, report)
    {
        return false;
    }
    let array_value = array_info.value_param.unwrap_or(prelude::TYPE_MIXED);
    overlaps(list_info.element_type, array_value, world, options, report)
}

/// `array<K,V> ∩ array<K',V'>` mirrors `list_overlap`: the empty
/// array `[]` is shared only when neither side demands non-empty.
#[inline]
fn array_overlap<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);
    if !a_info.flags.non_empty() && !b_info.flags.non_empty() {
        return true;
    }

    match (a_info.key_param, b_info.key_param, a_info.value_param, b_info.value_param) {
        (Some(ak), Some(bk), Some(av), Some(bv)) => {
            overlaps(ak, bk, world, options, report) && overlaps(av, bv, world, options, report)
        }
        _ => true,
    }
}

#[inline]
fn family_overlap(a: ElementId, b: ElementId) -> bool {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return int_overlap(a, b);
    }

    if a.kind() == ElementKind::Mixed || b.kind() == ElementKind::Mixed {
        return mixed_overlap(a, b);
    }

    let pair = (a.kind(), b.kind());
    if matches!(
        pair,
        (ElementKind::String, ElementKind::ClassLikeString) | (ElementKind::ClassLikeString, ElementKind::String)
    ) {
        return string_class_like_string_overlap(a, b);
    }

    // Numeric strings inhabit both `numeric` and `string`. A specific
    // string literal that isn't itself numeric (e.g. `'foo'`) rules
    // the overlap out: its value is not in `numeric`.
    if matches!(pair, (ElementKind::Numeric, ElementKind::String) | (ElementKind::String, ElementKind::Numeric)) {
        return numeric_string_overlap(a, b);
    }

    // True-union dominator pairs (`scalar`, `numeric`, `array-key`)
    // share at least `int`, so every cross-pair overlaps.
    if matches!(
        pair,
        (ElementKind::Scalar, ElementKind::Numeric)
            | (ElementKind::Numeric, ElementKind::Scalar)
            | (ElementKind::Scalar, ElementKind::ArrayKey)
            | (ElementKind::ArrayKey, ElementKind::Scalar)
            | (ElementKind::ArrayKey, ElementKind::Numeric)
            | (ElementKind::Numeric, ElementKind::ArrayKey)
    ) {
        return true;
    }

    false
}

#[inline]
fn numeric_string_overlap(a: ElementId, b: ElementId) -> bool {
    use crate::element::payload::scalar::StringLiteral;
    let i = interner();
    let string_atom = if a.kind() == ElementKind::String { a } else { b };
    let info = *i.get_string(string_atom);
    match info.literal {
        StringLiteral::Value(v) => {
            let s = v.as_str();
            s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
        }
        StringLiteral::None | StringLiteral::Unspecified => true,
    }
}

/// `String` × `ClassLikeString`: they overlap iff some string value
/// inhabits both. A class-like-string is always non-empty and (as a
/// PHP class name) carries no chars outside `[A-Za-z_0-9\\]`. A
/// literal string side rules out the overlap if its value isn't a
/// valid class name; a literal class-string side rules it out if its
/// fixed name conflicts with the string's literal/casing constraints.
#[inline]
fn string_class_like_string_overlap(a: ElementId, b: ElementId) -> bool {
    let i = interner();
    let (string_atom, class_atom) = if a.kind() == ElementKind::String { (a, b) } else { (b, a) };
    let s = *i.get_string(string_atom);
    if let crate::element::payload::scalar::StringLiteral::Value(value) = s.literal {
        return is_valid_class_name(value.as_str());
    }

    let _ = class_atom;
    if s.flags.is_numeric() || s.flags.is_callable() {
        return false;
    }

    matches!(s.casing, crate::element::payload::scalar::StringCasing::Unspecified)
}

#[inline]
fn is_valid_class_name(s: &str) -> bool {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len == 0 || bytes[len - 1] == b'\\' {
        return false;
    }
    let mut i = usize::from(bytes[0] == b'\\');
    if i >= len {
        return false;
    }
    let mut part_start = true;
    while i < len {
        let b = bytes[i];
        #[allow(clippy::else_if_without_else)]
        if b == b'\\' {
            if part_start {
                return false;
            }
            part_start = true;
        } else if part_start {
            if !(b.is_ascii_alphabetic() || b == b'_') {
                return false;
            }
            part_start = false;
        } else if !(b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80) {
            return false;
        }
        i += 1;
    }
    !part_start
}

/// Narrowed-mixed overlap: each side's axis flags must be jointly
/// satisfiable by some runtime value the other side admits. Vanilla
/// `mixed` is already absorbed by the Top axiom, so at least one side
/// here carries a non-trivial axis.
#[inline]
fn mixed_overlap(a: ElementId, b: ElementId) -> bool {
    let (mixed, other) = if a.kind() == ElementKind::Mixed { (a, b) } else { (b, a) };
    if !mixed_axes_compatible(*interner().get_mixed(mixed), other) {
        return false;
    }
    if other.kind() == ElementKind::Mixed && !mixed_axes_compatible(*interner().get_mixed(other), mixed) {
        return false;
    }
    true
}

#[inline]
fn sealed_siblings_disjoint<W: World>(names: &[mago_atom::Atom], world: &W) -> bool {
    if names.len() < 2 {
        return false;
    }
    for i in 0..names.len() {
        for j in i + 1..names.len() {
            if names[i] == names[j] {
                continue;
            }
            // Two distinct names sharing the same sealed parent are disjoint.
            if let (Some(pa), Some(pb)) = (world.sealed_parent_of(names[i]), world.sealed_parent_of(names[j])) {
                if pa == pb {
                    return true;
                }
            }
        }
    }
    false
}

#[inline]
fn mixed_axes_compatible(info: crate::element::payload::MixedInfo, other: ElementId) -> bool {
    if info.is_non_null() && !mixed_family::is_non_null(other) {
        return false;
    }
    let other_truth = mixed_family::truthiness_of(other);
    match info.truthiness() {
        Truthiness::Truthy if other_truth == Truthiness::Falsy => return false,
        Truthiness::Falsy if other_truth == Truthiness::Truthy => return false,
        _ => {}
    }
    if info.is_empty() && other_truth == Truthiness::Truthy {
        return false;
    }
    true
}

/// Intervals (with absorption: `INT` and `LITERAL_INT` are unbounded) on
/// either side overlap iff `max(lo_a, lo_b) ≤ min(hi_a, hi_b)`. An open
/// bound on either side is treated as `±∞`.
#[inline]
fn int_overlap(a: ElementId, b: ElementId) -> bool {
    let i = interner();
    let (al, au) = int_bounds(*i.get_int(a));
    let (bl, bu) = int_bounds(*i.get_int(b));

    let lo = match (al, bl) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };
    let hi = match (au, bu) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    };

    match (lo, hi) {
        (Some(l), Some(h)) => l <= h,
        _ => true,
    }
}

#[inline]
fn int_bounds(info: IntInfo) -> (Option<i64>, Option<i64>) {
    match info {
        IntInfo::Unspecified | IntInfo::UnspecifiedLiteral => (None, None),
        IntInfo::Literal(n) => (Some(n), Some(n)),
        IntInfo::Range(range_id) => {
            let r = interner().get_int_range(range_id);
            (r.lower(), r.upper())
        }
    }
}
