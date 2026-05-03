//! Refinement (subtype) relation: `refines(a, b)` is `true` iff every value
//! of type `a` is also a value of type `b` (i.e. `a <: b`).

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::StringCasing;
use crate::element::payload::StringLiteral;
use crate::element::payload::StringRefinementFlags;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::CoercionCauses;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family;
use crate::prelude::FALSE;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::NULL;
use crate::world::World;

/// `true` iff `a <: b`: every runtime value of type `a` is also a value of
/// type `b` (i.e. `a` is a refinement / narrowing of `b`).
///
/// Implements the universal axioms (refl / Bot / Top), the union
/// dispatch (Union-L / Union-R), and the structural scalar lattice
/// (bool / int / float / string / class-like-string / resource /
/// array-key / numeric / scalar / object-any). Object hierarchy
/// queries flow through `world`; callable variance, array shape
/// rules, mixed-axis refinements, and template machinery layer in
/// family by family; what isn't implemented returns `false`
/// conservatively.
#[inline]
pub fn refines<W: World>(
    input: TypeId,
    container: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input == container && !options.ignore_null && !options.ignore_false {
        return true;
    }

    let a = expand_double_negation(input);
    let b = expand_double_negation(container);

    let a_type = a.as_ref();
    let b_type = b.as_ref();

    // Union-L / Union-R: every element of `a` (modulo any caller-requested
    // skips for `null` / `false`) must fit some element of `b`.
    a_type
        .elements
        .iter()
        .filter(|elem| {
            let skipped = (options.ignore_null && **elem == NULL) || (options.ignore_false && **elem == FALSE);
            !skipped
        })
        .all(|elem| {
            if b_type.elements.iter().any(|rhs| element_refines(*elem, *rhs, world, options, report)) {
                return true;
            }

            // Fan-out: a single int element may be covered by the union of
            // several int elements on the rhs (e.g. `int<-∞,0> <: lit(0) |
            // int<-∞,-1>`). Element-by-element refines can't see that, so
            // try the family-level coverage check before giving up.
            if int_union_covers(*elem, b_type.elements) {
                return true;
            }

            if string_union_covers(*elem, b_type.elements) {
                return true;
            }

            if bool_union_covers(*elem, b_type.elements) {
                return true;
            }

            if mixed_union_covers(*elem, b_type.elements) {
                return true;
            }

            if list_union_covers(*elem, b_type.elements, world, options, report) {
                return true;
            }

            if array_union_covers(*elem, b_type.elements, world, options, report) {
                return true;
            }

            if intersected_partition_covers(*elem, b_type.elements, world, options, report) {
                return true;
            }

            if negation_partition_covers(*elem, b_type.elements, world, options, report) {
                return true;
            }

            generic_parameter_union_covers(*elem, b_type.elements, world, options, report)
        })
}

/// `T <: X ∪ ¬X` always (the union is `mixed`). Fires when the
/// container has a `Negated(Y)` atom whose inner `Y` is covered by
/// some other container atom, making the union `mixed`.
#[inline]
fn negation_partition_covers<W: World>(
    _input: ElementId,
    containers: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    for &c in containers {
        if c.kind() != ElementKind::Negated {
            continue;
        }
        let inner = i.get_negated(c).inner;
        let others: Vec<ElementId> = containers.iter().copied().filter(|&e| e != c).collect();
        if others.is_empty() {
            continue;
        }
        let others_t = i.intern_type(&others, FlowFlags::EMPTY);
        if refines(inner, others_t, world, options, report) {
            return true;
        }
    }
    false
}

/// Recognize the partition identity
/// `Intersected(H, [!X1, .., !Xn]) ∪ X1 ∪ .. ∪ Xn ≡ H ∪ X1 ∪ .. ∪ Xn`.
/// When a container atom is `Intersected(H, [!X1, ..])` with all
/// negated inners `Xi` present elsewhere in the container, the
/// container's value-set equals the container with the Intersected
/// replaced by its head. Recurse on that reduced container ; the
/// number of Intersected atoms strictly decreases each step.
#[inline]
fn intersected_partition_covers<W: World>(
    input: ElementId,
    containers: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    for (idx, &c) in containers.iter().enumerate() {
        if c.kind() != ElementKind::Intersected {
            continue;
        }
        let info = *i.get_intersected(c);
        let conjuncts = i.get_element_list(info.conjuncts);
        let mut all_negated = true;
        let mut all_inners_present = true;
        for &cj in conjuncts {
            if cj.kind() != ElementKind::Negated {
                all_negated = false;
                break;
            }
            for &ie in i.get_negated(cj).inner.as_ref().elements {
                if !containers.iter().enumerate().any(|(j, &other)| j != idx && other == ie) {
                    all_inners_present = false;
                    break;
                }
            }
            if !all_inners_present {
                break;
            }
        }
        if !all_negated || !all_inners_present {
            continue;
        }
        let mut reduced: Vec<ElementId> = containers.to_vec();
        reduced[idx] = info.head;
        let input_t = i.intern_type(&[input], FlowFlags::EMPTY);
        let reduced_t = i.intern_type(&reduced, FlowFlags::EMPTY);
        if refines(input_t, reduced_t, world, options, report) {
            return true;
        }
    }
    false
}

