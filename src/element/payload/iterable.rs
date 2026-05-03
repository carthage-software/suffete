use core::mem::size_of;

use crate::TypeId;

/// `iterable<K, V>`.
///
/// `iterable` is its own element because `array <: iterable` and
/// `Traversable <: iterable` both hold, but `iterable` does not commute with
/// arbitrary `Foo<K, V>` containers, so it can't be reduced to a union of
/// the two and must be tracked explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IterableInfo {
    pub key_type: TypeId,
    pub value_type: TypeId,
}

const _: () = assert!(size_of::<IterableInfo>() <= 24, "size budget exceeded");

impl core::fmt::Display for IterableInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "iterable<{}, {}>", self.key_type, self.value_type)
    }
}

impl IterableInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}
