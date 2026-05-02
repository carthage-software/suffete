//! `scalar` container: `bool | true | false | int | float | string |
//! class-like-string | array-key | numeric | scalar`.
//!
//! `null`, `void`, `never`, `mixed`, objects, resources, arrays, callables,
//! and iterables are NOT scalars.

use crate::ElementId;
use crate::ElementKind;

#[inline]
#[must_use]
pub const fn refines(input: ElementId, _container: ElementId) -> bool {
    matches!(
        input.kind(),
        ElementKind::Bool
            | ElementKind::True
            | ElementKind::False
            | ElementKind::Int
            | ElementKind::Float
            | ElementKind::String
            | ElementKind::ClassLikeString
            | ElementKind::ArrayKey
            | ElementKind::Numeric
            | ElementKind::Scalar
    )
}
