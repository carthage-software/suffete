//! Int-family join: range merging and literal-count collapse.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::IntInfo;
use crate::interner::interner;
use crate::prelude::INT;

/// Merge adjacent integer literals and bounded ranges into wider
/// ranges. `Unspecified` and `UnspecifiedLiteral` are dominators /
/// virtual forms and stay as-is.
pub(in crate::join) fn apply_merge_int_ranges(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut intervals: Vec<(Option<i64>, Option<i64>)> = Vec::new();
    let mut other: Vec<ElementId> = Vec::with_capacity(elements.len());
    for &el in elements.iter() {
        if el.kind() != ElementKind::Int {
            other.push(el);
            continue;
        }
        match *i.get_int(el) {
            IntInfo::Literal(n) => intervals.push((Some(n), Some(n))),
            IntInfo::Range(rid) => {
                let r = *i.get_int_range(rid);
                intervals.push((r.lower(), r.upper()));
            }
            _ => other.push(el),
        }
    }

    if intervals.is_empty() {
        return;
    }

    intervals.sort_by(|a, b| match (a.0, b.0) {
        (None, None) => core::cmp::Ordering::Equal,
        (None, _) => core::cmp::Ordering::Less,
        (_, None) => core::cmp::Ordering::Greater,
        (Some(x), Some(y)) => x.cmp(&y),
    });

    let mut merged: Vec<(Option<i64>, Option<i64>)> = Vec::with_capacity(intervals.len());
    for r in intervals {
        if let Some(last) = merged.last_mut() {
            let adjacent = match (last.1, r.0) {
                (None, _) => true,
                (Some(_), None) => true,
                (Some(lu), Some(rl)) => lu.checked_add(1).is_some_and(|n| n >= rl),
            };
            if adjacent {
                last.1 = match (last.1, r.1) {
                    (None, _) | (_, None) => None,
                    (Some(a), Some(b)) => Some(a.max(b)),
                };
                continue;
            }
        }
        merged.push(r);
    }

    let mut new_elements: Vec<ElementId> = other;
    for (lo, hi) in merged {
        let elem = match (lo, hi) {
            (None, None) => INT,
            (Some(l), Some(h)) if l == h => ElementId::int_literal(l),
            _ => ElementId::int_range(lo, hi),
        };
        new_elements.push(elem);
    }
    *elements = new_elements;
}

/// Drop integer literals and add the broad `int` form when the
/// literal count exceeds `threshold`.
pub(in crate::join) fn apply_int_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    if crate::element::simd::contains(elements, INT) {
        return;
    }

    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::Int && matches!(i.get_int(**e), IntInfo::Literal(_)))
        .count();

    if count as u32 <= u32::from(threshold) {
        return;
    }

    elements.retain(|e| !(e.kind() == ElementKind::Int && matches!(i.get_int(*e), IntInfo::Literal(_))));
    let pos = elements.binary_search(&INT).unwrap_or_else(|p| p);
    elements.insert(pos, INT);
}
