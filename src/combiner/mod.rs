//! Structural combination (canonicalization) of element multisets.
//!
//! [`combine`] takes a slice of [`ElementId`]s and returns the canonical
//! multiset that the corresponding union should hold. The pass is purely
//! structural: it inspects element identity and kind tags only, never
//! consults the comparator or a [`Codebase`](crate::comparator::Codebase),
//! and so can run without any subtype-driven information.
//!
//! # Why combination is separate from interning
//!
//! Combination preserves the subtype lattice. For any unions `A`, `B`:
//!
//! ```text
//! A ≤ B  ⟺  combine(A) ≤ B  ⟺  combine(A) ≤ combine(B)  ⟺  A ≤ combine(B)
//! ```
//!
//! That property is what lets the interner store unions in whatever shape
//! the caller hands in (sorted + deduplicated, but not otherwise canonical),
//! and the comparator answer subtype questions correctly on either side.
//! Calling `combine` is therefore an optional optimization for size and
//! readability, never a precondition for soundness.
//!
//! # What this pass does
//!
//! - Drops `void` and `never` when any non-bottom element exists; collapses
//!   an all-bottom multiset to `[never]`.
//! - Lets vanilla `mixed` absorb every other element.
//! - Merges `true ∨ false → bool`; lets `bool` absorb `true` / `false`.
//! - Lets `resource` absorb `open-resource` / `closed-resource`; merges
//!   `open-resource ∨ closed-resource → resource` when neither is dominated.
//! - Lets a same-kind dominator (`int`, `float`, `string`, `resource`,
//!   `callable`) absorb every other element of its kind.
//! - Lets `object` absorb the entire object family (named objects, enums,
//!   shapes, has-method, has-property).
//!
//! Subtype-driven absorptions (`int ∨ Literal(N) → int` when the comparator
//! decides the literal is a subtype, range merging, class hierarchy
//! collapse, etc.) require the comparator and a codebase, and are not
//! applied here.

use crate::ElementId;
use crate::ElementKind;
use crate::well_known::BOOL;
use crate::well_known::CALLABLE;
use crate::well_known::CLOSED_RESOURCE;
use crate::well_known::FALSE;
use crate::well_known::FLOAT;
use crate::well_known::INT;
use crate::well_known::MIXED;
use crate::well_known::NEVER;
use crate::well_known::OBJECT;
use crate::well_known::OPEN_RESOURCE;
use crate::well_known::RESOURCE;
use crate::well_known::STRING;
use crate::well_known::TRUE;
use crate::well_known::VOID;

/// Apply the structural canonicalization pass to a slice of elements.
///
/// Returns a freshly-allocated, sorted, deduplicated [`Vec`] with the
/// canonicalization rules applied. Empty input collapses to `[NEVER]` so
/// callers always receive a non-empty multiset suitable for [`Type`]
/// construction.
///
/// [`Type`]: crate::Type
pub fn combine(elements: &[ElementId]) -> Vec<ElementId> {
    let mut out: Vec<ElementId> = if elements.is_empty() { vec![NEVER] } else { elements.to_vec() };
    out.sort_unstable();
    out.dedup();
    canonicalize(&mut out);
    out
}

/// Apply the structural canonicalization rules. `elements` must be sorted
/// and deduplicated on entry; sorted order is preserved on exit.
fn canonicalize(elements: &mut Vec<ElementId>) {
    if elements.contains(&MIXED) {
        elements.clear();
        elements.push(MIXED);
        return;
    }

    let has_non_bottom = elements.iter().any(|e| *e != NEVER && *e != VOID);
    if has_non_bottom {
        elements.retain(|e| *e != NEVER && *e != VOID);
    } else if elements.len() > 1 {
        elements.clear();
        elements.push(NEVER);
    }

    let has_bool = elements.contains(&BOOL);
    let has_true = elements.contains(&TRUE);
    let has_false = elements.contains(&FALSE);

    if has_bool {
        elements.retain(|e| *e != TRUE && *e != FALSE);
    } else if has_true && has_false {
        elements.retain(|e| *e != TRUE && *e != FALSE);
        let pos = elements.binary_search(&BOOL).unwrap_or_else(|p| p);
        elements.insert(pos, BOOL);
    }

    let has_open_resource = elements.contains(&OPEN_RESOURCE);
    let has_closed_resource = elements.contains(&CLOSED_RESOURCE);
    let has_resource = elements.contains(&RESOURCE);
    if has_open_resource && has_closed_resource && !has_resource {
        elements.retain(|e| *e != OPEN_RESOURCE && *e != CLOSED_RESOURCE);
        let pos = elements.binary_search(&RESOURCE).unwrap_or_else(|p| p);
        elements.insert(pos, RESOURCE);
    }

    apply_same_kind_dominator(elements, INT);
    apply_same_kind_dominator(elements, FLOAT);
    apply_same_kind_dominator(elements, STRING);
    apply_same_kind_dominator(elements, RESOURCE);
    apply_same_kind_dominator(elements, CALLABLE);

    if elements.contains(&OBJECT) {
        elements.retain(|e| *e == OBJECT || !is_object_family_kind(e.kind()));
    }
}

