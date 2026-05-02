//! `Array` family subtract: empty-array-singleton elimination across
//! `Array \ Array` and `Array \ Iterable`.

use crate::ElementId;
use crate::element::payload::KeyedArrayInfo;
use crate::interner::interner;

/// `array<K, V>` empty-array elimination, mirror of
/// [`crate::subtract::family::list::list_minus`].
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

    let new_info = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    Some(vec![i.intern_array(new_info)])
}

/// `array<K, V> \ iterable<K2, V2>`: symmetric to
/// [`crate::subtract::family::list::list_minus_iterable`].
pub(in crate::subtract) fn array_minus_iterable(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let _ = b;
    let i = interner();
    let a_info = *i.get_array(a);
    if a_info.flags.non_empty() {
        return None;
    }
    if a_info.known_items.is_some() {
        return None;
    }
    let new_info = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    Some(vec![i.intern_array(new_info)])
}
