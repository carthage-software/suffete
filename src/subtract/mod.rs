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
use crate::element::payload::GenericParameterInfo;
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

    if crate::lattice::overlaps::is_uninhabited(b, world) {
        return vec![a];
    }

    if crate::lattice::overlaps::is_uninhabited(a, world) {
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

    if a.kind() == ElementKind::GenericParameter {
        return generic_parameter_minus(a, b, world, options, report).unwrap_or_else(|| vec![a]);
    }

    family_atom_minus(a, b).unwrap_or_else(|| vec![a])
}

/// `(T of X) \ Y`: narrow `T`'s constraint by removing `Y` from its
/// bound. When the new constraint is empty (every value of `T` was in
/// `Y`), the result is `[]` (impossible). When the same-`T` rule fires
/// (`(T of X) \ (T of Y) → T of (X \ Y)`), both sides agree on the
/// parameter identity. Otherwise the rhs is treated as a plain type
/// the constraint must shed.
fn generic_parameter_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_generic_parameter(a);

    let other_constraint = if b.kind() == ElementKind::GenericParameter {
        let b_info = *i.get_generic_parameter(b);
        if a_info.name != b_info.name || a_info.defining_entity != b_info.defining_entity {
            return None;
        }
        b_info.constraint
    } else {
        i.intern_type(&[b], FlowFlags::EMPTY)
    };

    let new_constraint = compute(a_info.constraint, other_constraint, world, options, report);
    if new_constraint == TYPE_NEVER {
        return Some(Vec::new());
    }
    let narrowed = i.intern_generic_parameter(GenericParameterInfo { constraint: new_constraint, ..a_info });
    Some(vec![narrowed])
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

    if a.kind() == ElementKind::String && b.kind() == ElementKind::String {
        return string_minus(a, b);
    }

    None
}

/// `String \ String` for axis-narrowing cases.
///
/// - Two distinct string literals: subtract is identity (the literal
///   sets are disjoint, but our `overlaps` returns `true` due to the
///   broader `String` family rules; we keep `a` unchanged here so the
///   distributive fold still terminates correctly).
/// - Equal literals: collapse to bottom.
/// - General string `\` non-empty / truthy string: only the empty
///   string `""` survives.
fn string_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    use crate::element::payload::scalar::StringCasing;
    use crate::element::payload::scalar::StringLiteral;

    let i = interner();
    let a_info = *i.get_string(a);
    let b_info = *i.get_string(b);

    if let StringLiteral::Value(av) = a_info.literal
        && let StringLiteral::Value(bv) = b_info.literal
        && av == bv
    {
        return Some(Vec::new());
    }

    let a_is_general = matches!(a_info.literal, StringLiteral::None | StringLiteral::Unspecified)
        && a_info.flags == crate::element::payload::scalar::StringRefinementFlags::EMPTY
        && matches!(a_info.casing, StringCasing::Unspecified);
    let b_requires_non_empty = b_info.flags.is_non_empty() || b_info.flags.is_truthy();
    if a_is_general && b_requires_non_empty {
        return Some(vec![ElementId::string_literal("")]);
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
