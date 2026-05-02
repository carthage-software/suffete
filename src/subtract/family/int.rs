#![allow(clippy::arithmetic_side_effects)]

//! Integer-range / literal subtract.

use crate::ElementId;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;

/// Difference of two integer atoms when neither side fully refines the
/// other. Produces 0, 1, or 2 surviving pieces, each of which is a
/// `Range` collapsed to a `Literal` when its bounds coincide.
pub(in crate::subtract) fn int_minus(a: ElementId, b: ElementId) -> Vec<ElementId> {
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
            let upper_bound = b_low - 1;
            let piece_hi = match ahi {
                Some(x) => Some(x.min(upper_bound)),
                None => Some(upper_bound),
            };

            if non_empty_interval(alo, piece_hi) {
                pieces.push(make_int_piece(alo, piece_hi));
            }
        }
    }

    if let Some(b_high) = bhi
        && let Some(lower_bound) = b_high.checked_add(1)
    {
        let a_ends_above = match ahi {
            Some(x) => x > b_high,
            None => true,
        };

        if a_ends_above {
            let piece_lo = match alo {
                Some(x) => Some(x.max(lower_bound)),
                None => Some(lower_bound),
            };

            if non_empty_interval(piece_lo, ahi) {
                pieces.push(make_int_piece(piece_lo, ahi));
            }
        }
    }

    pieces
}

#[inline]
const fn non_empty_interval(lo: Option<i64>, hi: Option<i64>) -> bool {
    match (lo, hi) {
        (Some(l), Some(h)) => l <= h,
        _ => true,
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

#[inline]
fn make_int_piece(lo: Option<i64>, hi: Option<i64>) -> ElementId {
    match (lo, hi) {
        (Some(l), Some(h)) if l == h => ElementId::int_literal(l),
        _ => ElementId::int_range(lo, hi),
    }
}