/// True iff a `array<K, V>` input is covered by the union of all
/// `Array` elements in `containers`. Mirrors [`list_union_covers`]:
/// the empty-array singleton needs coverage by some non-empty=false
/// container, and the (key, value) parameters must refine the
/// pointwise union of the containers' parameters.
#[inline]
fn array_union_covers<W: World>(
    input: ElementId,
    containers: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input.kind() != ElementKind::Array {
        return false;
    }
    let i = interner();
    let input_info = *i.get_array(input);
    if input_info.known_items.is_some() {
        return false;
    }
    let (Some(input_key), Some(input_value)) = (input_info.key_param, input_info.value_param) else {
        return false;
    };
    let mut key_types: Vec<TypeId> = Vec::new();
    let mut value_types: Vec<TypeId> = Vec::new();
    let mut covers_empty = false;
    for &c in containers {
        let head = if c.kind() == ElementKind::Array {
            c
        } else if c.kind() == ElementKind::Intersected && i.get_intersected(c).head.kind() == ElementKind::Array {
            i.get_intersected(c).head
        } else {
            continue;
        };
        let c_info = *i.get_array(head);
        if c_info.known_items.is_some() {
            continue;
        }
        let (Some(k), Some(v)) = (c_info.key_param, c_info.value_param) else {
            continue;
        };
        if !c_info.flags.non_empty() {
            covers_empty = true;
        }
        key_types.push(k);
        value_types.push(v);
    }
    if key_types.is_empty() {
        return false;
    }
    if !input_info.flags.non_empty() && !covers_empty {
        return false;
    }
    let mut key_union: Vec<ElementId> = Vec::new();
    for t in &key_types {
        key_union.extend_from_slice(t.as_ref().elements);
    }
    let key_t = i.intern_type(&key_union, FlowFlags::EMPTY);
    let mut value_union: Vec<ElementId> = Vec::new();
    for t in &value_types {
        value_union.extend_from_slice(t.as_ref().elements);
    }
    let value_t = i.intern_type(&value_union, FlowFlags::EMPTY);
    refines(input_key, key_t, world, options, report) && refines(input_value, value_t, world, options, report)
}

/// True iff a `list<E>` input is covered by the union of all `List`
/// elements in `containers`. Empty-list coverage requires some
/// container with `non_empty=false`; non-empty coverage requires
/// `E` to refine the union of all container element types.
#[inline]
fn list_union_covers<W: World>(
    input: ElementId,
    containers: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input.kind() != ElementKind::List {
        return false;
    }
    let i = interner();
    let input_info = *i.get_list(input);
    if input_info.known_elements.is_some() {
        return false;
    }
    let mut element_types: Vec<TypeId> = Vec::new();
    let mut covers_empty = false;
    for &c in containers {
        let head = if c.kind() == ElementKind::List {
            c
        } else if c.kind() == ElementKind::Intersected && i.get_intersected(c).head.kind() == ElementKind::List {
            i.get_intersected(c).head
        } else {
            continue;
        };
        let c_info = *i.get_list(head);
        if c_info.known_elements.is_some() {
            continue;
        }
        if !c_info.flags.non_empty() {
            covers_empty = true;
        }
        element_types.push(c_info.element_type);
    }
    if element_types.is_empty() {
        return false;
    }
    if !input_info.flags.non_empty() && !covers_empty {
        return false;
    }
    let mut union_elems: Vec<ElementId> = Vec::new();
    for t in &element_types {
        union_elems.extend_from_slice(t.as_ref().elements);
    }
    let union_ty = i.intern_type(&union_elems, FlowFlags::EMPTY);
    refines(input_info.element_type, union_ty, world, options, report)
}

/// Expand `!!X` element shapes inside a union to `X`'s elements.
/// Single-atom `!!T` collapses at intern; multi-atom `T = a|b`
/// survives as `Negated(Negated(T))` and gets flattened here so
/// the structural dispatch sees the inner atoms.
#[inline]
fn expand_double_negation(t: TypeId) -> TypeId {
    let elements = t.as_ref().elements;
    if !elements.iter().any(|&e| is_double_negation(e)) {
        return t;
    }
    let i = interner();
    let mut expanded: Vec<ElementId> = Vec::with_capacity(elements.len());
    for &elem in elements {
        if is_double_negation(elem) {
            let inner_neg = i.get_negated(elem);
            let inner_elem = inner_neg.inner.as_ref().elements[0];
            let inner_inner = i.get_negated(inner_elem).inner;
            expanded.extend_from_slice(inner_inner.as_ref().elements);
        } else {
            expanded.push(elem);
        }
    }
    i.intern_type(&expanded, FlowFlags::EMPTY)
}

#[inline]
fn is_double_negation(elem: ElementId) -> bool {
    if elem.kind() != ElementKind::Negated {
        return false;
    }
    let i = interner();
    let inner = i.get_negated(elem).inner.as_ref().elements;
    inner.len() == 1 && inner[0].kind() == ElementKind::Negated
}

