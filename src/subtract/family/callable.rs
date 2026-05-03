//! `Callable` family subtract: equal callables collapse to bottom;
//! otherwise wrap as `Intersected(a, [Negated(b)])` so the narrowing
//! survives.

use crate::ElementId;
use crate::FlowFlags;
use crate::interner::interner;

pub(in crate::subtract) fn callable_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    if a == b {
        return Some(Vec::new());
    }

    let i = interner();
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    Some(vec![ElementId::intersected(a, &[neg])])
}
