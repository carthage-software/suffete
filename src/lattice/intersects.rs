//! Intersection (overlap) relation: `intersects(a, b)` is `true` iff there
//! exists a runtime value `v` such that `v ∈ a ∩ b`.
//!
//! Symmetric: `intersects(a, b) == intersects(b, a)`. Distinct from
//! `refines`: `int<0,10>` and `int<5,15>` overlap (value 7 inhabits both)
//! without either refining the other.
//!
//! Strategy: distribute over union (any element pair on the two sides
//! that overlaps proves the whole types overlap), then for each element
//! pair fall through these rules in order:
//!
//! 1. Reflexivity / Top / Bot axioms.
//! 2. Generic-parameter projection — `T` overlaps `X` iff `T`'s constraint
//!    overlaps `X`.
//! 3. Subsumption — `a <: b` or `b <: a` implies overlap.
//! 4. Family-specific positive overlap rules (e.g. range overlap, the
//!    string/class-like-string crossing, narrowed-mixed conservatism).
//!
//! When none of those fire we report disjoint. The rule set is incomplete
//! by design — adding a positive rule never weakens correctness, since the
//! relation is monotone in true outcomes; missing rules only cost
//! precision (a downstream narrowing returns `never` instead of a real
//! overlap).

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::MixedInfo;
use crate::element::payload::Truthiness;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::lattice::family::mixed as mixed_family;
use crate::lattice::refines::element_refines;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::PLACEHOLDER;
use crate::world::World;

pub fn intersects<W: World>(
    a: TypeId,
    b: TypeId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    let a_type = a.as_ref();
    let b_type = b.as_ref();

    a_type
        .elements
        .iter()
        .any(|x| b_type.elements.iter().any(|y| element_intersects(*x, *y, codebase, options, report)))
}

fn element_intersects<W: World>(
    a: ElementId,
    b: ElementId,
    codebase: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> bool {
    if a == NEVER || b == NEVER {
        return false;
    }
    if a == b {
        return true;
    }
    if a == MIXED || b == MIXED || a == PLACEHOLDER || b == PLACEHOLDER {
        return true;
    }

    if a.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(a).constraint;
        let other = interner().intern_type(&[b], FlowFlags::EMPTY);
        return intersects(constraint, other, codebase, options, report);
    }
    if b.kind() == ElementKind::GenericParameter {
        let constraint = interner().get_generic_parameter(b).constraint;
        let other = interner().intern_type(&[a], FlowFlags::EMPTY);
        return intersects(constraint, other, codebase, options, report);
    }

    if element_refines(a, b, codebase, options, report) || element_refines(b, a, codebase, options, report) {
        return true;
    }

    family_overlap(a, b)
}

fn family_overlap(a: ElementId, b: ElementId) -> bool {
    if a.kind() == ElementKind::Int && b.kind() == ElementKind::Int {
        return int_overlap(a, b);
    }

    if a.kind() == ElementKind::Mixed || b.kind() == ElementKind::Mixed {
        return mixed_overlap(a, b);
    }

    let pair = (a.kind(), b.kind());
    if matches!(
        pair,
        (ElementKind::String, ElementKind::ClassLikeString) | (ElementKind::ClassLikeString, ElementKind::String)
    ) {
        return true;
    }

    false
}

/// Narrowed-mixed overlap: each side's axis flags must be jointly
/// satisfiable by some runtime value the other side admits. Vanilla
/// `mixed` is already absorbed by the Top axiom, so at least one side
/// here carries a non-trivial axis.
fn mixed_overlap(a: ElementId, b: ElementId) -> bool {
    let (mixed, other) = if a.kind() == ElementKind::Mixed { (a, b) } else { (b, a) };
    if !mixed_axes_compatible(*interner().get_mixed(mixed), other) {
        return false;
    }
    if other.kind() == ElementKind::Mixed && !mixed_axes_compatible(*interner().get_mixed(other), mixed) {
        return false;
    }
    true
}

fn mixed_axes_compatible(info: MixedInfo, other: ElementId) -> bool {
    if info.is_non_null() && !mixed_family::is_non_null(other) {
        return false;
    }
    let other_truth = mixed_family::truthiness_of(other);
    match info.truthiness() {
        Truthiness::Truthy if other_truth == Truthiness::Falsy => return false,
        Truthiness::Falsy if other_truth == Truthiness::Truthy => return false,
        _ => {}
    }
    if info.is_empty() && other_truth == Truthiness::Truthy {
        return false;
    }
    true
}

/// Intervals (with absorption: `INT` and `LITERAL_INT` are unbounded) on
/// either side overlap iff `max(lo_a, lo_b) ≤ min(hi_a, hi_b)`. An open
/// bound on either side is treated as `±∞`.
fn int_overlap(a: ElementId, b: ElementId) -> bool {
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
