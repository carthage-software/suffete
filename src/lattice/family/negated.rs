//! Negated family: `!T`, the complement of `T` against `mixed`.
//!
//! Semantically `!T` = `mixed \ T`. The lattice rules fall out from
//! that definition:
//!
//! - **`X <: !T`** iff every value of `X` is outside `T`. Equivalent
//!   to `meet(X, T) ≡ ⊥`. Equivalent to `!overlaps(X, T)` (since the
//!   empty meet is exactly the absence of overlap).
//! - **`!T <: X`** iff `X` covers `mixed \ T`. Special-case: if `X`
//!   is `mixed`, true. If `X` is `!U` and `U <: T`, then `!T <: !U`
//!   contravariantly. Otherwise we'd need to enumerate `mixed`'s
//!   complement of `T` against `X`, which the lattice can't do
//!   structurally without exhaustive case analysis. We answer
//!   conservatively `false`.
//! - **`!T <: !U`** iff `U <: T` (contravariance through negation).
//!
//! The dispatch sees these via the standard refines path: `input` of
//! kind `Negated` enters [`refines_input_negated`], `container` of
//! kind `Negated` enters [`refines_container_negated`].

use crate::ElementId;
use crate::ElementKind;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

/// `X <: !T` iff `meet(X, T) ≡ ⊥`. The check is by structural
/// disjointness: if `X` has no value in common with `T`, every
/// value of `X` lies outside `T`, satisfying the negation.
pub fn refines_container_negated<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let i = interner();
    let info = *i.get_negated(container);
    let input_t = i.intern_type(&[input], crate::FlowFlags::EMPTY);
    !crate::lattice::overlaps(input_t, info.inner, world, options, report)
}

/// `!T <: X` is true only in the structurally obvious cases:
///
/// - `X = mixed` (the absolute top accepts every value).
/// - `X = !U` where `U <: T` (contravariance through the negation).
///
/// Other `X` shapes would require enumerating `mixed \ T` and
/// proving every member fits `X`, which the lattice doesn't try
/// without an exhaustive partition; we return `false` conservatively.
pub fn refines_input_negated<W: World>(
    input: ElementId,
    container: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if container == crate::prelude::MIXED {
        return true;
    }
    let i = interner();
    let input_info = *i.get_negated(input);
    if container.kind() == ElementKind::Negated {
        let container_info = *i.get_negated(container);
        // Contravariance: `!T <: !U` iff `U <: T`.
        return crate::lattice::refines(container_info.inner, input_info.inner, world, options, report);
    }
    false
}
