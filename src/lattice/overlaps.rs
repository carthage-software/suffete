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
//! 2. Generic-parameter projection — `T` overlaps `X` iff `T`'s constraint
//!    overlaps `X`.
//! 3. Subsumption — `a <: b` or `b <: a` implies overlap.
//! 4. Family-specific positive overlap rules (e.g. range overlap, the
//!    string/class-like-string crossing, narrowed-mixed conservatism).
//!
//! When none of those fire we report disjoint. The rule set is incomplete
//! by design — adding a positive rule never weakens correctness, since the
//! relation is monotone in true outcomes; missing rules only cost
//! precision (a downstream narrowing returns `never` instead of a real
//! overlap).

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family::mixed as mixed_family;
use crate::lattice::refines::element_refines;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::world::Variance;
use crate::world::World;

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
        return surviving != crate::prelude::TYPE_NEVER;
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

fn object_structural_overlap<W: World>(object: ElementId, structural: ElementId, world: &W) -> bool {
    let i = interner();
    let info = *i.get_object(object);
    let mut classes: Vec<mago_atom::Atom> = vec![info.name];
    if let Some(id) = info.intersections {
        for &c in i.get_element_list(id) {
            if c.kind() == ElementKind::Object {
                classes.push(i.get_object(c).name);
            }
        }
    }

    !classes.iter().any(|&class| world.is_final(class) && !class_satisfies_structural(class, structural, world))
}

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
    let combined: Vec<mago_atom::Atom> = a_classes.iter().chain(b_classes.iter()).copied().collect();
    if intersection_uninhabited_under_finality(&combined, world) {
        return false;
    }

    // A `Negated` conjunct on either side that subsumes the other
    // side's whole value-set rules out the overlap: every value of
    // the other side falls inside the negation, leaving nothing
    // shared. This is the value-equality form of the
    // `negation_excludes_class` check used in `meet`, generalized
    // to arbitrary inner shapes (template-arg mismatches under
    // contravariance, structural conjuncts, etc.).
    if negation_covers_other(a_info, b, world, options, report)
        || negation_covers_other(b_info, a, world, options, report)
    {
        return false;
    }

    if a_info.name == b_info.name
        && let (Some(a_args_id), Some(b_args_id)) = (a_info.type_args, b_info.type_args)
    {
        // Arity normalization mirrors `refines_named_named`:
        // arity-0 classes ignore any explicit args; arity > 0
        // classes truncate over-supply and default-fill under-supply
        // before per-position checks. When either side omits
        // `type_args` entirely it denotes "any T" and the
        // per-position check is skipped (handled at the outer let).
        let arity = world.template_parameter_arity(a_info.name);
        if arity > 0 {
            let a_supplied = i.get_type_list(a_args_id);
            let b_supplied = i.get_type_list(b_args_id);
            let fill = |idx: usize| -> TypeId {
                world
                    .template_parameter_at(a_info.name, idx)
                    .and_then(|p| p.upper_bound)
                    .unwrap_or(crate::prelude::TYPE_MIXED)
            };
            for idx in 0..arity {
                let a_arg = a_supplied.get(idx).copied().unwrap_or_else(|| fill(idx));
                let b_arg = b_supplied.get(idx).copied().unwrap_or_else(|| fill(idx));
                let variance =
                    world.template_parameter_at(a_info.name, idx).map(|t| t.variance).unwrap_or(Variance::Invariant);
                match variance {
                    Variance::Invariant => {
                        let a_refines_b = crate::lattice::refines(a_arg, b_arg, world, options, report);
                        let b_refines_a = crate::lattice::refines(b_arg, a_arg, world, options, report);
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
        // Any `Negated` conjunct on the ancestor that subsumes the
        // descendant's nominal class makes the intersection
        // uninhabited: the ancestor's value-set rules out every
        // class in the descendant's subtree.
        if let Some(id) = ancestor.intersections {
            let i = interner();
            for &conjunct in i.get_element_list(id) {
                if conjunct.kind() != ElementKind::Negated {
                    continue;
                }
                let neg_info = *i.get_negated(conjunct);
                let inner_elements = neg_info.inner.as_ref().elements;
                let inner_covers_descendant = inner_elements.iter().any(|&inner| {
                    if inner.kind() != ElementKind::Object {
                        return false;
                    }
                    let inner_info = *i.get_object(inner);
                    if inner_info.intersections.is_some() {
                        return false;
                    }
                    world.descends_from(descendant.name, inner_info.name)
                });
                if inner_covers_descendant {
                    return false;
                }
            }
        }
    }

    true
}

/// `true` iff some `Negated` conjunct in `info`'s intersections
/// covers the entire value-set of `other`: every runtime value of
/// `other` would land inside the negation, so an `info ∩ other`
/// instance cannot exist.
fn negation_covers_other<W: World>(
    info: crate::element::payload::ObjectInfo,
    other: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let Some(intersections_id) = info.intersections else { return false };
    let i = interner();
    let other_t = i.intern_type(&[other], FlowFlags::EMPTY);
    for &conjunct in i.get_element_list(intersections_id) {
        if conjunct.kind() != ElementKind::Negated {
            continue;
        }
        let neg_info = *i.get_negated(conjunct);
        if crate::lattice::refines(other_t, neg_info.inner, world, options, report) {
            return true;
        }
    }
    false
}

fn descendant_args_satisfy_ancestor<W: World>(
    descendant: crate::element::payload::ObjectInfo,
    ancestor: crate::element::payload::ObjectInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    use crate::element::payload::DefiningEntity;
    use crate::element::payload::GenericParameterInfo;
    use crate::world::TemplateParameter;

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

/// `true` iff `Foo & Bar & …` is provably uninhabited via the
/// world's finality surface. A `final` class `F` admits no new
/// subclasses, so for `F & O` to be inhabited the value must
/// satisfy both `F` and `O` simultaneously — which is only
/// possible when `F` and `O` are ancestor-related in *some*
/// direction (`F` descends `O`, or `O` descends `F`). When `F` is
/// final and there exists an unrelated `O` in the intersection,
/// no runtime value can satisfy both and the type is bottom.
/// Without a final witness we stay open-world-conservative
/// (return `false`).
fn intersection_uninhabited_under_finality<W: World>(classes: &[mago_atom::Atom], world: &W) -> bool {
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
fn collect_class_names(elem: ElementId, info: crate::element::payload::ObjectInfo) -> Vec<mago_atom::Atom> {
    let i = interner();
    let mut names = vec![info.name];
    if let Some(id) = info.intersections {
        for &conjunct in i.get_element_list(id) {
            if conjunct.kind() == ElementKind::Object {
                names.push(i.get_object(conjunct).name);
            }
        }
    }
    let _ = elem;
    names
}

/// `true` for atoms that are structurally non-NEVER but whose value
/// set is empty: `non-empty-list<never>`, `non-empty-array<…, never>`,
/// `Foo<never>` with a non-contravariant template, and any container
/// nested over a value-never type (e.g. `non-empty-list<B<never>>`).
/// The lattice can construct these but no runtime value inhabits
/// them, so `overlap` treats them as bottom.
pub(crate) fn is_uninhabited<W: World>(elem: ElementId, world: &W) -> bool {
    let i = interner();
    match elem.kind() {
        ElementKind::List => {
            let info = *i.get_list(elem);
            info.flags.non_empty() && type_is_value_never(info.element_type, world)
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            if !info.flags.non_empty() {
                return false;
            }
            let key_empty = info.key_param.is_some_and(|t| type_is_value_never(t, world));
            let value_empty = info.value_param.is_some_and(|t| type_is_value_never(t, world));
            key_empty || value_empty
        }
        ElementKind::Object => {
            let info = *i.get_object(elem);
            if let Some(intersections_id) = info.intersections {
                let mut classes: Vec<mago_atom::Atom> = vec![info.name];
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

                // A `Negated` conjunct that subsumes any positive
                // class in the intersection makes it uninhabited:
                // every positive instance falls inside the negation.
                // Mirrors the value-set rule used by `compose_object_intersection`'s
                // `negation_excludes_class`, but without bespoke
                // descent-only logic — we ask the lattice whether the
                // bare nominal class refines the negation's inner.
                for &neg in &negations {
                    let neg_inner = i.get_negated(neg).inner;
                    for &class in &classes {
                        let bare = i.intern_object(crate::element::payload::ObjectInfo {
                            name: class,
                            type_args: None,
                            intersections: None,
                            flags: crate::element::payload::ObjectFlags::default(),
                        });
                        let bare_t = i.intern_type(&[bare], FlowFlags::EMPTY);
                        if crate::lattice::refines(
                            bare_t,
                            neg_inner,
                            world,
                            crate::lattice::LatticeOptions::default(),
                            &mut crate::lattice::LatticeReport::new(),
                        ) {
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
                            ElementKind::HasProperty => {
                                world.class_has_property(class, i.get_has_property(s).property_name)
                            }
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

                let variance =
                    world.template_parameter_at(info.name, idx).map(|p| p.variance).unwrap_or(Variance::Contravariant);
                !matches!(variance, Variance::Contravariant)
            })
        }
        _ => false,
    }
}

/// `true` when every atom in `t` is uninhabited or `t` is the
/// canonical `never`. Used by [`is_uninhabited`] to recurse into
/// container element types.
pub(crate) fn type_is_value_never<W: World>(t: TypeId, world: &W) -> bool {
    if t == crate::prelude::TYPE_NEVER {
        return true;
    }
    let elements = t.as_ref().elements;
    if elements.is_empty() {
        return true;
    }
    elements.iter().all(|e| *e == NEVER || is_uninhabited(*e, world))
}

/// `Callable × Callable` overlap. A function value has a fixed
/// arity at runtime, so two callable types with different parameter
/// counts cannot share any value. Same-arity (or one side `Any`)
/// callables share at least the always-throwing function (`return
/// never`), which trivially satisfies any return type.
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
    let a_arity = a_sig.parameters.map(|p| i.get_param_list(p).len()).unwrap_or(0);
    let b_arity = b_sig.parameters.map(|p| i.get_param_list(p).len()).unwrap_or(0);
    a_arity == b_arity
}

/// `String × String` overlap: defer to the meet rule. Two refined
/// string axes (`numeric-string`, `lowercase-string`, etc.) admit a
/// non-empty intersection unless their literal/casing/flags are
/// jointly unsatisfiable, which `string_meet` already decides.
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

/// `list<X> ∩ list<Y>` shares the empty list `[]` only when neither
/// side requires non-empty. When at least one side requires non-empty,
/// the element types must overlap for any concrete value to inhabit
/// both sets.
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
    let arr_key = arr_info.key_param.unwrap_or(crate::prelude::TYPE_ARRAY_KEY);
    let arr_value = arr_info.value_param.unwrap_or(crate::prelude::TYPE_MIXED);
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

    if !crate::lattice::refines(crate::prelude::TYPE_INT, it_info.key_type, world, options, report) {
        return false;
    }
    if !list_info.flags.non_empty() {
        return true;
    }
    overlaps(it_info.value_type, list_info.element_type, world, options, report)
}

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
        && !crate::lattice::refines(crate::prelude::TYPE_INT, array_key_param, world, options, report)
    {
        return false;
    }
    let array_value = array_info.value_param.unwrap_or(crate::prelude::TYPE_MIXED);
    overlaps(list_info.element_type, array_value, world, options, report)
}

/// `array<K,V> ∩ array<K',V'>` mirrors `list_overlap`: the empty
/// array `[]` is shared only when neither side demands non-empty.
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
    // the overlap out — its value is not in `numeric`.
    if matches!(pair, (ElementKind::Numeric, ElementKind::String) | (ElementKind::String, ElementKind::Numeric)) {
        return numeric_string_overlap(a, b);
    }

    // True-union dominator cross-axis overlap. Each of `scalar`,
    // `numeric`, `array-key` is a disjoint-union of primitive
    // members; whenever two dominators share at least one member
    // family, runtime values inhabit both. Concretely:
    //   scalar = bool | int | float | string
    //   numeric = int | float | numeric-string ⊂ string
    //   array-key = int | string
    // so every pairwise combination shares at least `int`. Without
    // this rule subtract's `array-key \ numeric` couldn't fan out
    // (`!overlaps` short-circuited it to identity), and downstream
    // anti-monotonicity broke against the precise `int \ numeric`
    // narrowing on a sibling atom.
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

fn mixed_axes_compatible(info: MixedInfo, other: ElementId) -> bool {
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
