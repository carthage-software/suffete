//! Lattice meet (greatest lower bound): the type-returning intersection.
//!
//! Two entry points:
//!
//! - [`narrow`] is the primary operation. It runs the meet and
//!   classifies the result for assertion-driven narrowing: `Impossible`
//!   when the inputs are disjoint, `Redundant` when the input already
//!   refines the narrowing (the assertion adds no information),
//!   `Narrowed` when the result is strictly more specific.
//! - [`compute`] is a thin wrapper that throws away the classification
//!   and returns just the meet's `TypeId`. Use it when you want the
//!   intersection of two types unrelated to assertions (e.g. computing
//!   `A ∧ B` to feed into a later operation).
//!
//! In type-lattice terms, `compute(A, B)` is the greatest lower bound
//! (meet, ⊓) of `A` and `B` under the suffete subtype order, paired
//! with the union join in [`crate::join`].
//!
//! # Strategy
//!
//! Intersection distributes over union: for each
//! element on either side we compute pairwise atom meets, drop the
//! disjoint pairs, and union the surviving atoms.
//!
//! Atom-pair meet walks these rules in order:
//!
//! 1. Reflexivity / `never` / `mixed` / `placeholder`.
//! 2. Subsumption: if either side refines the other, the more specific
//!    one is the meet.
//! 3. Family-specific positive rules in [`family`] (integer ranges,
//!    string axes + numeric-string crossing, list / keyed-array shape
//!    composition, compositional object intersections).
//! 4. Otherwise the pair is treated as disjoint (`None`).
//!
//! # Soundness vs precision
//!
//! Returning [`prelude::TYPE_NEVER`] for a pair that actually overlaps is
//! a precision loss but never an unsoundness: `never <: anything` so the
//! lower-bound axiom holds. As family rules grow, what previously
//! collapsed to `never` becomes the real meet; every step is monotone
//! in result precision. The same precision debt feeds the classifier in
//! [`narrow`]: an unhandled overlap pair will be misreported as
//! `Impossible`, never as a false `Redundant`/`Narrowed`.

pub(crate) mod family;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::overlaps;
use crate::lattice::refines;
use crate::lattice::sealed::SealedResidual;
use crate::lattice::sealed::compute_residual;
use crate::meet::family::generic;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::prelude::TYPE_NEVER;
use crate::world::World;

/// Outcome of [`narrow`], classifying an assertion-driven meet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MeetOutcome {
    /// The input and the narrowing have no values in common. The
    /// assertion `input is σ` cannot hold for any value of `input`.
    Impossible,
    /// The input is already a subtype of the narrowing; the assertion
    /// adds no information. Carries the (unchanged) input.
    Redundant(TypeId),
    /// The narrowing strictly refined the input. Carries the new type.
    Narrowed(TypeId),
}

impl MeetOutcome {
    /// Extract the resulting [`TypeId`], mapping `Impossible` to
    /// [`prelude::TYPE_NEVER`].
    #[inline]
    #[must_use]
    pub const fn into_type(self) -> TypeId {
        match self {
            Self::Impossible => TYPE_NEVER,
            Self::Redundant(t) | Self::Narrowed(t) => t,
        }
    }
}

