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

    // `subtract(X, !T)` ≡ `meet(X, T)`: removing "everything but T"
    // keeps the part of X that lands inside T. `subtract(!T, X)`
    // ≡ `!(T ∪ X)` (push X into the negation). Routing here keeps
    // the duality with meet symmetric and lets the existing meet
    // rules carry the work.
    if b.kind() == ElementKind::Negated {
        let neg_info = *interner().get_negated(b);
        let kept = crate::meet::compute(
            interner().intern_type(&[a], FlowFlags::EMPTY),
            neg_info.inner,
            world,
            options,
            report,
        );
        return kept.as_ref().elements.to_vec();
    }
    if a.kind() == ElementKind::Negated {
        let neg_info = *interner().get_negated(a);
        let mut union_elems: Vec<ElementId> = neg_info.inner.as_ref().elements.to_vec();
        union_elems.push(b);
        let union_ty = interner().intern_type(&union_elems, FlowFlags::EMPTY);
        return vec![ElementId::negated(union_ty)];
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

    if let Some(pieces) = true_union_minus(a, b, world, options, report) {
        return pieces;
    }

    if let Some(pieces) = object_descendant_minus(a, b, world) {
        return pieces;
    }

    if let Some(pieces) = family_atom_minus(a, b) {
        return pieces;
    }

    // `mixed \ B` and `nonnull-mixed \ B` have no positive
    // simplification once the family rules are exhausted, but with
    // the `Negated` element they have a precise representation as
    // the complement of the union of removed atoms. Without these
    // fallbacks the result would be order-dependent: subtracting
    // `[null, int]` versus `[int, null]` could land on
    // `nonnull-mixed` (over-approximate) versus `!(int|null)`
    // (precise) and break anti-monotonicity downstream.
    if a == MIXED {
        let b_t = interner().intern_type(&[b], FlowFlags::EMPTY);
        return vec![ElementId::negated(b_t)];
    }
    if a == NON_NULL_MIXED {
        let union_ty = interner().intern_type(&[NULL, b], FlowFlags::EMPTY);
        return vec![ElementId::negated(union_ty)];
    }

    vec![a]
}

