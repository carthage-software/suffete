//! Short-circuiting deep walkers for boolean queries on a [`TypeId`].
//!
//! Where [`crate::transform`] *transforms* a type by applying a closure
//! at every element position, [`inspect`] *queries* — the closure is a
//! predicate, the walker stops as soon as the answer is known.
//!
//! Two free functions:
//!
//! - [`any`] — `true` iff at least one element (at any depth) satisfies
//!   the predicate. Stops at the first `true`.
//! - [`all`] — `true` iff every element (at every depth) satisfies the
//!   predicate. Stops at the first `false`. Vacuously `true` for the
//!   empty case (a type with no nested types and no top-level
//!   elements; in practice, the interner always materialises at least
//!   `[never]`, which still gets visited).
//!
//! The walk descends through every nested-type carrier the
//! [`crate::transform`] walker handles: object type-args + intersections,
//! list element / known elements, keyed-array params + known items,
//! iterable key/value, object-shape known properties, class-like-string
//! constraints, generic-parameter constraints, reference type-args +
//! intersections, conditional 4 operands, all 8 derived variants, and
//! callable signatures (return / params / throws).
//!
//! The closure is called at every level (top-level union elements,
//! plus every element nested inside any payload). It is **not**
//! called twice on the same element.

mod walk;

use crate::ElementId;
use crate::TypeId;

/// `true` iff at least one element in `ty` (at any depth) satisfies
/// `predicate`. Stops walking on the first match.
#[inline]
pub fn any<F: FnMut(ElementId) -> bool>(ty: TypeId, mut predicate: F) -> bool {
    self::walk::any(ty, &mut predicate)
}

/// `true` iff every element in `ty` (at every depth) satisfies
/// `predicate`. Stops walking on the first failure.
#[inline(always)]
pub fn all<F: FnMut(ElementId) -> bool>(ty: TypeId, mut predicate: F) -> bool {
    !any(ty, |elem| !predicate(elem))
}