/// True iff a single generic-parameter input `T extends X` is covered
/// by the union of all same-`T` elements on the rhs. Each rhs element
/// contributes its constraint; if their union covers `X`, the input
/// is in the rhs (just split across same-template narrowings).
#[inline]
fn generic_parameter_union_covers<W: World>(
    input: ElementId,
    containers: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input.kind() != ElementKind::GenericParameter {
        return false;
    }

    let i = interner();
    let input_info = i.get_generic_parameter(input);
    let mut rhs_constraints: Vec<ElementId> = Vec::new();
    for &c in containers {
        if c.kind() != ElementKind::GenericParameter {
            continue;
        }

        let c_info = i.get_generic_parameter(c);
        if c_info.name != input_info.name || c_info.defining_entity != input_info.defining_entity {
            continue;
        }

        rhs_constraints.extend_from_slice(c_info.constraint.as_ref().elements);
    }

    if rhs_constraints.is_empty() {
        return false;
    }

    let combined = i.intern_type(&rhs_constraints, FlowFlags::EMPTY);
    refines(input_info.constraint, combined, world, options, report)
}

/// True iff the int range / literal `input` is fully covered by the union
/// of all int elements in `containers`. Used as a precision fallback when
/// no single container element accepts the input. The `UnspecifiedLiteral`
/// dominator is excluded because the lattice keeps it as a distinct axis
/// (`int <: literal-int` is intentionally false). The broad `Unspecified`
/// `int` input falls back here when the disjuncts collectively cover
/// the full integer range; required for partition-style properties
/// like `meet(a,b) ∪ subtract(a,b) ⊇ a`.
#[inline]
fn int_union_covers(input: ElementId, containers: &[ElementId]) -> bool {
    if input.kind() != ElementKind::Int {
        return false;
    }

    let i = interner();
    let input_info = *i.get_int(input);
    if matches!(input_info, IntInfo::UnspecifiedLiteral) {
        return false;
    }

    let (in_lo, in_hi) = int_bounds_of(input);

    let mut ranges: Vec<(Option<i64>, Option<i64>)> = containers
        .iter()
        .filter(|c| {
            // Skip `UnspecifiedLiteral` containers: at the value level they
            // span all ints, but the lattice keeps them distinct (refines
            // doesn't accept `int <: literal-int`). Treating them as
            // unbounded coverage would silently break that distinction.
            c.kind() == ElementKind::Int && !matches!(*interner().get_int(**c), IntInfo::UnspecifiedLiteral)
        })
        .map(|c| int_bounds_of(*c))
        .collect();

    if ranges.is_empty() {
        return false;
    }

    ranges.sort_by(|a, b| match (a.0, b.0) {
        (None, None) => core::cmp::Ordering::Equal,
        (None, _) => core::cmp::Ordering::Less,
        (_, None) => core::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(&y),
    });

    let mut covered_up_to: Option<i64> = None;
    let mut started = false;

    for (lo, hi) in ranges {
        if !started {
            let starts_input = match (lo, in_lo) {
                (None, _) => true,
                (Some(_), None) => false,
                (Some(l), Some(s)) => l <= s,
            };

            if !starts_input {
                continue;
            }

            covered_up_to = match (in_lo, hi) {
                (Some(s), Some(h)) if h < s => continue,
                _ => hi,
            };

            started = true;
        } else {
            let connects = match (lo, covered_up_to) {
                (None, _) => true,
                (_, None) => true,
                (Some(l), Some(c)) => l <= c.saturating_add(1),
            };

            if !connects {
                return false;
            }

            covered_up_to = match (covered_up_to, hi) {
                (None, _) | (_, None) => None,
                (Some(c), Some(h)) => Some(c.max(h)),
            };
        }

        let covers_top = match (in_hi, covered_up_to) {
            (_, None) => true,
            (None, Some(_)) => false,
            (Some(t), Some(c)) => t <= c,
        };

        if covers_top {
            return true;
        }
    }

    false
}

/// True iff a broad `string` input is covered by the union of refined
/// string elements in `containers`. Sufficient condition: rhs contains
/// some atom that covers all non-empty strings AND some atom that
/// covers the empty string. Together that is the empty/non-empty
/// partition of `string`. Refined inputs (already non-empty,
/// truthy, etc.) bail; element-wise refines is exact for them.
#[inline]
fn string_union_covers(input: ElementId, containers: &[ElementId]) -> bool {
    if input.kind() != ElementKind::String {
        return false;
    }

    let i = interner();
    let info = *i.get_string(input);
    let is_broad_string = matches!(info.literal, StringLiteral::None)
        && info.flags == StringRefinementFlags::EMPTY
        && matches!(info.casing, StringCasing::Unspecified);
    if !is_broad_string {
        return false;
    }

    let mut covers_empty = false;
    let mut covers_non_empty = false;
    for &c in containers {
        if c.kind() != ElementKind::String {
            continue;
        }

        let c_info = *i.get_string(c);
        if matches!(c_info.literal, StringLiteral::Value(v) if v.as_str().is_empty()) {
            covers_empty = true;
        }

        // A broad non-empty-string atom: literal None/Unspecified, the
        // is_non_empty flag set, no casing/numeric/callable/truthy
        // refinements. `truthy-string` excludes the literal `"0"`, so
        // it does NOT cover all non-empty strings; treating it as
        // such here would falsely make `string \ (truthy | empty)`
        // collapse to `never`.
        if matches!(c_info.literal, StringLiteral::None | StringLiteral::Unspecified)
            && c_info.flags.is_non_empty()
            && !c_info.flags.is_truthy()
            && !c_info.flags.is_numeric()
            && !c_info.flags.is_callable()
            && matches!(c_info.casing, StringCasing::Unspecified)
        {
            covers_non_empty = true;
        }
    }

    covers_empty && covers_non_empty
}

