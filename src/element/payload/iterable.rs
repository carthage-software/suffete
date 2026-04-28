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

impl std::fmt::Display for IterableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = format!("iterable<{}, {}>", self.key_type, self.value_type);
        if let Some(id) = self.intersections {
            f.write_str("(")?;
            f.write_str(&base)?;
            f.write_str(")")?;
            for &conjunct in crate::interner::interner().get_element_list(id) {
                write!(f, "&{conjunct}")?;
            }
            Ok(())
        } else {
            f.write_str(&base)
        }
    }
}

impl IterableInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}