/// Compute `input ∧ narrowing` and classify the outcome for
/// assertion-driven diagnostics.
///
/// `input` is the existing type; `narrowing` is the type asserted at
/// the use site (e.g. the right-hand side of `instanceof`). Both
/// `result <: input` and `result <: narrowing` always hold for the
/// `Narrowed` and `Redundant` variants; `Impossible` corresponds to
/// `result ≡ ⊥`.
#[inline]
pub fn narrow<W: World>(
    input: TypeId,
    narrowing: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> MeetOutcome {
    if input == narrowing {
        return MeetOutcome::Redundant(input);
    }

    let input_type = input.as_ref();
    let narrowing_type = narrowing.as_ref();

    let mut atoms: Vec<ElementId> =
        Vec::with_capacity(input_type.elements.len().saturating_mul(narrowing_type.elements.len()));

    let any_negated = crate::element::simd::any_of_kind(input_type.elements, ElementKind::Negated)
        || crate::element::simd::any_of_kind(narrowing_type.elements, ElementKind::Negated);
    let any_mixed = crate::element::simd::any_of_kind(input_type.elements, ElementKind::Mixed)
        || crate::element::simd::any_of_kind(narrowing_type.elements, ElementKind::Mixed);

    for &x in input_type.elements {
        for &y in narrowing_type.elements {
            if any_negated && (x.kind() == ElementKind::Negated || y.kind() == ElementKind::Negated) {
                atoms.extend(negated_atom_meet_multi(x, y, world, options, report));
                continue;
            }

            if let Some(pieces) = cross_dominator_meet(x, y) {
                atoms.extend(pieces);
                continue;
            }

            if any_mixed && let Some(pieces) = narrowed_mixed_meet_multi(x, y, world) {
                atoms.extend(pieces);
                continue;
            }

            if let Some(m) = atom_meet(x, y, world, options, report) {
                atoms.push(m);
            }
        }
    }

    if atoms.is_empty() {
        return MeetOutcome::Impossible;
    }

    let result = TypeId::union(&atoms);
    if result == input { MeetOutcome::Redundant(input) } else { MeetOutcome::Narrowed(result) }
}

/// Compute `A ∧ B`: the largest type whose values are in both `A` and `B`.
///
/// Returns [`prelude::TYPE_NEVER`] when the two are disjoint (or when no
/// rule yet describes their overlap; precision can only grow).
///
/// This is a thin wrapper over [`narrow`] for callers that don't need
/// the assertion classification.
#[inline]
pub fn compute<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> TypeId {
    narrow(a, b, world, options, report).into_type()
}

#[inline]
fn atom_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    if overlaps::is_uninhabited(a, world) || overlaps::is_uninhabited(b, world) {
        return None;
    }

    if a == b {
        return Some(a);
    }

    if a == NEVER || b == NEVER {
        return None;
    }

    if a == MIXED || a == PLACEHOLDER {
        if overlaps::is_uninhabited(b, world) {
            return None;
        }

        return Some(b);
    }

    if b == MIXED || b == PLACEHOLDER {
        if overlaps::is_uninhabited(a, world) {
            return None;
        }

        return Some(a);
    }

    // `int <: float` is a one-directional PHP parameter-coercion rule,
    // not a value-set subtype relation: an `int(0)` runtime value is
    // not a member of `float`. Treat the pair as disjoint in meet so
    // the value-level intersection is never re-introduced via the
    // coercion-aware `refines` short-circuit below.
    if matches!((a.kind(), b.kind()), (ElementKind::Int, ElementKind::Float) | (ElementKind::Float, ElementKind::Int)) {
        return None;
    }

    // `meet(X, !T) ≡ subtract(X, T)`; `meet(!T, !U) ≡ !(T ∪ U)`.
    // `narrow` handles multi-atom results via
    // [`negated_atom_meet_multi`]; this single-atom path drops
    // conservatively when the result needs more than one atom.
    if a.kind() == ElementKind::Negated || b.kind() == ElementKind::Negated {
        return negated_atom_meet(a, b, world, options, report);
    }

    if a.kind() == ElementKind::Intersected || b.kind() == ElementKind::Intersected {
        return intersected_atom_meet(a, b, world, options, report);
    }

    if a.kind() == ElementKind::Mixed || b.kind() == ElementKind::Mixed {
        return normalise_meet_result(narrowed_mixed_meet(a, b, world), world);
    }

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);

    if refines(a_t, b_t, world, options, report) {
        return normalise_meet_result(Some(a), world);
    }

    if refines(b_t, a_t, world, options, report) {
        return normalise_meet_result(Some(b), world);
    }

    if a.kind() == ElementKind::GenericParameter || b.kind() == ElementKind::GenericParameter {
        return normalise_meet_result(generic::generic_parameter_meet(a, b, world, options, report), world);
    }

    normalise_meet_result(family_atom_meet(a, b, world, options, report), world)
}

