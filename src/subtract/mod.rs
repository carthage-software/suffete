//! Lattice difference: `A \ B` is the type whose values are in `A` but not in `B`.
//!
//! Pairs with [`crate::meet`] the way negative narrowing pairs with
//! positive narrowing: `if ($x !== null)` produces `subtract(T_x, null)`.
//!
//! Two entry points:
//!
//! - [`narrow`] is the primary operation. It runs the difference and
//!   classifies the result for assertion-driven narrowing:
//!   `Impossible` when `input ⊆ σ` (the negation can never hold),
//!   `Redundant` when `input # σ` (the negation is trivially true and
//!   adds nothing), `Narrowed` when the result is strictly smaller.
//! - [`compute`] is a thin wrapper that returns just the resulting
//!   `TypeId`, mapping `Impossible` to [`prelude::TYPE_NEVER`].
//!
//! The operation is *partial*: when no rule
//! describes the precise difference, the input is returned unchanged.
//! Returning a superset of the true difference is sound; the
//! soundness invariants in. 1 are
//!
//! - `result <: A` (no value escapes the original),
//! - `result ∧ B ≡ ⊥` *if precise*, `result ⊇ A \ B` always.
//!
//! # Strategy
//!
//! Difference distributes over union on the left and intersects with the
//! complement on the right:
//!
//! ```text
//! (α ∨ β) \ γ  ≡  (α \ γ) ∨ (β \ γ)
//! α \ (β ∨ γ)  ≡  (α \ β) \ γ  ≡  (α \ γ) \ β
//! ```
//!
//! So for each atom in `A` we fold over the atoms in `B`, subtracting
//! one at a time and accumulating the surviving pieces.
//!
//! Atom-pair difference walks these rules in order:
//!
//! 1. `α <: β` ⇒ `⊥` (every `α`-value is a `β`-value).
//! 2. `α # β` (disjoint) ⇒ `α` (subtraction is identity).
//! 3. Family-specific positive rule (e.g. integer-range split).
//! 4. Otherwise return `α` unchanged (conservative fallback).

mod family;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::ListInfo;
use crate::interner::interner;
use crate::lattice;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::overlaps;
use crate::lattice::refines;
use crate::meet;
use crate::prelude;
use crate::prelude::FALSE;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::NON_NULL_MIXED;
use crate::prelude::NULL;
use crate::prelude::TRUE;
use crate::prelude::TYPE_NEVER;
use crate::subtract::family::array;
use crate::subtract::family::callable;
use crate::subtract::family::dominator;
use crate::subtract::family::generic;
use crate::subtract::family::has_member;
use crate::subtract::family::int;
use crate::subtract::family::iterable;
use crate::subtract::family::list;
use crate::subtract::family::object;
use crate::subtract::family::string;
use crate::world::World;

/// Outcome of [`narrow`], classifying an assertion-driven difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SubtractOutcome {
    /// `input ⊆ σ`: every value of the input also satisfies the
    /// predicate being negated, so the negative assertion can never
    /// hold. The result is `never`.
    Impossible,
    /// `input # σ` (already disjoint): the input has no values in
    /// common with the predicate, so the negation is trivially true
    /// and adds no information. Carries the (unchanged) input.
    Redundant(TypeId),
    /// The subtraction strictly narrowed the input. Carries the new
    /// type.
    Narrowed(TypeId),
}

