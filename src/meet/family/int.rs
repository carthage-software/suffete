//! `Int` family meet: range / literal intersection.

use crate::ElementId;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::prelude::NON_ZERO_INT;

/// Intersect two `Int` atoms. Subsumption (e.g. `INT ∧ Range(0,10)`)
/// is handled by the caller; this only fires when neither side
/// refines the other, which means both are bounded ranges or distinct
/// literals. The result is `Range(max(lo), min(hi))` collapsed to a
/// `Literal` when the bounds coincide, or `None` when the interval is
/// empty.
///
/// `NonZero` participates as either side: against another `NonZero`
/// it stays itself; against a literal `n` the result is `n` when
/// non-zero (else `None`); against a range that does not include `0`
/// the range wins (it's strictly more refined); against a range that
/// straddles `0` the precise meet is two disjoint sub-ranges, which
/// the single-atom return cannot express, so this case yields `None`
/// (a sound but imprecise drop — the missing values are recovered
/// when the same case lands in `subtract`).
pub(in crate::meet) fn int_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_int(a);
    let b_info = *i.get_int(b);

    if matches!((a_info, b_info), (IntInfo::NonZero, IntInfo::NonZero)) {
        return Some(NON_ZERO_INT);
    }
    if a_info == IntInfo::NonZero {
        return non_zero_meet(b_info, b);
    }
    if b_info == IntInfo::NonZero {
        return non_zero_meet(a_info, a);
    }

    let (al, au) = int_bounds(a_info);
    let (bl, bu) = int_bounds(b_info);

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

fn non_zero_meet(other_info: IntInfo, other: ElementId) -> Option<ElementId> {
    match other_info {
        IntInfo::Unspecified | IntInfo::UnspecifiedLiteral => Some(NON_ZERO_INT),
        IntInfo::Literal(0) => None,
        IntInfo::Literal(_) => Some(other),
        IntInfo::Range(rid) => {
            let r = *interner().get_int_range(rid);
            let lo = r.lower();
            let hi = r.upper();
            let contains_zero = match (lo, hi) {
                (Some(l), Some(h)) => l <= 0 && 0 <= h,
                (Some(l), None) => l <= 0,
                (None, Some(h)) => 0 <= h,
                (None, None) => true,
            };
            if !contains_zero {
                Some(other)
            } else if matches!(lo, Some(0)) && matches!(hi, Some(0)) {
                None
            } else {
                // Range straddles zero — splitting into [lo, -1] | [1, hi]
                // can't fit a single atom return. Conservative drop.
                None
            }
        }
        IntInfo::NonZero => Some(NON_ZERO_INT),
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
        // Caller routes NonZero through `non_zero_meet`; bounds aren't
        // meaningful as a single interval, so default to open-open.
        IntInfo::NonZero => (None, None),
    }
}
