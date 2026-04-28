//! Lattice meet (greatest lower bound) — the type-returning intersection.
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
//! Intersection distributes over union (intersection.md §2.1): for each
//! element on either side we compute pairwise atom meets, drop the
//! disjoint pairs, and union the surviving atoms via [`crate::join`].
//!
//! Atom-pair meet (intersection.md §2.2) walks these rules in order:
//!
//! 1. Reflexivity / `never` / `mixed` / `placeholder`.
//! 2. Subsumption — if either side refines the other, the more specific
//!    one is the meet.
//! 3. Family-specific positive rules (e.g. integer range intersection,
//!    compositional object intersections).
//! 4. Otherwise the pair is treated as disjoint (`None`).
//!
//! # Soundness vs precision
//!
//! Returning [`prelude::TYPE_NEVER`] for a pair that actually overlaps is
//! a precision loss but never an unsoundness: `never <: anything` so the
//! lower-bound axiom holds. As family rules grow, what previously
//! collapsed to `never` becomes the real meet — every step is monotone
//! in result precision. The same precision debt feeds the classifier in
//! [`narrow`]: an unhandled overlap pair will be misreported as
//! `Impossible`, never as a false `Redundant`/`Narrowed`.

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::IntInfo;
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
    pub fn into_type(self) -> TypeId {
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
    for &x in input_type.elements.iter() {
        for &y in narrowing_type.elements.iter() {
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

/// Compute `A ∧ B`: the largest type whose values are in both `A` and
/// `B`. Returns [`prelude::TYPE_NEVER`] when the two are disjoint (or
/// when no rule yet describes their overlap; precision can only grow).
///
/// This is a thin wrapper over [`narrow`] for callers that don't need
/// the assertion classification.
pub fn compute<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> TypeId {
    narrow(a, b, world, options, report).into_type()
}

fn atom_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
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

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    if refines(a_t, b_t, world, options, report) {
        return Some(a);
    }

    if refines(b_t, a_t, world, options, report) {
        return Some(b);
    }

    family_atom_meet(a, b)
}

fn family_atom_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return int_meet(a, b);
    }

    if a.kind() == ElementKind::Object && b.kind() == ElementKind::Object {
        return Some(compose_object_intersection(a, b));
    }

    None
}

/// Intersect two `Int` atoms. Subsumption (e.g. `INT ∧ Range(0,10)`) is
/// already handled by the caller; this only fires when neither side
/// refines the other, which means both are bounded ranges or distinct
/// literals. The result is `Range(max(lo), min(hi))` — collapsed to a
/// `Literal` when the bounds coincide, or `None` when the interval is
/// empty.
fn int_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
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
        (Some(l), Some(h)) if l > h => None,
        (Some(l), Some(h)) if l == h => Some(ElementId::int_literal(l)),
        _ => Some(ElementId::int_range(lo, hi)),
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

/// Compositional object intersection (intersection.md §2.3.2): when
/// neither named object refines the other, the meet is one side as the
/// head with the other side (and any pre-existing intersection
/// conjuncts on either side) gathered into the `intersections` list.
/// `a`'s head, type arguments, and flags are kept; `b`'s head is added
/// as a conjunct.
///
/// `final` classes (which would force `Foo & Bar → never` when
/// unrelated) are not yet exposed by [`World`], so this function never
/// short-circuits to disjoint. Adding that query is a follow-up.
fn compose_object_intersection(a: ElementId, b: ElementId) -> ElementId {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    let mut conjuncts: Vec<ElementId> = Vec::new();
    if let Some(id) = a_info.intersections {
        conjuncts.extend_from_slice(i.get_element_list(id));
    }
    let b_head = i.intern_object(ObjectInfo { intersections: None, ..b_info });
    conjuncts.push(b_head);
    if let Some(id) = b_info.intersections {
        conjuncts.extend_from_slice(i.get_element_list(id));
    }

    conjuncts.sort();
    conjuncts.dedup();

    i.intern_object(ObjectInfo { intersections: Some(i.intern_element_list(&conjuncts)), ..a_info })
}
