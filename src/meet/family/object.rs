//! Compositional object intersection: when neither named object refines
//! the other, the meet is `Foo & Bar`, with the canonical-smallest
//! participant chosen as the head so the operation is commutative.

use crate::ElementId;
use crate::element::payload::ObjectInfo;
use crate::interner::interner;

/// `Foo ∧ Bar` collects every participant (both heads + any
/// pre-existing conjuncts on either side), strips intersections from
/// each, sorts/dedups, and picks the canonical smallest as the new
/// head with the rest as conjuncts.
///
/// `final` classes (which would force `Foo & Bar → never` when
/// unrelated) are not yet exposed by [`crate::world::World`], so this
/// function never short-circuits to disjoint. Adding that query is a
/// follow-up.
pub(in crate::meet) fn compose_object_intersection(a: ElementId, b: ElementId) -> ElementId {
    let i = interner();
    let a_info = *i.get_object(a);
    let b_info = *i.get_object(b);

    let mut participants: Vec<ElementId> = Vec::new();
    participants.push(i.intern_object(ObjectInfo { intersections: None, ..a_info }));
    if let Some(id) = a_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }
    participants.push(i.intern_object(ObjectInfo { intersections: None, ..b_info }));
    if let Some(id) = b_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }

    participants.sort();
    participants.dedup();

    let head_elem = participants.remove(0);
    let head_info = *i.get_object(head_elem);
    let intersections = if participants.is_empty() { None } else { Some(i.intern_element_list(&participants)) };
    i.intern_object(ObjectInfo { intersections, ..head_info })
}
