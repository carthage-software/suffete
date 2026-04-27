//! Element-level transformation primitives over a [`TypeId`].
//!
//! Four pure functions ([`map`], [`flat_map`], [`filter_map`],
//! [`filter`]) traverse a type **structurally**: the closure is
//! invoked at every element position, including elements buried
//! inside nested type-carriers (object type-arguments, list element
//! types, keyed-array keys/values/known items, iterable key/value,
//! class-like-string constraints, conditional branches, derived
//! operands, generic-parameter constraints, callable signatures).
//!
//! These primitives are the shared structural walker used by
//! [`crate::widen`] and (in a follow-up refactor) by
//! [`crate::expand`], [`crate::template::standin`], and
//! [`crate::template::substitute`].
//!
//! # Order
//!
//! Post-order. Nested types are rebuilt first, then the closure is
//! invoked on the (possibly rebuilt) element. The closure therefore
//! sees an element whose nested types have already been transformed —
//! useful when the decision depends on the final shape of the
//! children.
//!
//! # Cost model
//!
//! At each type-level the walker accumulates results in a single
//! `Vec<ElementId>` and runs `intern_type` exactly once when it
//! commits. Nested type-levels each commit once for their level. A
//! type with `N` top-level elements that each become `K` elements
//! after `flat_map` costs **one** top-level intern (not `N`) plus the
//! per-nested-level interns dictated by the structure.
//!
//! When the closure returns each element unchanged at every level (and
//! no recursion observed a change), the original `TypeId` is returned
//! verbatim — no intern call at all.

mod walk;

use crate::ElementId;
use crate::TypeId;

use self::walk::Outcome;
use self::walk::walk;

/// Apply `f` to every element in `ty`, recursively descending into
/// nested types. The closure runs in post-order (after the element's
/// nested types have been transformed).
///
/// Returns the original `TypeId` unchanged when the closure returned
/// each element identical at every level.
pub fn map<F: FnMut(ElementId) -> ElementId>(ty: TypeId, mut f: F) -> TypeId {
    walk(ty, &mut |elem| {
        let replaced = f(elem);
        if replaced == elem { Outcome::Unchanged } else { Outcome::Single(replaced) }
    })
}

/// Apply `f` to every element, replacing each with zero or more
/// elements. Returning an empty iterator drops the element from the
/// surrounding union (collapses to `never` if the level becomes empty).
pub fn flat_map<F, I>(ty: TypeId, mut f: F) -> TypeId
where
    F: FnMut(ElementId) -> I,
    I: IntoIterator<Item = ElementId>,
{
    walk(ty, &mut |elem| {
        let collected: Vec<ElementId> = f(elem).into_iter().collect();
        match collected.as_slice() {
            [only] if *only == elem => Outcome::Unchanged,
            [only] => Outcome::Single(*only),
            [] => Outcome::Drop,
            _ => Outcome::Many(collected),
        }
    })
}

/// Apply `f` to every element, dropping any element for which `f`
/// returns `None`.
pub fn filter_map<F: FnMut(ElementId) -> Option<ElementId>>(ty: TypeId, mut f: F) -> TypeId {
    walk(ty, &mut |elem| match f(elem) {
        Some(replaced) if replaced == elem => Outcome::Unchanged,
        Some(replaced) => Outcome::Single(replaced),
        None => Outcome::Drop,
    })
}

/// Drop every element for which `predicate` returns `false`.
pub fn filter<F: FnMut(&ElementId) -> bool>(ty: TypeId, mut predicate: F) -> TypeId {
    walk(ty, &mut |elem| if predicate(&elem) { Outcome::Unchanged } else { Outcome::Drop })
}