/// If the synthesised element is uninhabited (e.g. sealed-class
/// intersection with all inheritors negated), collapse to `None`
/// so the caller treats it as the empty meet.
#[inline]
fn normalise_meet_result<W: World>(result: Option<ElementId>, world: &W) -> Option<ElementId> {
    match result {
        Some(elem) if overlaps::is_uninhabited(elem, world) => None,
        other => other,
    }
}

/// Cross-dominator pair meet: `(ArrayKey, Numeric)` shares `int`
/// and `numeric-string` but neither dominates, so subsumption
/// can't fire. `(Scalar, *)` already collapses via subsumption.
#[inline]
fn cross_dominator_meet(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    use crate::prelude::INT;
    use crate::prelude::NUMERIC_STRING;
    if matches!(
        (a.kind(), b.kind()),
        (ElementKind::ArrayKey, ElementKind::Numeric) | (ElementKind::Numeric, ElementKind::ArrayKey)
    ) {
        return Some(vec![INT, NUMERIC_STRING]);
    }
    None
}

/// Multi-atom variant of [`negated_atom_meet`] used by [`narrow`]:
/// returns every surviving atom (e.g. `meet(non-negative-int, !int(1))`
/// yields `[int(0), int<2,∞>]`).
#[inline]
fn negated_atom_meet_multi<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Vec<ElementId> {
    if a.kind() == ElementKind::Negated && b.kind() == ElementKind::Negated {
        return vec![negated_pair_meet(a, b, world, options, report)];
    }

    let i = interner();
    let (positive, negated_atom) = if a.kind() == ElementKind::Negated { (b, a) } else { (a, b) };
    let neg_info = *i.get_negated(negated_atom);
    let positive_t = i.intern_type(&[positive], FlowFlags::EMPTY);
    let surviving = crate::subtract::compute(positive_t, neg_info.inner, world, options, report);
    if surviving == crate::prelude::TYPE_NEVER {
        return Vec::new();
    }
    surviving.as_ref().elements.to_vec()
}

/// `meet(!T, !U) ≡ !(T ∪ U)`. When `T <: U` the union collapses
/// to `U` and the result is `!U`; symmetric for `U <: T`.
#[inline]
fn negated_pair_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> ElementId {
    let i = interner();
    let a_inner = i.get_negated(a).inner;
    let b_inner = i.get_negated(b).inner;
    if refines(a_inner, b_inner, world, options, report) {
        return b;
    }
    if refines(b_inner, a_inner, world, options, report) {
        return a;
    }
    let mut union_elems: Vec<ElementId> = a_inner.as_ref().elements.to_vec();
    union_elems.extend_from_slice(b_inner.as_ref().elements);
    let union_ty = i.intern_type(&union_elems, FlowFlags::EMPTY);
    ElementId::negated(union_ty)
}

/// `meet` with a `Negated` participant. `meet(X, !T)` ≡
/// `subtract(X, T)`; `meet(!T, !U)` ≡ `!(T ∪ U)`. Returning a single
/// `ElementId` constrains the surviving form: when `subtract`
/// produces multiple atoms (e.g. `int \ int(0) → negative-int |
/// positive-int`), we union them under a single negated atom only
/// when both operands were negated; otherwise we conservatively
/// drop to `None` and let the caller (via the loop in `narrow`)
/// fall back through other meet pairs. A future refactor of
/// `atom_meet` to return `Vec<ElementId>` would make this exact.
#[inline]
fn negated_atom_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    if a.kind() == ElementKind::Negated && b.kind() == ElementKind::Negated {
        return Some(negated_pair_meet(a, b, world, options, report));
    }

    let i = interner();
    let (positive, negated_atom) = if a.kind() == ElementKind::Negated { (b, a) } else { (a, b) };
    let neg_info = *i.get_negated(negated_atom);
    let positive_t = i.intern_type(&[positive], FlowFlags::EMPTY);
    let surviving = crate::subtract::compute(positive_t, neg_info.inner, world, options, report);
    let elements = surviving.as_ref().elements;
    if elements.is_empty() || surviving == crate::prelude::TYPE_NEVER {
        return None;
    }
    if elements.len() == 1 {
        return Some(elements[0]);
    }
    // Multi-atom subtract result can't fit a single-atom return.
    // Conservative drop; the surviving values still flow through
    // the wider `subtract` API for non-meet uses.
    None
}

