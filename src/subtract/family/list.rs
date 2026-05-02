//! `List` family subtract: empty-list elimination + element-type
//! narrowing via `Negated` intersection conjuncts.

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::ListInfo;
use crate::interner::interner;

/// `list<E1> \ list<E2>`. The empty-list singleton drops out when both
/// sides allow empty. The non-empty residue is tightened by attaching a
/// `Negated(list<E2>)` conjunct to the result, so the lattice can later
/// detect contradictions like `(non-empty-list<X> & Negated(list<Y>))
/// ∩ list<Y> ≡ ⊥` and prune the imprecise overlap.
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

    let mut new_info = ListInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    // Attach `Negated(list<E2>)` when the element types differ ; that's
    // the only case where the residual non-empty-list values still have
    // any chance of overlapping `b`. When element types are identical,
    // the residual is already disjoint from `b` modulo non-empty.
    if a_info.element_type != b_info.element_type {
        let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
        let neg = ElementId::negated(b_t);
        new_info.intersections = Some(i.intern_element_list(&[neg]));
    }

    Some(vec![i.intern_list(new_info)])
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