impl SubtractOutcome {
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

/// Compute `input \ narrowing` and classify the outcome for
/// assertion-driven diagnostics.
///
/// `input` is the existing type; `narrowing` is the type the negative
/// assertion is removing (e.g. the right-hand side of
/// `!($x instanceof Foo)`).
///
/// `result <: input` always; `result ∧ narrowing ≡ ⊥` when the family
/// rules cover every surviving atom precisely.
#[inline]
pub fn narrow<W: World>(
    input: TypeId,
    narrowing: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> SubtractOutcome {
    if input == narrowing {
        return SubtractOutcome::Impossible;
    }

    let input_type = input.as_ref();
    let narrowing_type = narrowing.as_ref();

    let mut atoms: Vec<ElementId> = Vec::with_capacity(input_type.elements.len());
    let bs_type = if narrowing.flags() == FlowFlags::EMPTY {
        narrowing
    } else {
        interner().intern_type(narrowing_type.elements, FlowFlags::EMPTY)
    };

    let mut current_scratch: Vec<ElementId> = Vec::new();
    let mut next_scratch: Vec<ElementId> = Vec::new();
    for &x in input_type.elements {
        let pieces = subtract_all(
            x,
            narrowing_type.elements,
            bs_type,
            world,
            options,
            report,
            &mut current_scratch,
            &mut next_scratch,
        );

        atoms.extend(pieces.iter().copied());
    }

    if atoms.is_empty() {
        return SubtractOutcome::Impossible;
    }

    let result = TypeId::union(&atoms);
    if result == input { SubtractOutcome::Redundant(input) } else { SubtractOutcome::Narrowed(result) }
}

/// Compute `A \ B`: the largest representable type whose values are in
/// `A` but not in `B`. Thin wrapper over [`narrow`] for callers that
/// don't need the assertion classification.
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

/// Apply `α \ β₁ \ β₂ \ … \ βₙ` by folding over the right-hand atoms,
/// then drain to empty when the surviving atoms refine the full
/// `bs` union (the per-atom fold sees one container at a time and
/// can stall on partition-style coverage).
///
/// `bs_type` is the pre-interned `TypeId` for `bs` (caller-hoisted to
/// amortize across input atoms). `current_scratch` and `next_scratch`
/// are reused per call ; the function clears and refills them, returning
/// a borrowed view into `current_scratch` for the caller to copy out.
#[inline]
#[allow(clippy::too_many_arguments)]
fn subtract_all<'scratch, W: World>(
    x: ElementId,
    bs: &[ElementId],
    bs_type: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
    current_scratch: &'scratch mut Vec<ElementId>,
    next_scratch: &mut Vec<ElementId>,
) -> &'scratch [ElementId] {
    current_scratch.clear();
    current_scratch.push(x);
    for &b in bs {
        if current_scratch.is_empty() {
            break;
        }

        next_scratch.clear();
        for &c in current_scratch.iter() {
            next_scratch.extend(atom_minus(c, b, world, options, report));
        }

        core::mem::swap(current_scratch, next_scratch);
    }

    if !current_scratch.is_empty() {
        let i = interner();
        let current_t = i.intern_type(current_scratch, FlowFlags::EMPTY);
        if refines(current_t, bs_type, world, options, report) {
            current_scratch.clear();
        }
    }

    current_scratch.as_slice()
}

pub(in crate::subtract) fn atom_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Vec<ElementId> {
    if a == b || a == NEVER {
        return Vec::new();
    }

    if b == NEVER {
        return vec![a];
    }

    if b == MIXED {
        return Vec::new();
    }

    if lattice::overlaps::is_uninhabited(b, world) {
        return vec![a];
    }

    if lattice::overlaps::is_uninhabited(a, world) {
        return Vec::new();
    }

    if a.kind() == ElementKind::Intersected {
        let i = interner();
        let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
        let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
        if refines(a_t, b_t, world, options, report) {
            return Vec::new();
        }

        if !overlaps(a_t, b_t, world, options, report) {
            return vec![a];
        }

        let info = *i.get_intersected(a);
        let head_pieces = atom_minus(info.head, b, world, options, report);
        let conjuncts: Vec<ElementId> = i.get_element_list(info.conjuncts).to_vec();

        return head_pieces.into_iter().map(|h| ElementId::intersected(h, &conjuncts)).collect();
    }

    // `subtract(X, !T)` ≡ `meet(X, T)` and `subtract(!T, X)` ≡
    // `!(T ∪ X)`. Routing here preserves the duality with meet.
    if b.kind() == ElementKind::Negated {
        let neg_info = *interner().get_negated(b);
        let kept =
            meet::compute(interner().intern_type(&[a], FlowFlags::EMPTY), neg_info.inner, world, options, report);

        return kept.as_ref().elements.to_vec();
    }

    if a.kind() == ElementKind::Negated {
        let neg_info = *interner().get_negated(a);
        let mut union_elems: Vec<ElementId> = neg_info.inner.as_ref().elements.to_vec();
        union_elems.push(b);
        let union_ty = interner().intern_type(&union_elems, FlowFlags::EMPTY);
        return vec![ElementId::negated(union_ty)];
    }

    if a == MIXED {
        let b_ty = interner().intern_type(&[b], FlowFlags::EMPTY);
        return vec![ElementId::negated(b_ty)];
    }

    if a == NON_NULL_MIXED {
        let union_ty = interner().intern_type(&[NULL, b], FlowFlags::EMPTY);
        return vec![ElementId::negated(union_ty)];
    }

    if a.kind() == ElementKind::ClassLikeString && b.kind() == ElementKind::ClassLikeString && a != b {
        let b_ty = interner().intern_type(&[b], FlowFlags::EMPTY);
        return vec![ElementId::intersected(a, &[ElementId::negated(b_ty)])];
    }

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);

    if refines(a_t, b_t, world, options, report) {
        return Vec::new();
    }

    if !overlaps(a_t, b_t, world, options, report) {
        return vec![a];
    }

    if a.kind() == ElementKind::GenericParameter {
        return generic::generic_parameter_minus(a, b, world, options, report).unwrap_or_else(|| vec![a]);
    }

    if let Some(pieces) = dominator::true_union_minus(a, b, world, options, report) {
        return pieces;
    }

    if let Some(pieces) = object::object_descendant_minus(a, b, world) {
        return pieces;
    }

    if let Some(pieces) = family_atom_minus(a, b) {
        return pieces;
    }

    if b.kind() == ElementKind::Intersected {
        return vec![a];
    }

    if scalar_supports_intersected_subtract(a.kind()) {
        return vec![ElementId::intersected(a, &[ElementId::negated(b_t)])];
    }

    vec![a]
}