#[inline]
fn intersected_atom_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();

    let result = if a.kind() == ElementKind::Intersected && b.kind() == ElementKind::Intersected {
        let a_info = *i.get_intersected(a);
        let b_info = *i.get_intersected(b);
        let head = atom_meet(a_info.head, b_info.head, world, options, report)?;
        let mut all_conjuncts: Vec<ElementId> = i.get_element_list(a_info.conjuncts).to_vec();
        all_conjuncts.extend_from_slice(i.get_element_list(b_info.conjuncts));
        ElementId::intersected(head, &all_conjuncts)
    } else {
        let (wrapped, other) = if a.kind() == ElementKind::Intersected { (a, b) } else { (b, a) };
        let info = *i.get_intersected(wrapped);
        let head = atom_meet(info.head, other, world, options, report)?;
        let conjuncts: Vec<ElementId> = i.get_element_list(info.conjuncts).to_vec();
        ElementId::intersected(head, &conjuncts)
    };

    if let Some(canon) = canonicalise_intersected(result, world, options, report) {
        return Some(canon);
    }

    if overlaps::is_uninhabited(result, world) {
        return None;
    }

    Some(result)
}

/// Drop redundant negated conjuncts and collapse sealed-cover residuals.
/// A negated conjunct `!X` is redundant when `head` is disjoint from `X`
/// (`!overlaps(head, X)`), meaning the head already satisfies the negation.
/// After dropping redundancies, a sealed-cover single-survivor residual
/// replaces the Intersected with the bare inheritor.
#[inline]
fn canonicalise_intersected<W: World>(
    elem: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    if elem.kind() != ElementKind::Intersected {
        return None;
    }
    let i = interner();
    let info = *i.get_intersected(elem);

    let conjuncts = i.get_element_list(info.conjuncts);
    let head_is_object = info.head.kind() == ElementKind::Object;
    let head_t = if head_is_object { Some(i.intern_type(&[info.head], FlowFlags::EMPTY)) } else { None };

    let mut kept: Vec<ElementId> = Vec::with_capacity(conjuncts.len());
    let mut negated_inners: Vec<TypeId> = Vec::with_capacity(conjuncts.len());
    for &c in conjuncts {
        if c.kind() == ElementKind::Negated {
            let inner = i.get_negated(c).inner;
            if let Some(t) = head_t
                && !overlaps(t, inner, world, options, report)
            {
                continue;
            }
            negated_inners.push(inner);
        }
        kept.push(c);
    }

    if kept.is_empty() {
        return Some(info.head);
    }

    if info.head.kind() == ElementKind::Object && !negated_inners.is_empty() {
        let residual = compute_residual(info.head, &negated_inners, world, options, report);
        match residual {
            SealedResidual::Surviving(survivors) if survivors.len() == 1 => {
                return Some(survivors[0]);
            }
            SealedResidual::FullyCovered => {
                return Some(crate::prelude::NEVER);
            }
            _ => {}
        }
    }

    if kept.len() == conjuncts.len() {
        return None;
    }

    Some(ElementId::intersected(info.head, &kept))
}

/// `meet(narrowed-mixed, X)` where `narrowed-mixed` is `truthy-mixed`,
/// `falsy-mixed`, or `non-null-mixed`. Returns `X` filtered by the
/// flag, expressed via the universal [`Intersected`] / [`Negated`]
/// machinery and PHP truthiness semantics for each element kind.
#[inline]
fn narrowed_mixed_meet<W: World>(a: ElementId, b: ElementId, world: &W) -> Option<ElementId> {
    let pieces = narrowed_mixed_meet_multi(a, b, world)?;
    match pieces.as_slice() {
        [] => None,
        [single] => Some(*single),
        _ => None,
    }
}

