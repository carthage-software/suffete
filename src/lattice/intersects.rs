//! Intersection (overlap) relation: `intersects(a, b)` is `true` iff there
//! exists a runtime value `v` such that `v ∈ a ∩ b`.
//!
//! This is symmetric: `intersects(a, b) == intersects(b, a)`. It is NOT the
//! same as "either is a subtype of the other" — for example,
//! `int<0,10>` and `int<5,15>` intersect (value 7 is in both) but neither
//! refines the other.
//!
//! The current implementation is a placeholder: it returns `true` whenever
//! either side refines the other, plus the trivial `mixed`/`never` cases.
//! Family-specific overlap rules (range overlap, string-axis intersection,
//! object hierarchy meet, etc.) layer in as each family's intersection
//! logic lands.

use crate::TypeId;
use crate::lattice::Codebase;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::refines::generalizes;
use crate::lattice::refines::refines;

/// `true` iff `a` and `b` share at least one runtime value.
///
/// **WARNING:** This is currently incomplete. It returns `true` only when
/// the lattice can prove overlap via subtype edges; pairs that overlap
/// without either side refining the other (e.g. overlapping int ranges) may
/// return `false` until family-specific overlap rules land.
///
/// Returning `false` is therefore not yet a guarantee of disjointness; only
/// `true` is sound to act on (modulo the current rule coverage).
pub fn intersects<C: Codebase>(
    a: TypeId,
    b: TypeId,
    codebase: &C,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == b {
        return true;
    }

    refines(a, b, codebase, options, report) || generalizes(a, b, codebase, options, report)
}
