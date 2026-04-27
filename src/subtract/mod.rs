//! Lattice difference: `A \ B` is the type whose values are in `A`
//! but not in `B`. Pairs with [`crate::meet`] the way negative
//! narrowing pairs with positive narrowing: `if ($x !== null)`
//! produces `subtract(T_x, null)`.
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
//! The operation is *partial* (intersection.md §3.3.2): when no rule
//! describes the precise difference, the input is returned unchanged.
//! Returning a superset of the true difference is sound — the
//! soundness invariants in §3.1 are
//!
//! - `result <: A` (no value escapes the original),
//! - `result ∧ B ≡ ⊥` *if precise*, `result ⊇ A \ B` always.
//!
//! # Strategy
//!
//! Difference distributes over union on the left and intersects with the
//! complement on the right (intersection.md §3.2):
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
//! 4. Otherwise return `α` unchanged (the spec's conservative fallback).

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::overlaps;
use crate::lattice::refines;
use crate::prelude::FALSE;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::NON_NULL_MIXED;
use crate::prelude::NULL;
use crate::prelude::TRUE;
use crate::prelude::TYPE_NEVER;
use crate::world::World;

/// Outcome of [`narrow`], classifying an assertion-driven difference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    pub fn into_type(self) -> TypeId {
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

    let mut atoms: Vec<ElementId> = Vec::new();
    for &x in input_type.elements.iter() {
        let pieces = subtract_all(x, narrowing_type.elements, world, options, report);
        atoms.extend(pieces);
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
pub fn compute<W: World>(
    a: TypeId,
    b: TypeId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> TypeId {
    narrow(a, b, world, options, report).into_type()
}

/// Apply `α \ β₁ \ β₂ \ … \ βₙ` by folding over the right-hand atoms.
fn subtract_all<W: World>(
    x: ElementId,
    bs: &[ElementId],
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Vec<ElementId> {
    let mut current: Vec<ElementId> = vec![x];
    for &b in bs {
        if current.is_empty() {
            break;
        }
        let mut next: Vec<ElementId> = Vec::new();
        for c in current {
            next.extend(atom_minus(c, b, world, options, report));
        }
        current = next;
    }
    current
}

fn atom_minus<W: World>(
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

    let i = interner();
    let a_t = i.intern_type(&[a], FlowFlags::EMPTY);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);

    if refines(a_t, b_t, world, options, report) {
        return Vec::new();
    }

    if !overlaps(a_t, b_t, world, options, report) {
        return vec![a];
    }

    family_atom_minus(a, b).unwrap_or_else(|| vec![a])
}

fn family_atom_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return Some(int_minus(a, b));
    }

    if a == MIXED && b == NULL {
        return Some(vec![NON_NULL_MIXED]);
    }

    if a == crate::prelude::BOOL && b == TRUE {
        return Some(vec![FALSE]);
    }
    if a == crate::prelude::BOOL && b == FALSE {
        return Some(vec![TRUE]);
    }

    None
}

/// Difference of two integer atoms when neither side fully refines the
/// other. Produces 0, 1, or 2 surviving pieces, each of which is a
/// `Range` collapsed to a `Literal` when its bounds coincide.
fn int_minus(a: ElementId, b: ElementId) -> Vec<ElementId> {
    let i = interner();
    let (alo, ahi) = int_bounds(*i.get_int(a));
    let (blo, bhi) = int_bounds(*i.get_int(b));

    let mut pieces: Vec<ElementId> = Vec::new();

    if let Some(b_low) = blo {
        let a_starts_below = match alo {
            Some(x) => x < b_low,
            None => true,
        };
        if a_starts_below {
            let piece_hi = b_low - 1;
            let piece_hi = match ahi {
                Some(x) => Some(x.min(piece_hi)),
                None => Some(piece_hi),
            };
            if non_empty_interval(alo, piece_hi) {
                pieces.push(make_int_piece(alo, piece_hi));
            }
        }
    }

    if let Some(b_high) = bhi
        && let Some(piece_lo) = b_high.checked_add(1)
    {
        let a_ends_above = match ahi {
            Some(x) => x > b_high,
            None => true,
        };
        if a_ends_above {
            let piece_lo = match alo {
                Some(x) => Some(x.max(piece_lo)),
                None => Some(piece_lo),
            };
            if non_empty_interval(piece_lo, ahi) {
                pieces.push(make_int_piece(piece_lo, ahi));
            }
        }
    }

    pieces
}

fn non_empty_interval(lo: Option<i64>, hi: Option<i64>) -> bool {
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

fn make_int_piece(lo: Option<i64>, hi: Option<i64>) -> ElementId {
    match (lo, hi) {
        (Some(l), Some(h)) if l == h => ElementId::int_literal(l),
        _ => ElementId::int_range(lo, hi),
    }
}
