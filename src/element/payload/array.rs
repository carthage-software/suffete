use std::mem::size_of;
use std::num::NonZeroU32;

use mago_atom::Atom;

use crate::TypeId;
use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned `&'static [KnownItemEntry]`:
    /// [`KeyedArrayInfo`] known items.
    KnownItemsId
}

define_handle! {
    /// Handle to an interned `&'static [KnownElementEntry]`:
    /// [`ListInfo`] known elements.
    KnownElementsId
}

/// A literal key in a keyed-array shape.
///
/// `Const` carries an unresolved `Class::CONSTANT` reference; resolution
/// happens during the population phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ArrayKey {
    Int(i64),
    String(Atom),
    Const { class: Atom, name: Atom },
}

/// One entry in a keyed-array shape's known items list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownItemEntry {
    pub key: ArrayKey,
    pub value: TypeId,
    pub optional: bool,
}

/// One entry in a list shape's known elements list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownElementEntry {
    pub index: u32,
    pub value: TypeId,
    pub optional: bool,
}

/// `array<K, V>`, `array{a: int, ...}`, `array{}`.
///
/// `key_param` / `value_param` describe the rest type when present.
/// `known_items` is a sorted, interned list of fixed entries.
///
/// "Sealed" is the absence of a rest type: `key_param` and `value_param` both
/// `None` means the shape admits no extra entries beyond `known_items`. There
/// is intentionally no separate `sealed` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyedArrayInfo {
    pub key_param: Option<TypeId>,
    pub value_param: Option<TypeId>,
    pub known_items: Option<KnownItemsId>,
    pub flags: KeyedArrayFlags,
}

impl KeyedArrayInfo {
    /// `true` iff this shape admits no entries beyond its known items.
    #[inline]
    pub const fn is_sealed(&self) -> bool {
        self.key_param.is_none() && self.value_param.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyedArrayFlags(u8);

impl KeyedArrayFlags {
    const NON_EMPTY: u8 = 1 << 0;

    #[inline]
    pub const fn non_empty(self) -> bool {
        self.0 & Self::NON_EMPTY != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_non_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::NON_EMPTY } else { self.0 & !Self::NON_EMPTY })
    }
}

/// `list<T>`, `non-empty-list<T>`, `list{0: int, 1: string, ...}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListInfo {
    pub element_type: TypeId,
    pub known_elements: Option<KnownElementsId>,
    pub known_count: Option<NonZeroU32>,
    pub flags: ListFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ListFlags(u8);

impl ListFlags {
    const NON_EMPTY: u8 = 1 << 0;

    #[inline]
    pub const fn non_empty(self) -> bool {
        self.0 & Self::NON_EMPTY != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_non_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::NON_EMPTY } else { self.0 & !Self::NON_EMPTY })
    }
}

const _: () = assert!(size_of::<KeyedArrayInfo>() <= 32);
const _: () = assert!(size_of::<ListInfo>() <= 24);
const _: () = assert!(size_of::<ArrayKey>() <= 24);
const _: () = assert!(size_of::<KnownItemEntry>() <= 40);
const _: () = assert!(size_of::<KnownElementEntry>() <= 24);

impl std::fmt::Display for ArrayKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArrayKey::Int(n) => write!(f, "{n}"),
            ArrayKey::String(a) => write!(f, "'{}'", a.as_str()),
            ArrayKey::Const { class, name } => write!(f, "{}::{}", class.as_str(), name.as_str()),
        }
    }
}

impl std::fmt::Display for KeyedArrayInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i = crate::interner::interner();
        if let Some(known_id) = self.known_items {
            f.write_str("array{")?;
            let mut first = true;
            for entry in i.get_known_items(known_id) {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                std::fmt::Display::fmt(&entry.key, f)?;
                if entry.optional {
                    f.write_str("?")?;
                }
                f.write_str(": ")?;
                std::fmt::Display::fmt(&entry.value, f)?;
            }
            if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
                if !first {
                    f.write_str(", ")?;
                }
                write!(f, "...<{}, {}>", k, v)?;
            }
            f.write_str("}")
        } else if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
            let head = if self.flags.non_empty() { "non-empty-array" } else { "array" };
            write!(f, "{head}<{k}, {v}>")
        } else {
            f.write_str("array{}")
        }
    }
}

impl KeyedArrayInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}

impl std::fmt::Display for ListInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let i = crate::interner::interner();
        if let Some(known_id) = self.known_elements {
            f.write_str("list{")?;
            let mut first = true;
            for entry in i.get_known_elements(known_id) {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                write!(f, "{}", entry.index)?;
                if entry.optional {
                    f.write_str("?")?;
                }
                write!(f, ": {}", entry.value)?;
            }
            f.write_str("}")
        } else {
            let head = if self.flags.non_empty() { "non-empty-list" } else { "list" };
            write!(f, "{head}<{}>", self.element_type)
        }
    }
}

impl ListInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}
