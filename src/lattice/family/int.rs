//! Int family: `int`, `literal-int`, integer literals, bounded ranges.
//!
//! Container variants accept inputs as follows:
//!
//! - `Unspecified` (general `int`) accepts any Int-kind input.
//! - `UnspecifiedLiteral` (`literal-int`) accepts `Literal(_)` and itself.
//! - `Literal(N)` accepts only the same literal (handled by reflexivity).
//! - `Range(R)` accepts `Literal(N)` if `N ∈ R`, and `Range(R')` if `R' ⊆ R`.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Int {
        return false;
    }

    let i = interner();
    let container_info = *i.get_int(container);
    let input_info = *i.get_int(input);

    match (input_info, container_info) {
        (_, IntInfo::Unspecified) => true,
        (IntInfo::Literal(_) | IntInfo::UnspecifiedLiteral, IntInfo::UnspecifiedLiteral) => true,
        (IntInfo::Literal(n), IntInfo::Range(rid)) => {
            let r = *i.get_int_range(rid);
            range_contains_value(r, n)
        }
        (IntInfo::Range(input_rid), IntInfo::Range(container_rid)) => {
            let inner = *i.get_int_range(input_rid);
            let outer = *i.get_int_range(container_rid);
            range_contains_range(outer, inner)
        }
        _ => false,
    }
}

fn range_contains_value(range: IntRange, n: i64) -> bool {
    let lower_ok = match range.lower() {
        Some(lo) => lo <= n,
        None => true,
    };
    let upper_ok = match range.upper() {
        Some(hi) => n <= hi,
        None => true,
    };
    lower_ok && upper_ok
}

fn range_contains_range(outer: IntRange, inner: IntRange) -> bool {
    let lower_ok = match (outer.lower(), inner.lower()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(o), Some(i)) => o <= i,
    };
    let upper_ok = match (outer.upper(), inner.upper()) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(o), Some(i)) => i <= o,
    };
    lower_ok && upper_ok
}