/// `Object \ B` precision via `Negated` conjuncts on the surviving
/// object's intersection list. Four shapes fire:
///
/// - **Strict bare descendant.** `b` is a bare nominal descendant
///   of `a` (no `type_args` / `intersections`). Excluding the bare
///   descendant subsumes every value of `b`'s nominal subtree, so
///   the negation is exact.
/// - **Specialized descendant.** `b` descends `a` but carries
///   `type_args` or `intersections` (e.g. `C<int> \ A<int>` when
///   `A` extends `C`). Recording `b` itself as the excluded atom
///   removes the specific specialization without over-excluding
///   sibling specializations of the same nominal class.
/// - **Same class, different type args.** Under non-invariant
///   variance the value-sets can have a non-trivial difference
///   (`B<never> \ B<object>` under contravariant `T` leaves the
///   B-instances whose `T`-view doesn't contain `object`).
/// - **Structural narrowing.** `b` is a `HasMethod` /
///   `HasProperty` / `ObjectShape` atom. Open-world objects can
///   gain or lack the structural feature, so the precise residual
///   is "values of `a` that don't satisfy the structural
///   constraint" — represented as `a & !b` via a `Negated`
///   conjunct.
///
/// All other shapes keep the conservative identity fallback so we
/// don't introduce asymmetric precision the rest of the lattice
/// can't yet honor.
fn object_descendant_minus<W: World>(a: ElementId, b: ElementId, world: &W) -> Option<Vec<ElementId>> {
    if a.kind() != ElementKind::Object {
        return None;
    }
    let i = interner();
    let a_info = *i.get_object(a);

    let b_is_object = b.kind() == ElementKind::Object;
    let b_is_structural =
        matches!(b.kind(), ElementKind::HasMethod | ElementKind::HasProperty | ElementKind::ObjectShape);

    let (strict_bare_descendant, specialized_descendant, same_class_different_args) = if b_is_object {
        let b_info = *i.get_object(b);
        let descends = a_info.name != b_info.name && world.descends_from(b_info.name, a_info.name);
        let strict = descends && b_info.type_args.is_none() && b_info.intersections.is_none();
        let specialized = descends && !strict;
        let same_class = a_info.name == b_info.name && a_info.type_args != b_info.type_args;
        (strict, specialized, same_class)
    } else {
        (false, false, false)
    };

    if !strict_bare_descendant && !specialized_descendant && !same_class_different_args && !b_is_structural {
        return None;
    }

    let exclude_atom = if strict_bare_descendant {
        let b_info = *i.get_object(b);
        i.intern_object(crate::element::payload::ObjectInfo { intersections: None, ..b_info })
    } else {
        b
    };
    let exclude_ty = i.intern_type(&[exclude_atom], FlowFlags::EMPTY);
    let new_negated = ElementId::negated(exclude_ty);

    let mut conjuncts: Vec<ElementId> = Vec::new();
    if let Some(id) = a_info.intersections {
        for &existing in i.get_element_list(id) {
            if strict_bare_descendant && existing.kind() == ElementKind::Negated {
                let neg_info = *i.get_negated(existing);
                let inner_elements = neg_info.inner.as_ref().elements;
                if inner_elements.len() == 1 && inner_elements[0].kind() == ElementKind::Object {
                    let existing_info = *i.get_object(inner_elements[0]);
                    let b_info = *i.get_object(b);
                    if world.descends_from(b_info.name, existing_info.name) {
                        return Some(vec![a]);
                    }
                }
            }
            conjuncts.push(existing);
        }
    }
    if !conjuncts.contains(&new_negated) {
        conjuncts.push(new_negated);
    }
    conjuncts.sort();

    let new_info =
        crate::element::payload::ObjectInfo { intersections: Some(i.intern_element_list(&conjuncts)), ..a_info };
    Some(vec![i.intern_object(new_info)])
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

/// Fan out a true-union dominator (`scalar`, `numeric`, `array-key`)
/// when the right-hand side is a member of one of its sub-families.
/// The dominator's value-set is the disjoint union of its members;
/// subtracting splits the dominator into its constituents and
/// delegates the in-family subtraction to the recursive call.
///
/// `scalar = bool | int | float | string`,
/// `numeric = int | float | numeric-string`,
/// `array-key = int | string`.
fn true_union_minus<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<Vec<ElementId>> {
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::BOOL;
    use crate::prelude::FLOAT;
    use crate::prelude::INT;
    use crate::prelude::NUMERIC;
    use crate::prelude::NUMERIC_STRING;
    use crate::prelude::SCALAR;
    use crate::prelude::STRING;

    let members: &[ElementId] = if a == SCALAR {
        &[BOOL, INT, FLOAT, STRING]
    } else if a == NUMERIC {
        &[INT, FLOAT, NUMERIC_STRING]
    } else if a == ARRAY_KEY {
        &[INT, STRING]
    } else {
        return None;
    };

    // Only fan out when `b` lands inside one of the sub-families.
    // Otherwise we'd needlessly re-emit the dominator's constituents
    // for an unrelated subtraction (e.g. `scalar \ Foo`).
    if !members.iter().any(|m| dominator_member_covers(*m, b)) {
        return None;
    }

    let mut pieces: Vec<ElementId> = Vec::with_capacity(members.len());
    for &m in members {
        for piece in atom_minus(m, b, world, options, report) {
            pieces.push(piece);
        }
    }
    Some(pieces)
}

/// `true` iff member `m` and `b` share at least one runtime axis,
/// so splitting the dominator into its members would let the
/// per-member subtract drop or narrow some pieces. Covers two
/// shapes:
///
/// - Same-axis: `b` is the same primitive family as `m`
///   (`int \ int`, `string \ string`, etc.) so the family rule can
///   refine.
/// - Subsuming-axis: `b` is itself a true-union dominator that
///   contains values of `m`'s kind (`array-key \ numeric` splits
///   into `int|string`, and the `int` piece collapses to `never`
///   because `int <: numeric`).
///
/// Without the subsuming-axis case the dominator is preserved
/// intact even when its constituents are precisely subtractable,
/// breaking anti-monotonicity (`(a\c) <: (a\b)` for `b <: c`)
/// against more precise siblings on the other axis.
fn dominator_member_covers(m: ElementId, b: ElementId) -> bool {
    use ElementKind::*;
    match (m.kind(), b.kind()) {
        (Bool, Bool | True | False) => true,
        (Int, Int) => true,
        (Float, Float) => true,
        (String, String | ClassLikeString) => true,
        (Int, Numeric | Scalar | ArrayKey) => true,
        (Float, Numeric | Scalar) => true,
        (Bool, Scalar) => true,
        (String, Numeric | Scalar | ArrayKey) => true,
        _ => false,
    }
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

    // The "general string \ non-empty/truthy" → empty-literal rule
    // only applies when `b` is the *broad* non-empty/truthy string
    // (no literal value): subtracting `non-empty-string` from
    // `string` leaves exactly `""`. A specific literal like
    // `'foo'` removes only one value, so the complement is still
    // the full `string` lattice (no canonical form for
    // "string except 'foo'", subtract is identity).
    let b_is_broad = matches!(b_info.literal, StringLiteral::None | StringLiteral::Unspecified);
    let b_requires_non_empty = b_info.flags.is_non_empty() || b_info.flags.is_truthy();
    if a_is_general && b_is_broad && b_requires_non_empty {
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
