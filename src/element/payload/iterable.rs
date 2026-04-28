use std::mem::size_of;

use crate::ElementListId;
use crate::TypeId;

/// `iterable<K, V>`, optionally narrowed by intersection (e.g.
/// `iterable<int, string>&Countable`).
///
/// `iterable` is its own element because `array <: iterable` and
/// `Traversable <: iterable` both hold, but `iterable` does not commute with
/// arbitrary `Foo<K, V>` containers, so it can't be reduced to a union of
/// the two and must be tracked explicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IterableInfo {
    pub key_type: TypeId,
    pub value_type: TypeId,
    pub intersections: Option<ElementListId>,
}

const _: () = assert!(size_of::<IterableInfo>() <= 24);
