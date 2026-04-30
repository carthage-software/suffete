use std::mem::size_of;

use crate::TypeId;

/// `!T`, the complement of `T` against the universal type (`mixed`).
///
/// Free-standing: `!resource` is a valid parameter type and means
/// "anything that is not a resource". Inside an intersection list,
/// `Object & !D` narrows the object's value-set by removing every
/// instance of `D` (and `D`'s descendants).
///
/// Soundness invariants the interner enforces at construction:
///
/// - `!never` collapses to `mixed` (the inverse identity).
/// - `!mixed` collapses to `never`.
/// - `!!T` collapses to `T`.
///
/// The remaining cases store the inner [`TypeId`] verbatim. The
/// lattice's refines / overlaps / meet / subtract rules answer
/// queries against the structural form without further unfolding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NegatedInfo {
    pub inner: TypeId,
}

const _: () = assert!(size_of::<NegatedInfo>() <= 8);

impl std::fmt::Display for NegatedInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let elements = self.inner.as_ref().elements;
        if elements.len() == 1 && !elements[0].has_intersection_types() {
            write!(f, "!{}", elements[0])
        } else {
            write!(f, "!({})", self.inner)
        }
    }
}