/// Multi-atom variant of [`narrowed_mixed_meet`]. Returns `None` when
/// neither side is a `Mixed` element. Returns `Some(vec![])` for the
/// empty meet.
#[inline]
fn narrowed_mixed_meet_multi<W: World>(a: ElementId, b: ElementId, world: &W) -> Option<Vec<ElementId>> {
    if a.kind() != ElementKind::Mixed && b.kind() != ElementKind::Mixed {
        return None;
    }
    let i = interner();
    if a.kind() == ElementKind::Mixed && b.kind() == ElementKind::Mixed {
        let a_info = *i.get_mixed(a);
        let b_info = *i.get_mixed(b);
        let merged_truthiness = match (a_info.truthiness(), b_info.truthiness()) {
            (Truthiness::Truthy, Truthiness::Falsy) | (Truthiness::Falsy, Truthiness::Truthy) => {
                return Some(Vec::new());
            }
            (Truthiness::Truthy, _) | (_, Truthiness::Truthy) => Truthiness::Truthy,
            (Truthiness::Falsy, _) | (_, Truthiness::Falsy) => Truthiness::Falsy,
            (Truthiness::Undetermined, Truthiness::Undetermined) => Truthiness::Undetermined,
        };

        let merged = MixedInfo::EMPTY
            .with_is_non_null(a_info.is_non_null() || b_info.is_non_null())
            .with_is_empty(a_info.is_empty() || b_info.is_empty())
            .with_is_isset_from_loop(a_info.is_isset_from_loop() || b_info.is_isset_from_loop())
            .with_truthiness(merged_truthiness);

        return Some(vec![i.intern_mixed(merged)]);
    }

    let (mixed_atom, other) = if a.kind() == ElementKind::Mixed { (a, b) } else { (b, a) };
    let info = *i.get_mixed(mixed_atom);
    if info == MixedInfo::EMPTY {
        if overlaps::is_uninhabited(other, world) {
            return Some(Vec::new());
        }

        return Some(vec![other]);
    }

    if info.is_non_null() && other == crate::prelude::NULL {
        return Some(Vec::new());
    }

    let truthy_pieces = narrow_by_truthiness(other, info.truthiness());
    let with_non_null: Vec<ElementId> = if info.is_non_null() {
        let null_t = i.intern_type(&[crate::prelude::NULL], FlowFlags::EMPTY);
        let neg_null = ElementId::negated(null_t);
        truthy_pieces.into_iter().map(|p| ElementId::intersected(p, &[neg_null])).collect()
    } else {
        truthy_pieces
    };

    Some(with_non_null)
}

/// Narrow `other` by PHP truthiness. `Vec::new()` when the kind is
/// incompatible with the requested truthiness (e.g. `Object` is always
/// truthy, so falsy narrowing yields the empty set). `vec![other]`
/// when truthiness is undetermined.
#[inline]
fn narrow_by_truthiness(other: ElementId, truthiness: crate::element::payload::Truthiness) -> Vec<ElementId> {
    use crate::element::payload::Truthiness;
    if matches!(truthiness, Truthiness::Undetermined) {
        return vec![other];
    }

    let i = interner();
    match (other.kind(), truthiness) {
        (ElementKind::Null | ElementKind::False, Truthiness::Truthy) => Vec::new(),
        (ElementKind::True, Truthiness::Falsy) => Vec::new(),
        (
            ElementKind::Object
            | ElementKind::ObjectAny
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
            | ElementKind::Resource
            | ElementKind::Callable
            | ElementKind::ClassLikeString,
            Truthiness::Falsy,
        ) => Vec::new(),
        (ElementKind::Bool, Truthiness::Truthy) => vec![crate::prelude::TRUE],
        (ElementKind::Bool, Truthiness::Falsy) => vec![crate::prelude::FALSE],
        (ElementKind::Int, Truthiness::Truthy) => {
            let zero_t = i.intern_type(&[crate::prelude::INT_ZERO], FlowFlags::EMPTY);
            vec![ElementId::intersected(other, &[ElementId::negated(zero_t)])]
        }
        (ElementKind::Int, Truthiness::Falsy) => vec![crate::prelude::INT_ZERO],
        (ElementKind::Float, Truthiness::Truthy) => {
            let zero = ElementId::float_literal(0.0);
            let zero_t = i.intern_type(&[zero], FlowFlags::EMPTY);
            vec![ElementId::intersected(other, &[ElementId::negated(zero_t)])]
        }
        (ElementKind::Float, Truthiness::Falsy) => vec![ElementId::float_literal(0.0)],
        (ElementKind::String, Truthiness::Truthy) => vec![narrow_string_truthy(other)],
        (ElementKind::String, Truthiness::Falsy) => narrow_string_falsy(other),
        (ElementKind::List | ElementKind::Array | ElementKind::Iterable, Truthiness::Truthy) => {
            vec![force_non_empty(other)]
        }
        (ElementKind::List | ElementKind::Array, Truthiness::Falsy) => match falsy_collection(other) {
            Some(empty) => vec![empty],
            None => Vec::new(),
        },
        (ElementKind::Iterable, Truthiness::Falsy) => Vec::new(),
        _ => match (crate::lattice::family::mixed::truthiness_of(other), truthiness) {
            (Truthiness::Truthy, Truthiness::Falsy) | (Truthiness::Falsy, Truthiness::Truthy) => Vec::new(),
            _ => vec![other],
        },
    }
}

