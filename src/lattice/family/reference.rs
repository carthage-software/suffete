//! Indirection family: `Variable`, `Reference`, `MemberReference`,
//! `GlobalReference`, `Alias`, `Conditional`, `Derived`.
//!
//! Per comparison.md §1.11 these atoms are normally **resolved by the
//! analyser before subtyping is consulted**: a `Reference` becomes the
//! type recorded for that name in `Γ`, an `Alias` substitutes its body,
//! a `Derived` evaluates once its inputs are concrete. Two unresolved
//! atoms refine each other only by *structural identity*.
//!
//! Interning gives us structural identity for free: equal payloads
//! intern to one handle, and the universal reflexivity axiom in
//! [`element_refines`](crate::lattice::refines) catches `input == container`
//! before this family is even consulted. As a result, this file's only job
//! is to keep the dispatch honest:
//!
//! - Same-kind input with a *different* handle implies the payloads
//!   differ structurally (otherwise interning would have collapsed them);
//!   without resolution we cannot decide subtyping, so the answer is
//!   `false`.
//! - Cross-kind input refines an indirection container only via
//!   resolution, which the analyser must run beforehand. Until then,
//!   `false`.
//!
//! Returning `false` is sound: it just means "the lattice cannot prove
//! this without resolution". A downstream analyser that resolves the
//! atom and re-asks gets the real answer.

use crate::ElementId;
use crate::ElementKind;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    if !is_indirection_kind(container.kind()) {
        return false;
    }

    if input == container {
        return true;
    }

    false
}

fn is_indirection_kind(kind: ElementKind) -> bool {
    matches!(
        kind,
        ElementKind::Variable
            | ElementKind::Reference
            | ElementKind::MemberReference
            | ElementKind::GlobalReference
            | ElementKind::Alias
            | ElementKind::Conditional
            | ElementKind::Derived
    )
}
