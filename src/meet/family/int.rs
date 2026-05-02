//! `Int` family meet: range / literal intersection.

use crate::ElementId;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;

/// Intersect two `Int` atoms. Subsumption (e.g. `INT ∧ Range(0,10)`)
/// is handled by the caller; this only fires when neither side
/// refines the other, which means both are bounded ranges or distinct
/// literals. The result is `Range(max(lo), min(hi))` collapsed to a
/// `Literal` when the bounds coincide, or `None` when the interval is
/// empty.
pub(in crate::meet) fn int_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
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

#[inline]
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