/// `falsy ∩ list/array<X>` is the empty collection singleton when the
/// input allows empty (`non_empty=false` and not sealed-non-empty),
/// otherwise the empty set (non-empty collections are all truthy).
#[inline]
fn falsy_collection(elem: ElementId) -> Option<ElementId> {
    let i = interner();
    match elem.kind() {
        ElementKind::List => {
            let info = *i.get_list(elem);
            if info.flags.non_empty() {
                return None;
            }

            Some(i.intern_list(crate::element::payload::ListInfo {
                element_type: crate::prelude::TYPE_NEVER,
                known_elements: None,
                known_count: None,
                flags: crate::element::payload::ListFlags::default(),
            }))
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            if info.flags.non_empty() {
                return None;
            }

            Some(crate::prelude::EMPTY_ARRAY)
        }
        _ => None,
    }
}

#[inline]
fn narrow_string_truthy(elem: ElementId) -> ElementId {
    use crate::element::payload::scalar::StringLiteral;
    let i = interner();
    let info = *i.get_string(elem);
    if let StringLiteral::Value(v) = info.literal {
        let s = v.as_str();
        if s.is_empty() || s == "0" {
            return crate::prelude::NEVER;
        }

        return elem;
    }

    let flags = info.flags.with_is_truthy(true).with_is_non_empty(true);
    i.intern_string(crate::element::payload::scalar::StringInfo { flags, ..info })
}

#[inline]
fn narrow_string_falsy(elem: ElementId) -> Vec<ElementId> {
    use crate::element::payload::scalar::StringLiteral;
    let i = interner();
    let info = *i.get_string(elem);
    if let StringLiteral::Value(v) = info.literal {
        let s = v.as_str();
        return if s.is_empty() || s == "0" { vec![elem] } else { Vec::new() };
    }

    if info.flags.is_non_empty() && !v_zero_compatible(info) {
        return Vec::new();
    }

    if info.flags.is_truthy() {
        return Vec::new();
    }

    let mut pieces: Vec<ElementId> = Vec::new();
    if !info.flags.is_non_empty() {
        pieces.push(ElementId::string_literal(""));
    }

    if v_zero_compatible(info) {
        pieces.push(ElementId::string_literal("0"));
    }

    pieces.retain(|&p| {
        let p_info = *i.get_string(p);
        let StringLiteral::Value(pv) = p_info.literal else { return true };
        let s = pv.as_str();
        casing_compatible(info.casing, s) && (!info.flags.is_numeric() || s.parse::<i64>().is_ok())
    });

    pieces
}