/// True iff broad `bool` is covered by the union of `true` and `false`
/// in `containers`. Mirrors `int_union_covers` for the bool axis.
#[inline]
fn bool_union_covers(input: ElementId, containers: &[ElementId]) -> bool {
    if input.kind() != ElementKind::Bool {
        return false;
    }
    let has_true = crate::element::simd::any_of_kind(containers, ElementKind::True);
    let has_false = crate::element::simd::any_of_kind(containers, ElementKind::False);
    has_true && has_false
}

/// True iff broad `mixed` is covered by `nonnull-mixed | null` in
/// `containers`. The null/non-null axis is the only structural
/// partition of `mixed` the lattice can recognize directly; deeper
/// coverage (e.g. `int | string | … = mixed`) needs an exhaustive
/// case-analysis we don't try here.
#[inline]
fn mixed_union_covers(input: ElementId, containers: &[ElementId]) -> bool {
    use crate::element::payload::MixedInfo;
    if input.kind() != ElementKind::Mixed {
        return false;
    }
    let i = interner();
    let info = *i.get_mixed(input);
    if info != MixedInfo::EMPTY {
        return false;
    }
    let has_null = crate::element::simd::contains(containers, NULL);
    // Mixed-kind containers are rare in practice ; the SIMD prefilter
    // picks them out cheaply, and only matched lanes pay the
    // `is_non_null` payload check.
    let has_nonnull = crate::element::simd::any_of_kind(containers, ElementKind::Mixed)
        && containers.iter().any(|c| c.kind() == ElementKind::Mixed && i.get_mixed(*c).is_non_null());
    has_null && has_nonnull
}

#[inline]
fn int_bounds_of(elem: ElementId) -> (Option<i64>, Option<i64>) {
    match *interner().get_int(elem) {
        IntInfo::Unspecified | IntInfo::UnspecifiedLiteral => (None, None),
        IntInfo::Literal(n) => (Some(n), Some(n)),
        IntInfo::Range(rid) => {
            let r = *interner().get_int_range(rid);
            (r.lower(), r.upper())
        }
    }
}

/// `true` iff `a :> b`: every value of type `b` is also a value of type `a`
/// (`a` generalizes `b`). Equivalent to `refines(b, a, world, options, report)`.
#[inline]
pub fn generalizes<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    refines(b, a, world, options, report)
}

/// Decide whether one element refines another, ignoring flow flags.
///
/// Universal axioms first (refl, `never <: anything`, `anything <: mixed`),
/// then dispatch on the container's kind into a family-specific helper.
/// When the result is `false` and the input belongs to a "true-union" kind
/// (`mixed`, `array_key`, `bool`, `object_any`, `scalar`, `numeric`), the
/// [`CoercionCauses::TRUE_UNION_NARROW`] cause is recorded to flag that the
/// rejection was a narrowing, not an out-of-family mismatch. `mixed` inputs
/// additionally record [`CoercionCauses::NESTED_MIXED`]. `object_any`
/// inputs additionally record [`CoercionCauses::OBJECT_ANY_DOWN`].
#[inline]
pub(crate) fn element_refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input == container {
        return true;
    }

    if input == NEVER {
        return true;
    }

    if crate::lattice::overlaps::is_uninhabited(input, world) {
        return true;
    }

    // Don't short-circuit on `is_uninhabited(container)`: an open-world
    // `Foo & Bar` can pick up a common subclass via interfaces / traits,
    // and the container-intersection rule below handles it via per-conjunct
    // refinement. `atom_minus` uses the symmetric check because subtract
    // has the inverse soundness needs.
    if (input == crate::prelude::VOID && container == NULL) || (input == NULL && container == crate::prelude::VOID) {
        return true;
    }

    if container == MIXED {
        return true;
    }

    if input.kind() == ElementKind::GenericParameter && container.kind() != ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(input).constraint;
        let container_type = interner().intern_type(&[container], FlowFlags::EMPTY);
        let result = refines(constraint, container_type, world, options, report);
        if !result && container != MIXED && constraint.as_ref().elements.contains(&MIXED) {
            report.causes.remove(CoercionCauses::NESTED_MIXED);
            report.add_cause(CoercionCauses::TRUE_UNION_NARROW);
            report.add_cause(CoercionCauses::FROM_AS_MIXED);
        }

        return result;
    }

    if container.kind() == ElementKind::Intersected {
        let info = *interner().get_intersected(container);
        if let Some(reconstructed) = crate::element::reconstruct_with_intersections(info.head, info.conjuncts) {
            return element_refines(input, reconstructed, world, options, report);
        }
        return refines_container_intersected(input, container, world, options, report);
    }
    if input.kind() == ElementKind::Intersected {
        let info = *interner().get_intersected(input);
        if let Some(reconstructed) = crate::element::reconstruct_with_intersections(info.head, info.conjuncts) {
            return element_refines(reconstructed, container, world, options, report);
        }
        return refines_input_intersected(input, container, world, options, report);
    }

    let result = dispatch_refines(input, container, world, options, report);

    #[allow(clippy::else_if_without_else)]
    if result {
        if input.kind() == ElementKind::Int && container.kind() == ElementKind::Float {
            report.add_cause(CoercionCauses::PHP_RUNTIME_COERCE);
        }
    } else if is_true_union_kind(input.kind()) {
        report.add_cause(CoercionCauses::TRUE_UNION_NARROW);
        match input.kind() {
            ElementKind::Mixed => report.add_cause(CoercionCauses::NESTED_MIXED),
            ElementKind::ObjectAny => report.add_cause(CoercionCauses::OBJECT_ANY_DOWN),
            _ => {}
        }
    }

    result
}

