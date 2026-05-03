//! `Array` family subtract: empty-array elimination + key/value
//! narrowing via `Negated` intersection conjuncts.

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::interner::interner;
use crate::prelude::TYPE_NEVER;

/// `array<K1, V1> \ array<K2, V2>` (or against `non-empty-array<K2, V2>`).
/// The empty-array singleton survives when `a` allows empty and `b`
/// doesn't; otherwise drops. The non-empty residue is tightened by
/// attaching `Negated(b)` when key or value parameters differ, so the
/// lattice can later detect overlap-collapsing intersections like
/// `(non-empty-array<K1, V1> & Negated(array<K2, V2>)) ∩ array<K2, V2>
/// ≡ ⊥`.
pub(in crate::subtract) fn array_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);

    if a_info.known_items.is_some() || b_info.known_items.is_some() {
        return None;
    }

    let a_allows_empty = !a_info.flags.non_empty();
    let b_allows_empty = !b_info.flags.non_empty();

    let mut pieces: Vec<ElementId> = Vec::new();

    if a_allows_empty && !b_allows_empty {
        pieces.push(empty_array(i));
    }

    let non_empty_residue = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    if a_info.key_param != b_info.key_param || a_info.value_param != b_info.value_param {
        let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
        let neg = ElementId::negated(b_t);
        let head = i.intern_array(non_empty_residue);
        pieces.push(ElementId::intersected(head, &[neg]));
    } else if a_allows_empty == b_allows_empty {
        pieces.push(i.intern_array(non_empty_residue));
    } else {
        // Element params equal and `a` allows empty while `b` doesn't:
        // empty piece already pushed; non-empty values of `a` are
        // entirely covered by `b`, so no non-empty residue.
    }

    Some(pieces)
}

#[inline]
fn empty_array(i: &crate::interner::Interner) -> ElementId {
    i.intern_array(KeyedArrayInfo {
        key_param: Some(TYPE_NEVER),
        value_param: Some(TYPE_NEVER),
        known_items: None,
        flags: KeyedArrayFlags::default(),
    })
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

    let new_info = KeyedArrayInfo { flags: a_info.flags.with_non_empty(true), ..a_info };
    let head = i.intern_array(new_info);
    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    Some(vec![ElementId::intersected(head, &[neg])])
}