#[inline]
const fn scalar_supports_intersected_subtract(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::String
            | ElementKind::Int
            | ElementKind::Float
            | ElementKind::Bool
            | ElementKind::Numeric
            | ElementKind::ArrayKey
            | ElementKind::ClassLikeString
            | ElementKind::Resource
            | ElementKind::Scalar
            | ElementKind::Enum
    )
}

#[inline]
fn family_atom_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return Some(int::int_minus(a, b));
    }

    if a == prelude::BOOL && b == TRUE {
        return Some(vec![FALSE]);
    }

    if a == prelude::BOOL && b == FALSE {
        return Some(vec![TRUE]);
    }

    if a.kind() == ElementKind::String && b.kind() == ElementKind::String {
        return string::string_minus(a, b);
    }

    if a.kind() == ElementKind::List && b.kind() == ElementKind::List {
        return list::list_minus(a, b);
    }

    if a.kind() == ElementKind::Array && b.kind() == ElementKind::Array {
        return array::array_minus(a, b);
    }

    if a.kind() == ElementKind::List && b.kind() == ElementKind::Iterable {
        return list::list_minus_iterable(a, b);
    }

    if a.kind() == ElementKind::Array && b.kind() == ElementKind::Iterable {
        return array::array_minus_iterable(a, b);
    }

    if a.kind() == ElementKind::List && b.kind() == ElementKind::Array {
        let b_allows_empty = !interner().get_array(b).flags.non_empty();
        let head = if b_allows_empty {
            let i = interner();
            let a_info = *i.get_list(a);
            i.intern_list(ListInfo { flags: a_info.flags.with_non_empty(true), ..a_info })
        } else {
            a
        };

        let b_ty = interner().intern_type(&[b], FlowFlags::EMPTY);
        return Some(vec![ElementId::intersected(head, &[ElementId::negated(b_ty)])]);
    }

    if a.kind() == ElementKind::Array && b.kind() == ElementKind::List {
        let b_allows_empty = !interner().get_list(b).flags.non_empty();
        let head = if b_allows_empty {
            let i = interner();
            let a_info = *i.get_array(a);
            i.intern_array(KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info })
        } else {
            a
        };

        let b_ty = interner().intern_type(&[b], FlowFlags::EMPTY);
        return Some(vec![ElementId::intersected(head, &[ElementId::negated(b_ty)])]);
    }

    if a.kind() == ElementKind::Iterable && b.kind() == ElementKind::Iterable {
        return iterable::iterable_minus(a, b);
    }

    if a.kind() == ElementKind::Callable && b.kind() == ElementKind::Callable {
        return callable::callable_minus(a, b);
    }

    if a.kind() == ElementKind::HasMethod && b.kind() == ElementKind::HasMethod {
        return has_member::has_method_minus(a, b);
    }

    if a.kind() == ElementKind::HasProperty && b.kind() == ElementKind::HasProperty {
        return has_member::has_property_minus(a, b);
    }

    None
}