#[inline]
fn refines_container_intersected<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let info = *interner().get_intersected(container);
    if !element_refines(input, info.head, world, options, report) {
        return false;
    }
    for &conjunct in interner().get_element_list(info.conjuncts) {
        if !element_refines(input, conjunct, world, options, report) {
            return false;
        }
    }
    true
}

#[inline]
fn refines_input_intersected<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let info = *interner().get_intersected(input);
    if element_refines(info.head, container, world, options, report) {
        return true;
    }
    for &conjunct in interner().get_element_list(info.conjuncts) {
        if element_refines(conjunct, container, world, options, report) {
            return true;
        }
    }
    false
}

/// `true` for kinds whose values inhabit multiple disjoint sub-families:
/// narrowing one of these to a concrete sub-form is the standard PHP
/// "type-coerced" pattern that the lattice records via
/// [`CoercionCauses::TRUE_UNION_NARROW`].
#[inline]
const fn is_true_union_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Mixed
            | ElementKind::ArrayKey
            | ElementKind::Bool
            | ElementKind::ObjectAny
            | ElementKind::Scalar
            | ElementKind::Numeric
    )
}

#[inline]
fn dispatch_refines<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if input.kind() == ElementKind::Negated {
        return family::negated::refines_input_negated(input, container, world, options, report);
    }
    if container.kind() == ElementKind::Negated {
        return family::negated::refines_container_negated(input, container, world, options, report);
    }
    match container.kind() {
        ElementKind::Bool => family::bool::refines(input, container),
        ElementKind::Resource => family::resource::refines(input, container),
        ElementKind::Int => family::int::refines(input, container),
        ElementKind::Float => family::float::refines(input, container),
        ElementKind::String => family::string::refines(input, container),
        ElementKind::ClassLikeString => family::class_like_string::refines(input, container, world, options, report),
        ElementKind::ArrayKey => family::array_key::refines(input, container),
        ElementKind::Numeric => family::numeric::refines(input, container),
        ElementKind::Scalar => family::scalar::refines(input, container),
        ElementKind::ObjectAny => family::object::refines_object_any(input, container),
        ElementKind::Object
        | ElementKind::Enum
        | ElementKind::ObjectShape
        | ElementKind::HasMethod
        | ElementKind::HasProperty => family::object::refines(input, container, world, options, report),
        ElementKind::Array | ElementKind::List => family::array::refines(input, container, world, options, report),
        ElementKind::Iterable => family::iterable::refines(input, container, world, options, report),
        ElementKind::Callable => family::callable::refines(input, container, world, options, report),
        ElementKind::Mixed => family::mixed::refines(input, container),
        ElementKind::GenericParameter => family::generic::refines(input, container, world, options, report),
        ElementKind::Variable
        | ElementKind::Reference
        | ElementKind::MemberReference
        | ElementKind::GlobalReference
        | ElementKind::Alias
        | ElementKind::Conditional
        | ElementKind::Derived => family::reference::refines(input, container),
        ElementKind::Null
        | ElementKind::Never
        | ElementKind::Void
        | ElementKind::Placeholder
        | ElementKind::True
        | ElementKind::False
        | ElementKind::Negated
        // `Intersected` containers are stripped + re-routed by the
        // `dispatch_intersected_*` paths above this match ; if we
        // reach this arm with one, the structural check has already
        // ruled the input out.
        | ElementKind::Intersected => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ElementId;
    use crate::FlowFlags;
    use crate::interner::interner;
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::BOOL;
    use crate::prelude::CALLABLE_STRING;
    use crate::prelude::CLASS_STRING;
    use crate::prelude::CLOSED_RESOURCE;
    use crate::prelude::ENUM_STRING;
    use crate::prelude::FALSE;
    use crate::prelude::FLOAT;
    use crate::prelude::INT;
    use crate::prelude::INTERFACE_STRING;
    use crate::prelude::LITERAL_FLOAT;
    use crate::prelude::LITERAL_INT;
    use crate::prelude::LITERAL_STRING;
    use crate::prelude::LOWERCASE_STRING;
    use crate::prelude::NEGATIVE_INT;
    use crate::prelude::NON_EMPTY_STRING;
    use crate::prelude::NULL;
    use crate::prelude::NUMERIC;
    use crate::prelude::NUMERIC_STRING;
    use crate::prelude::OPEN_RESOURCE;
    use crate::prelude::POSITIVE_INT;
    use crate::prelude::RESOURCE;
    use crate::prelude::SCALAR;
    use crate::prelude::STRING;
    use crate::prelude::TRUE;
    use crate::prelude::TRUTHY_STRING;
    use crate::prelude::TYPE_ARRAY_KEY;
    use crate::prelude::TYPE_BOOL;
    use crate::prelude::TYPE_FLOAT;
    use crate::prelude::TYPE_INT;
    use crate::prelude::TYPE_INT_OR_FLOAT;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;
    use crate::prelude::TYPE_NEVER;
    use crate::prelude::TYPE_NULL;
    use crate::prelude::TYPE_NUMERIC;
    use crate::prelude::TYPE_SCALAR;
    use crate::prelude::TYPE_STRING;
    use crate::prelude::UPPERCASE_STRING;
    use crate::world::NullWorld;

    #[inline]
    fn check(input: TypeId, container: TypeId) -> bool {
        let mut report = LatticeReport::new();
        refines(input, container, &NullWorld, LatticeOptions::default(), &mut report)
    }

    #[inline]
    fn check_elem(input: ElementId, container: ElementId) -> bool {
        let i = interner();
        let it = i.intern_type(&[input], FlowFlags::EMPTY);
        let ct = i.intern_type(&[container], FlowFlags::EMPTY);
        check(it, ct)
    }

    #[test]
    #[inline]
    fn reflexivity_holds_for_well_known_types() {
        assert!(check(TYPE_INT, TYPE_INT));
        assert!(check(TYPE_NULL, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_NEVER));
        assert!(check(TYPE_MIXED, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_INT_OR_STRING));
    }

    #[test]
    #[inline]
    fn bot_axiom_never_refines_anything() {
        assert!(check(TYPE_NEVER, TYPE_INT));
        assert!(check(TYPE_NEVER, TYPE_NULL));
        assert!(check(TYPE_NEVER, TYPE_MIXED));
        assert!(check(TYPE_NEVER, TYPE_INT_OR_STRING));
    }

    #[test]
    #[inline]
    fn top_axiom_anything_refines_vanilla_mixed() {
        assert!(check(TYPE_INT, TYPE_MIXED));
        assert!(check(TYPE_NULL, TYPE_MIXED));
        assert!(check(TYPE_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
    }

    #[test]
    #[inline]
    fn bool_family_refines_bool() {
        assert!(check_elem(TRUE, BOOL));
        assert!(check_elem(FALSE, BOOL));
        assert!(!check_elem(TRUE, FALSE));
        assert!(!check_elem(FALSE, TRUE));
        assert!(!check_elem(BOOL, TRUE));
        assert!(!check_elem(BOOL, FALSE));
    }

    #[test]
    #[inline]
    fn resource_family_refines_resource() {
        assert!(check_elem(OPEN_RESOURCE, RESOURCE));
        assert!(check_elem(CLOSED_RESOURCE, RESOURCE));
        assert!(!check_elem(OPEN_RESOURCE, CLOSED_RESOURCE));
        assert!(!check_elem(CLOSED_RESOURCE, OPEN_RESOURCE));
        assert!(!check_elem(RESOURCE, OPEN_RESOURCE));
    }

    #[test]
    #[inline]
    fn int_dominator_absorbs_subforms() {
        assert!(check_elem(POSITIVE_INT, INT));
        assert!(check_elem(NEGATIVE_INT, INT));
        assert!(check_elem(LITERAL_INT, INT));
        assert!(check_elem(ElementId::int_literal(42), INT));
        assert!(check_elem(ElementId::int_literal(-1), INT));
    }

    #[test]
    #[inline]
    fn int_literal_in_range() {
        let r = ElementId::int_range(Some(0), Some(10));
        assert!(check_elem(ElementId::int_literal(0), r));
        assert!(check_elem(ElementId::int_literal(5), r));
        assert!(check_elem(ElementId::int_literal(10), r));
        assert!(!check_elem(ElementId::int_literal(-1), r));
        assert!(!check_elem(ElementId::int_literal(11), r));
    }

    #[test]
    #[inline]
    fn int_range_in_range() {
        let outer = ElementId::int_range(Some(0), Some(100));
        let inner = ElementId::int_range(Some(10), Some(20));
        assert!(check_elem(inner, outer));
        assert!(!check_elem(outer, inner));
    }

    #[test]
    #[inline]
    fn int_open_range_subsumes_closed() {
        let from_zero = ElementId::int_range(Some(0), None);
        let bounded = ElementId::int_range(Some(5), Some(10));
        assert!(check_elem(bounded, from_zero));
        assert!(!check_elem(from_zero, bounded));
    }

    #[test]
    #[inline]
    fn int_unspec_literal_accepts_concrete_literals() {
        assert!(check_elem(ElementId::int_literal(42), LITERAL_INT));
        assert!(check_elem(ElementId::int_literal(-1), LITERAL_INT));
        assert!(check_elem(LITERAL_INT, LITERAL_INT));
        assert!(!check_elem(INT, LITERAL_INT));
    }

    #[test]
    #[inline]
    fn float_dominator_absorbs_subforms() {
        assert!(check_elem(LITERAL_FLOAT, FLOAT));
        assert!(check_elem(ElementId::float_literal(1.5), FLOAT));
        assert!(check_elem(ElementId::float_literal(-2.5), FLOAT));
    }

    #[test]
    #[inline]
    fn float_unspec_literal_accepts_concrete_literals() {
        assert!(check_elem(ElementId::float_literal(1.5), LITERAL_FLOAT));
        assert!(check_elem(LITERAL_FLOAT, LITERAL_FLOAT));
        assert!(!check_elem(FLOAT, LITERAL_FLOAT));
    }

    #[test]
    #[inline]
    fn string_dominator_absorbs_subforms() {
        assert!(check_elem(NON_EMPTY_STRING, STRING));
        assert!(check_elem(NUMERIC_STRING, STRING));
        assert!(check_elem(LOWERCASE_STRING, STRING));
        assert!(check_elem(UPPERCASE_STRING, STRING));
        assert!(check_elem(TRUTHY_STRING, STRING));
        assert!(check_elem(LITERAL_STRING, STRING));
        assert!(check_elem(ElementId::string_literal("hi"), STRING));
        assert!(check_elem(ElementId::string_literal(""), STRING));
    }

    #[test]
    #[inline]
    fn string_literal_satisfies_non_empty() {
        assert!(check_elem(ElementId::string_literal("hi"), NON_EMPTY_STRING));
        assert!(check_elem(ElementId::string_literal("0"), NON_EMPTY_STRING));
        assert!(!check_elem(ElementId::string_literal(""), NON_EMPTY_STRING));
    }

    #[test]
    #[inline]
    fn string_literal_satisfies_truthy() {
        assert!(check_elem(ElementId::string_literal("hi"), TRUTHY_STRING));
        assert!(check_elem(ElementId::string_literal("1"), TRUTHY_STRING));
        assert!(!check_elem(ElementId::string_literal(""), TRUTHY_STRING));
        assert!(!check_elem(ElementId::string_literal("0"), TRUTHY_STRING));
    }

    #[test]
    #[inline]
    fn string_literal_satisfies_numeric() {
        assert!(check_elem(ElementId::string_literal("123"), NUMERIC_STRING));
        assert!(check_elem(ElementId::string_literal("-1"), NUMERIC_STRING));
        assert!(check_elem(ElementId::string_literal("1.5"), NUMERIC_STRING));
        assert!(!check_elem(ElementId::string_literal("hi"), NUMERIC_STRING));
        assert!(!check_elem(ElementId::string_literal(""), NUMERIC_STRING));
    }

    #[test]
    #[inline]
    fn string_literal_satisfies_casing() {
        assert!(check_elem(ElementId::string_literal("hello"), LOWERCASE_STRING));
        assert!(!check_elem(ElementId::string_literal("Hello"), LOWERCASE_STRING));
        assert!(check_elem(ElementId::string_literal("HELLO"), UPPERCASE_STRING));
        assert!(!check_elem(ElementId::string_literal("Hello"), UPPERCASE_STRING));
        // Case-neutral characters satisfy both.
        assert!(check_elem(ElementId::string_literal("123"), LOWERCASE_STRING));
        assert!(check_elem(ElementId::string_literal("123"), UPPERCASE_STRING));
    }

    #[test]
    #[inline]
    fn truthy_string_refines_non_empty_string() {
        assert!(check_elem(TRUTHY_STRING, NON_EMPTY_STRING));
    }

    #[test]
    #[inline]
    fn callable_string_does_not_refine_numeric_or_lowercase_by_default() {
        assert!(check_elem(CALLABLE_STRING, STRING));
        assert!(!check_elem(CALLABLE_STRING, NUMERIC_STRING));
        assert!(!check_elem(CALLABLE_STRING, LOWERCASE_STRING));
    }

    #[test]
    #[inline]
    fn class_like_string_refines_string() {
        assert!(check_elem(CLASS_STRING, STRING));
        assert!(check_elem(INTERFACE_STRING, STRING));
        assert!(check_elem(ENUM_STRING, STRING));
    }

    #[test]
    #[inline]
    fn distinct_class_like_kinds_are_not_subtypes() {
        assert!(!check_elem(CLASS_STRING, INTERFACE_STRING));
        assert!(!check_elem(INTERFACE_STRING, CLASS_STRING));
        assert!(!check_elem(CLASS_STRING, ENUM_STRING));
    }

    #[test]
    #[inline]
    fn array_key_absorbs_int_string_class_string() {
        assert!(check_elem(INT, ARRAY_KEY));
        assert!(check_elem(STRING, ARRAY_KEY));
        assert!(check_elem(CLASS_STRING, ARRAY_KEY));
        assert!(check_elem(ElementId::int_literal(42), ARRAY_KEY));
        assert!(check_elem(ElementId::string_literal("k"), ARRAY_KEY));
    }

    #[test]
    #[inline]
    fn array_key_does_not_absorb_float_or_bool() {
        assert!(!check_elem(FLOAT, ARRAY_KEY));
        assert!(!check_elem(BOOL, ARRAY_KEY));
        assert!(!check_elem(NULL, ARRAY_KEY));
    }

    #[test]
    #[inline]
    fn numeric_absorbs_int_float_and_numeric_string() {
        assert!(check_elem(INT, NUMERIC));
        assert!(check_elem(FLOAT, NUMERIC));
        assert!(check_elem(NUMERIC_STRING, NUMERIC));
        assert!(check_elem(ElementId::int_literal(5), NUMERIC));
        assert!(check_elem(ElementId::float_literal(1.5), NUMERIC));
        assert!(check_elem(ElementId::string_literal("123"), NUMERIC));
    }

    #[test]
    #[inline]
    fn numeric_does_not_absorb_general_string() {
        assert!(!check_elem(STRING, NUMERIC));
        assert!(!check_elem(ElementId::string_literal("hi"), NUMERIC));
        assert!(!check_elem(BOOL, NUMERIC));
    }

    #[test]
    #[inline]
    fn scalar_absorbs_all_scalar_families() {
        assert!(check_elem(INT, SCALAR));
        assert!(check_elem(FLOAT, SCALAR));
        assert!(check_elem(STRING, SCALAR));
        assert!(check_elem(BOOL, SCALAR));
        assert!(check_elem(TRUE, SCALAR));
        assert!(check_elem(FALSE, SCALAR));
        assert!(check_elem(ARRAY_KEY, SCALAR));
        assert!(check_elem(NUMERIC, SCALAR));
        assert!(check_elem(CLASS_STRING, SCALAR));
        assert!(check_elem(ElementId::int_literal(0), SCALAR));
    }

    #[test]
    #[inline]
    fn scalar_does_not_absorb_null_or_resource() {
        assert!(!check_elem(NULL, SCALAR));
        assert!(!check_elem(RESOURCE, SCALAR));
    }

    #[test]
    #[inline]
    fn union_left_every_input_element_must_fit_some_container_element() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_MIXED));
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_INT_OR_FLOAT));
        assert!(!check(TYPE_INT_OR_STRING, TYPE_INT));
    }

    #[test]
    #[inline]
    fn union_right_singleton_input_fits_member_of_union() {
        assert!(check(TYPE_INT, TYPE_INT_OR_STRING));
        assert!(check(TYPE_STRING, TYPE_INT_OR_STRING));
        assert!(!check(TYPE_FLOAT, TYPE_INT_OR_STRING));
    }

    #[test]
    #[inline]
    fn unrelated_types_do_not_refine() {
        assert!(!check(TYPE_INT, TYPE_STRING));
        assert!(!check(TYPE_FLOAT, TYPE_STRING));
        assert!(!check(TYPE_NULL, TYPE_INT));
    }

    #[test]
    #[inline]
    fn int_does_not_refine_float() {
        // `int` and `float` are distinct value sets at the runtime
        // type level. PHP's implicit int→float coercion at parameter
        // binding is a callsite convenience, not a subtype relation,
        // and is intentionally not modeled by `refines`.
        assert!(!check(TYPE_INT, TYPE_FLOAT));
    }

    #[test]
    #[inline]
    fn fresh_int_literal_refines_int() {
        let lit = ElementId::int_literal(42);
        let lit_type = interner().intern_type(&[lit], FlowFlags::EMPTY);
        assert!(check(lit_type, TYPE_INT));
        assert!(check(lit_type, TYPE_MIXED));
    }

    #[test]
    #[inline]
    fn input_int_or_float_fits_int_or_float_via_union_dispatch() {
        let int_or_float = interner().intern_type(&[INT, FLOAT], FlowFlags::EMPTY);
        assert!(check(int_or_float, TYPE_INT_OR_FLOAT));

        let with_string = interner().intern_type(&[INT, FLOAT, STRING], FlowFlags::EMPTY);
        assert!(!check(with_string, TYPE_INT_OR_FLOAT));
    }

    #[test]
    #[inline]
    fn int_or_string_refines_array_key() {
        assert!(check(TYPE_INT_OR_STRING, TYPE_ARRAY_KEY));
    }

    #[test]
    #[inline]
    fn int_or_float_refines_numeric() {
        assert!(check(TYPE_INT_OR_FLOAT, TYPE_NUMERIC));
    }

    #[test]
    #[inline]
    fn int_or_string_or_float_or_bool_refines_scalar() {
        let id = interner().intern_type(&[INT, STRING, FLOAT, BOOL], FlowFlags::EMPTY);
        assert!(check(id, TYPE_SCALAR));
    }

    #[test]
    #[inline]
    fn nullable_int_refines_nullable_array_key() {
        let nullable_int = interner().intern_type(&[NULL, INT], FlowFlags::EMPTY);
        let nullable_ak = interner().intern_type(&[NULL, ARRAY_KEY], FlowFlags::EMPTY);
        assert!(check(nullable_int, nullable_ak));
    }

    #[test]
    #[inline]
    fn type_bool_refines_scalar() {
        assert!(check(TYPE_BOOL, TYPE_SCALAR));
    }

    #[test]
    #[inline]
    fn generalizes_is_inverse_of_refines() {
        let mut r = LatticeReport::new();
        assert!(generalizes(TYPE_MIXED, TYPE_INT, &NullWorld, LatticeOptions::default(), &mut r));
        assert!(generalizes(TYPE_INT, TYPE_NEVER, &NullWorld, LatticeOptions::default(), &mut r));
        assert!(!generalizes(TYPE_INT, TYPE_FLOAT, &NullWorld, LatticeOptions::default(), &mut r));
    }
}
