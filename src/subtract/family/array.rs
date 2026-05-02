//! `Array` family subtract: empty-array elimination + key/value
//! narrowing via `Negated` intersection conjuncts.

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::interner::interner;

/// `array<K1, V1> \ array<K2, V2>`. Empty-array singleton drops out
/// when both sides allow empty. Mirror of
/// [`crate::subtract::family::list::list_minus`]: when the
/// key/value parameters differ between `a` and `b`, the residual non-
/// empty arrays are tightened by attaching `Negated(b)` so the lattice
/// can later detect overlap-collapsing intersections like
/// `(non-empty-array<K1, V1> & Negated(array<K2, V2>)) ∩ array<K2, V2>
/// ≡ ⊥`.
pub(in crate::subtract) fn array_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);
    if a_info.flags.non_empty() || b_info.flags.non_empty() {
        return None;
    }

    if a_info.known_items.is_some() || b_info.known_items.is_some() {
        return None;
    }

    let mut new_info = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    if a_info.key_param != b_info.key_param || a_info.value_param != b_info.value_param {
        let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
        let neg = ElementId::negated(b_t);
        new_info.intersections = Some(i.intern_element_list(&[neg]));
    }
    Some(vec![i.intern_array(new_info)])
}

/// `array<K, V> \ iterable<K2, V2>`: symmetric to
/// [`crate::subtract::family::list::list_minus_iterable`].
pub(in crate::subtract) fn array_minus_iterable(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_array(a);
    if a_info.flags.non_empty() {
        return None;
    }
    if a_info.known_items.is_some() {
        return None;
    }
    let mut new_info = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    new_info.intersections = Some(i.intern_element_list(&[neg]));
    Some(vec![i.intern_array(new_info)])
}
