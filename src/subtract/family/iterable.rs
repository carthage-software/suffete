//! `Iterable` family subtract: key/value narrowing via `Negated`
//! intersection conjuncts, mirroring [`super::list::list_minus`].

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::IterableInfo;
use crate::interner::interner;

/// `iterable<K1, V1> \ iterable<K2, V2>`. When key/value parameters
/// match exactly, the residue is empty (the lattice's `refines`
/// short-circuit upstream catches the equal case). Otherwise attach
/// `Negated(b)` to preserve the narrowing structurally.
pub(in crate::subtract) fn iterable_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_iterable(a);
    let b_info = *i.get_iterable(b);

    if a_info.key_type == b_info.key_type && a_info.value_type == b_info.value_type {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    let new_info = IterableInfo { intersections: Some(i.intern_element_list(&[neg])), ..a_info };
    Some(vec![i.intern_iterable(new_info)])
}