/// If `dominator` is in `elements`, drop every other element of the same
/// kind (the dominator is the unrefined / top-of-its-family form).
fn apply_same_kind_dominator(elements: &mut Vec<ElementId>, dominator: ElementId) {
    if !elements.contains(&dominator) {
        return;
    }

    let kind = dominator.kind();
    elements.retain(|e| *e == dominator || e.kind() != kind);
}

/// `true` for the kinds that all sit under `Object::Any` and are absorbed by
/// it: named objects, enums (including specific cases), object shapes,
/// has-method / has-property narrowings.
fn is_object_family_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Object
            | ElementKind::Enum
            | ElementKind::ObjectShape
            | ElementKind::HasMethod
            | ElementKind::HasProperty
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FlowFlags;
    use crate::TypeId;
    use crate::interner::interner;
    use crate::well_known::ARRAY_KEY;
    use crate::well_known::NULL;
    use crate::well_known::TYPE_BOOL;
    use crate::well_known::TYPE_INT_OR_STRING;
    use crate::well_known::TYPE_MIXED;

    #[test]
    fn combine_empty_yields_never() {
        assert_eq!(combine(&[]), vec![NEVER]);
    }

    #[test]
    fn combine_sorts_and_dedupes() {
        let a = ElementId::int_literal(99);
        let b = ElementId::int_literal(100);
        let r1 = combine(&[a, b]);
        let r2 = combine(&[b, a]);
        let r3 = combine(&[a, b, a, b, a]);
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
        assert_eq!(r1.len(), 2);
    }

    #[test]
    fn never_is_dropped_when_other_elements_exist() {
        assert_eq!(combine(&[NEVER, INT]), vec![INT]);
    }

    #[test]
    fn never_alone_is_preserved() {
        assert_eq!(combine(&[NEVER]), vec![NEVER]);
    }

    #[test]
    fn void_alone_is_preserved() {
        assert_eq!(combine(&[VOID]), vec![VOID]);
    }

    #[test]
    fn void_with_other_elements_is_dropped() {
        assert_eq!(combine(&[VOID, INT]), vec![INT]);
    }

    #[test]
    fn void_and_never_together_collapse_to_never() {
        assert_eq!(combine(&[VOID, NEVER]), vec![NEVER]);
    }

    #[test]
    fn true_or_false_merges_to_bool() {
        assert_eq!(combine(&[TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn bool_absorbs_true_and_false() {
        assert_eq!(combine(&[BOOL, TRUE]), vec![BOOL]);
        assert_eq!(combine(&[BOOL, FALSE]), vec![BOOL]);
        assert_eq!(combine(&[BOOL, TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn vanilla_mixed_absorbs_everything_else() {
        assert_eq!(combine(&[MIXED, INT, STRING, NEVER]), vec![MIXED]);
    }

    #[test]
    fn open_or_closed_resource_merges_to_resource() {
        assert_eq!(combine(&[OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn resource_absorbs_open_and_closed() {
        assert_eq!(combine(&[RESOURCE, OPEN_RESOURCE]), vec![RESOURCE]);
        assert_eq!(combine(&[RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
        assert_eq!(combine(&[RESOURCE, OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn unrelated_elements_are_preserved() {
        let mut out = combine(&[INT, STRING]);
        out.sort();
        let mut expected = vec![INT, STRING];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn null_and_array_key_kept_separate() {
        let mut out = combine(&[NULL, ARRAY_KEY]);
        out.sort();
        let mut expected = vec![NULL, ARRAY_KEY];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn type_id_union_routes_through_combine() {
        // TypeId::union calls combine and then intern_type, so the well-known
        // unions stay reachable through the sugar API.
        let id = TypeId::union(&[INT, STRING]);
        assert_eq!(id, TYPE_INT_OR_STRING);

        let bool_id = TypeId::union(&[TRUE, FALSE]);
        assert_eq!(bool_id, TYPE_BOOL);

        let mixed_id = TypeId::union(&[MIXED, INT, STRING]);
        assert_eq!(mixed_id, TYPE_MIXED);
    }

    #[test]
    fn intern_type_does_not_canonicalize() {
        let i = interner();
        let raw = i.intern_type(&[TRUE, FALSE], FlowFlags::EMPTY);
        assert_eq!(raw.as_ref().elements, &[TRUE, FALSE]);
        assert_ne!(raw, TYPE_BOOL);
    }
}
