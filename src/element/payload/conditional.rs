use std::mem::size_of;

use crate::TypeId;

/// A `T is U ? V : W` conditional type (or its `T is not U` form via
/// [`Self::negated`]).
///
/// Frozen until the `subject` is concrete; then evaluates to `then` or
/// `otherwise` depending on whether `subject <: target` holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConditionalInfo {
    pub subject: TypeId,
    pub target: TypeId,
    pub then: TypeId,
    pub otherwise: TypeId,
    pub negated: bool,
}

const _: () = assert!(size_of::<ConditionalInfo>() <= 24);
