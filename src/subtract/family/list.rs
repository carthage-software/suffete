//! `List` family subtract: empty-list elimination + element-type
//! narrowing via `Negated` intersection conjuncts.

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::interner::interner;
use crate::prelude::TYPE_NEVER;

/// `list<E1> \ list<E2>` (or `\ non-empty-list<E2>`). The empty-list
/// singleton drops out when both sides allow empty; otherwise it
/// survives. The non-empty residue is tightened by attaching a
/// `Negated(b)` conjunct when element types differ, so the lattice can
/// later detect contradictions like `(non-empty-list<X> & Negated(list<Y>))
/// ∩ list<Y> ≡ ⊥` and prune the imprecise overlap.
pub(in crate::subtract) fn list_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);

    if a_info.known_elements.is_some() || b_info.known_elements.is_some() {
        return None;
    }

    let a_allows_empty = !a_info.flags.non_empty();
    let b_allows_empty = !b_info.flags.non_empty();

    let mut pieces: Vec<ElementId> = Vec::new();

    if a_allows_empty && !b_allows_empty {
        pieces.push(empty_list(i));
    }

    let non_empty_residue =
        ListInfo { element_type: a_info.element_type, flags: a_info.flags.with_non_empty(true), ..a_info };
    if a_info.element_type != b_info.element_type {
        let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
        let neg = ElementId::negated(b_t);
        pieces
            .push(i.intern_list(ListInfo { intersections: Some(i.intern_element_list(&[neg])), ..non_empty_residue }));
    } else if a_allows_empty == b_allows_empty {
        pieces.push(i.intern_list(non_empty_residue));
    } else {
        // Element types equal and `a` allows empty while `b` doesn't:
        // the empty piece is already pushed above; non-empty values of
        // `a` are entirely covered by `b`, so no non-empty residue.
    }

    Some(pieces)
}

#[inline]
fn empty_list(i: &crate::interner::Interner) -> ElementId {
    i.intern_list(ListInfo {
        element_type: TYPE_NEVER,
        known_elements: None,
        known_count: None,
        intersections: None,
        flags: ListFlags::default(),
    })
}

/// `list<E> \ iterable<K, V>`: every iterable accepts the empty
/// iterator, so when `a` allows the empty list it sits in `b` and
/// gets removed. Element-type narrowing on the non-empty pieces is
/// captured by attaching a `Negated(iterable<K, V>)` conjunct, the
/// same way [`list_minus`] does for list-vs-list.
pub(in crate::subtract) fn list_minus_iterable(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_list(a);
    if a_info.flags.non_empty() {
        return None;
    }
    if a_info.known_elements.is_some() {
        return None;
    }
    let mut new_info = ListInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    new_info.intersections = Some(i.intern_element_list(&[neg]));
    Some(vec![i.intern_list(new_info)])
}
