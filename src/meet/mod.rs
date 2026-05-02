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
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines;
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

    let mut atoms: Vec<ElementId> = Vec::new();
    for &x in input_type.elements {
        for &y in narrowing_type.elements {
            if x.kind() == ElementKind::Negated || y.kind() == ElementKind::Negated {
                atoms.extend(negated_atom_meet_multi(x, y, world, options, report));
                continue;
            }
            if let Some(pieces) = cross_dominator_meet(x, y) {
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
    if crate::lattice::overlaps::is_uninhabited(a, world) || crate::lattice::overlaps::is_uninhabited(b, world) {
        return None;
    }
    if a == b {
        return Some(a);
    }
    if a == NEVER || b == NEVER {
        return None;
    }
    if a == MIXED || a == PLACEHOLDER {
        return Some(b);
    }
    if b == MIXED || b == PLACEHOLDER {
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

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    if refines(a_t, b_t, world, options, report) {
        return if crate::lattice::overlaps::is_uninhabited(a, world) { None } else { Some(a) };
    }
    if refines(b_t, a_t, world, options, report) {
        return if crate::lattice::overlaps::is_uninhabited(b, world) { None } else { Some(b) };
    }

    if a.kind() == ElementKind::GenericParameter || b.kind() == ElementKind::GenericParameter {
        return family::generic::generic_parameter_meet(a, b, world, options, report);
    }

    family_atom_meet(a, b, world, options, report)
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
