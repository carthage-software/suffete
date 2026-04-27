//! Lattice join (least upper bound) of element multisets.
//!
//! [`compute`] takes a slice of [`ElementId`]s and returns the canonical
//! multiset that the corresponding union should hold. The pass is purely
//! structural: it inspects element identity and kind tags only, never
//! consults the lattice machinery, and so can run without any
//! subtype-driven information.
//!
//! In type-lattice terms, `compute(elements)` is the least upper bound
//! (join, ⊔) of the element multiset under the suffete subtype order.
//! A future `meet` (greatest lower bound, ⊓) module will pair with this
//! one when narrowing / intersection lands.
//!
//! # Why join is separate from interning
//!
//! The join preserves the subtype order. For any unions `A`, `B`:
//!
//! ```text
//! A ≤ B  ⟺  compute(A) ≤ B  ⟺  compute(A) ≤ compute(B)  ⟺  A ≤ compute(B)
//! ```
//!
//! That property is what lets the interner store unions in whatever shape
//! the caller hands in (sorted + deduplicated, but not otherwise canonical),
//! and the lattice answer refinement questions correctly on either side.
//! Calling [`compute`] is therefore an optional optimization for size and
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
//! Refinement-driven absorptions (`int ∨ Literal(N) → int` once the lattice
//! decides the literal refines the dominator, range merging, class hierarchy
//! collapse, etc.) require the lattice and a codebase, and are not applied
//! here.

use crate::ElementId;
use crate::ElementKind;
use crate::prelude::BOOL;
use crate::prelude::CALLABLE;
use crate::prelude::CLOSED_RESOURCE;
use crate::prelude::FALSE;
use crate::prelude::FLOAT;
use crate::prelude::INT;
use crate::prelude::MIXED;
use crate::prelude::NEVER;
use crate::prelude::OBJECT;
use crate::prelude::OPEN_RESOURCE;
use crate::prelude::RESOURCE;
use crate::prelude::STRING;
use crate::prelude::TRUE;
use crate::prelude::VOID;

/// Compute the join (least upper bound) of a slice of elements.
///
/// Returns a freshly-allocated, sorted, deduplicated [`Vec`] with the
/// canonicalization rules applied. Empty input collapses to `[NEVER]` so
/// callers always receive a non-empty multiset suitable for [`Type`]
/// construction.
///
/// [`Type`]: crate::Type
pub fn compute(elements: &[ElementId]) -> Vec<ElementId> {
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
    use crate::prelude::ARRAY_KEY;
    use crate::prelude::NULL;
    use crate::prelude::TYPE_BOOL;
    use crate::prelude::TYPE_INT_OR_STRING;
    use crate::prelude::TYPE_MIXED;

    #[test]
    fn empty_yields_never() {
        assert_eq!(compute(&[]), vec![NEVER]);
    }

    #[test]
    fn sorts_and_dedupes() {
        let a = ElementId::int_literal(99);
        let b = ElementId::int_literal(100);
        let r1 = compute(&[a, b]);
        let r2 = compute(&[b, a]);
        let r3 = compute(&[a, b, a, b, a]);
        assert_eq!(r1, r2);
        assert_eq!(r1, r3);
        assert_eq!(r1.len(), 2);
    }

    #[test]
    fn never_is_dropped_when_other_elements_exist() {
        assert_eq!(compute(&[NEVER, INT]), vec![INT]);
    }

    #[test]
    fn never_alone_is_preserved() {
        assert_eq!(compute(&[NEVER]), vec![NEVER]);
    }

    #[test]
    fn void_alone_is_preserved() {
        assert_eq!(compute(&[VOID]), vec![VOID]);
    }

    #[test]
    fn void_with_other_elements_is_dropped() {
        assert_eq!(compute(&[VOID, INT]), vec![INT]);
    }

    #[test]
    fn void_and_never_together_collapse_to_never() {
        assert_eq!(compute(&[VOID, NEVER]), vec![NEVER]);
    }

    #[test]
    fn true_or_false_merges_to_bool() {
        assert_eq!(compute(&[TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn bool_absorbs_true_and_false() {
        assert_eq!(compute(&[BOOL, TRUE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, FALSE]), vec![BOOL]);
        assert_eq!(compute(&[BOOL, TRUE, FALSE]), vec![BOOL]);
    }

    #[test]
    fn vanilla_mixed_absorbs_everything_else() {
        assert_eq!(compute(&[MIXED, INT, STRING, NEVER]), vec![MIXED]);
    }

    #[test]
    fn open_or_closed_resource_merges_to_resource() {
        assert_eq!(compute(&[OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn resource_absorbs_open_and_closed() {
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
        assert_eq!(compute(&[RESOURCE, OPEN_RESOURCE, CLOSED_RESOURCE]), vec![RESOURCE]);
    }

    #[test]
    fn unrelated_elements_are_preserved() {
        let mut out = compute(&[INT, STRING]);
        out.sort();
        let mut expected = vec![INT, STRING];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn null_and_array_key_kept_separate() {
        let mut out = compute(&[NULL, ARRAY_KEY]);
        out.sort();
        let mut expected = vec![NULL, ARRAY_KEY];
        expected.sort();
        assert_eq!(out, expected);
    }

    #[test]
    fn type_id_union_does_not_apply_join_rules() {
        // `TypeId::union` only sort+dedups via the interner; it does
        // not run the merges in `join::compute`. Callers wanting the
        // collapsed form route through `join::compute` explicitly.
        let pair = TypeId::union(&[TRUE, FALSE]);
        assert_ne!(pair, TYPE_BOOL);
        assert_eq!(pair.as_ref().elements, &[TRUE, FALSE]);

        let with_mixed = TypeId::union(&[MIXED, INT, STRING]);
        assert_ne!(with_mixed, TYPE_MIXED);
        assert_eq!(with_mixed.as_ref().elements.len(), 3);

        // Sort+dedup still happens, so unions of distinct elements
        // canonical to the well-known handle when slot order matches.
        let int_or_string = TypeId::union(&[INT, STRING]);
        assert_eq!(int_or_string, TYPE_INT_OR_STRING);
    }

    #[test]
    fn join_compute_then_union_collapses_to_well_known_handles() {
        let collapsed_bool = TypeId::union(&compute(&[TRUE, FALSE]));
        assert_eq!(collapsed_bool, TYPE_BOOL);

        let collapsed_mixed = TypeId::union(&compute(&[MIXED, INT, STRING]));
        assert_eq!(collapsed_mixed, TYPE_MIXED);
    }

    #[test]
    fn intern_type_does_not_canonicalize() {
        let i = interner();
        let raw = i.intern_type(&[TRUE, FALSE], FlowFlags::EMPTY);
        assert_eq!(raw.as_ref().elements, &[TRUE, FALSE]);
        assert_ne!(raw, TYPE_BOOL);
    }
}