#[inline]
fn v_zero_compatible(info: crate::element::payload::scalar::StringInfo) -> bool {
    if info.flags.is_truthy() {
        return false;
    }
    casing_compatible(info.casing, "0")
}

#[inline]
fn casing_compatible(casing: crate::element::payload::scalar::StringCasing, s: &str) -> bool {
    use crate::element::payload::scalar::StringCasing;
    let has_lower = s.chars().any(|c| c.is_ascii_lowercase());
    let has_upper = s.chars().any(|c| c.is_ascii_uppercase());
    match casing {
        StringCasing::Unspecified => true,
        StringCasing::Lowercase => !has_upper,
        StringCasing::Uppercase => !has_lower,
    }
}

#[inline]
fn force_non_empty(elem: ElementId) -> ElementId {
    let i = interner();
    match elem.kind() {
        ElementKind::List => {
            let info = *i.get_list(elem);
            i.intern_list(crate::element::payload::ListInfo { flags: info.flags.with_non_empty(true), ..info })
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            i.intern_array(crate::element::payload::KeyedArrayInfo { flags: info.flags.with_non_empty(true), ..info })
        }
        _ => elem,
    }
}

#[inline]
fn family_atom_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    match (a.kind(), b.kind()) {
        (ElementKind::Int, ElementKind::Int) => family::int::int_meet(a, b),
        (ElementKind::String, ElementKind::String) => family::string::string_meet(a, b),
        (ElementKind::Numeric, ElementKind::String) | (ElementKind::String, ElementKind::Numeric) => {
            family::string::numeric_string_meet(a, b)
        }
        (ElementKind::List, ElementKind::List) => family::array::list_meet(a, b, world, options, report),
        (ElementKind::Array, ElementKind::Array) => family::array::keyed_array_meet(a, b, world, options, report),
        (ElementKind::List, ElementKind::Array) | (ElementKind::Array, ElementKind::List) => {
            family::array::list_array_meet(a, b, world, options, report)
        }
        (ElementKind::Iterable, ElementKind::Iterable) => family::iterable::iterable_meet(a, b, world, options, report),
        (ElementKind::Callable, ElementKind::Callable) => family::callable::callable_meet(a, b, world, options, report),
        (ElementKind::HasMethod, ElementKind::HasMethod) => family::has_member::has_method_meet(a, b),
        (ElementKind::HasProperty, ElementKind::HasProperty) => family::has_member::has_property_meet(a, b),
        (ElementKind::HasMethod, ElementKind::HasProperty) | (ElementKind::HasProperty, ElementKind::HasMethod) => {
            family::has_member::has_method_property_meet(a, b)
        }
        (ElementKind::Object, ElementKind::Object) => {
            family::object::compose_object_intersection(a, b, world, options, report)
        }
        (ElementKind::Object, ElementKind::HasMethod)
        | (ElementKind::Object, ElementKind::HasProperty)
        | (ElementKind::Object, ElementKind::ObjectShape) => {
            family::object::compose_object_with_structural(a, b, world)
        }
        (ElementKind::HasMethod, ElementKind::Object)
        | (ElementKind::HasProperty, ElementKind::Object)
        | (ElementKind::ObjectShape, ElementKind::Object) => {
            family::object::compose_object_with_structural(b, a, world)
        }
        (ElementKind::ObjectShape, ElementKind::HasMethod | ElementKind::HasProperty) => {
            family::object::compose_shape_with_structural(a, b)
        }
        (ElementKind::HasMethod | ElementKind::HasProperty, ElementKind::ObjectShape) => {
            family::object::compose_shape_with_structural(b, a)
        }
        (ElementKind::Iterable, ElementKind::Array) => family::array::iterable_array_meet(a, b, world, options, report),
        (ElementKind::Array, ElementKind::Iterable) => family::array::iterable_array_meet(b, a, world, options, report),
        (ElementKind::Iterable, ElementKind::List) => family::array::iterable_list_meet(a, b, world, options, report),
        (ElementKind::List, ElementKind::Iterable) => family::array::iterable_list_meet(b, a, world, options, report),

        _ => None,
    }
}
