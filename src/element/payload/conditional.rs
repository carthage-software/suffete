use core::mem::size_of;

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

const _: () = assert!(size_of::<ConditionalInfo>() <= 40, "size budget exceeded");

impl core::fmt::Display for ConditionalInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let op = if self.negated { " is not " } else { " is " };
        write!(f, "({}{}{} ? {} : {})", self.subject, op, self.target, self.then, self.otherwise)
    }
}

impl ConditionalInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}
