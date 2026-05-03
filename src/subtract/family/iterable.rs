//! `Iterable` family subtract: key/value narrowing via `Negated`
//! intersection conjuncts, mirroring [`super::list::list_minus`].

use crate::ElementId;
use crate::FlowFlags;
use crate::interner::interner;

/// `iterable<K1, V1> \ iterable<K2, V2>`. When key/value parameters
/// match exactly, the residue is empty. Otherwise return `a & !b` via
/// the [`Intersected`](crate::ElementKind::Intersected) wrapper.
pub(in crate::subtract) fn iterable_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_iterable(a);
    let b_info = *i.get_iterable(b);

    if a_info.key_type == b_info.key_type && a_info.value_type == b_info.value_type {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    Some(vec![ElementId::intersected(a, &[neg])])
}
