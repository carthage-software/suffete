//! `List` family subtract: empty-list-singleton elimination across
//! `List \ List` and `List \ Iterable`.

use crate::ElementId;
use crate::element::payload::ListInfo;
use crate::interner::interner;

/// `list<E1> \ list<E2>` empty-list elimination: when both sides
/// have `non_empty=false`, the empty list is in `b` and gets
/// dropped from `a \ b`. Element-type narrowing for non-empty
/// pieces would need list intersections and isn't representable.
pub(in crate::subtract) fn list_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);
    if a_info.flags.non_empty() || b_info.flags.non_empty() {
        return None;
    }

    if a_info.known_elements.is_some() || b_info.known_elements.is_some() {
        return None;
    }

    let new_info = ListInfo { flags: a_info.flags.with_non_empty(true), ..a_info };

    Some(vec![i.intern_list(new_info)])
}

/// `list<E> \ iterable<K, V>`: every iterable accepts the empty
/// iterator, so when `a` allows the empty list it sits in `b` and
/// gets removed. Element-type narrowing on the non-empty pieces is
/// left to the post-fold union-coverage rescue.
pub(in crate::subtract) fn list_minus_iterable(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let _ = b;
    let i = interner();
    let a_info = *i.get_list(a);
    if a_info.flags.non_empty() {
        return None;
    }
    if a_info.known_elements.is_some() {
        return None;
    }
    let new_info = ListInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    Some(vec![i.intern_list(new_info)])
}
