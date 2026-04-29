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
//! disjoint pairs, and union the surviving atoms.
//!
//! Atom-pair meet (intersection.md §2.2) walks these rules in order:
//!
//! 1. Reflexivity / `never` / `mixed` / `placeholder`.
//! 2. Subsumption — if either side refines the other, the more specific
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
//! collapsed to `never` becomes the real meet — every step is monotone
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
    if crate::lattice::overlaps::is_uninhabited(a, world)
        || crate::lattice::overlaps::is_uninhabited(b, world)
    {
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

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    if refines(a_t, b_t, world, options, report) {
        return if object_intersection_is_uninhabited(a, world) { None } else { Some(a) };
    }
    if refines(b_t, a_t, world, options, report) {
        return if object_intersection_is_uninhabited(b, world) { None } else { Some(b) };
    }

    if a.kind() == ElementKind::GenericParameter || b.kind() == ElementKind::GenericParameter {
        return family::generic::generic_parameter_meet(a, b, world, options, report);
    }

    family_atom_meet(a, b, world, options, report)
}

/// `Foo & Bar` with unrelated nominal classes (neither descends the
/// other) is uninhabited under PHP's single-inheritance class graph,
/// even when the lattice can construct it. Used as a guard so
/// subsumption doesn't pass an uninhabited intersection back as the
/// meet result.
fn object_intersection_is_uninhabited<W: World>(elem: ElementId, world: &W) -> bool {
    if elem.kind() != ElementKind::Object {
        return false;
    }
    let i = interner();
    let info = *i.get_object(elem);
    let Some(intersections_id) = info.intersections else { return false };
    let mut classes: Vec<mago_atom::Atom> = vec![info.name];
    for &conjunct in i.get_element_list(intersections_id) {
        if conjunct.kind() == ElementKind::Object {
            classes.push(i.get_object(conjunct).name);
        }
    }
    for (idx, &left) in classes.iter().enumerate() {
        for &right in &classes[idx + 1..] {
            if left == right {
                continue;
            }
            if !world.descends_from(left, right) && !world.descends_from(right, left) {
                return true;
            }
        }
    }
    false
}

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
        (ElementKind::List, ElementKind::List) => family::array::list_meet(a, b),
        (ElementKind::Array, ElementKind::Array) => family::array::keyed_array_meet(a, b),
        (ElementKind::List, ElementKind::Array) | (ElementKind::Array, ElementKind::List) => {
            family::array::list_array_meet(a, b)
        }
        (ElementKind::Iterable, ElementKind::Iterable) => family::iterable::iterable_meet(a, b),
        (ElementKind::Callable, ElementKind::Callable) => family::callable::callable_meet(a, b),
        (ElementKind::HasMethod, ElementKind::HasMethod) => family::has_member::has_method_meet(a, b),
        (ElementKind::HasProperty, ElementKind::HasProperty) => family::has_member::has_property_meet(a, b),
        (ElementKind::Object, ElementKind::Object) => {
            family::object::compose_object_intersection(a, b, world, options, report)
        }
        _ => None,
    }
}
