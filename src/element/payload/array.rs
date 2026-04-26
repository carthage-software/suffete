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

const _: () = assert!(size_of::<KeyedArrayInfo>() <= 16);
const _: () = assert!(size_of::<ListInfo>() <= 16);
const _: () = assert!(size_of::<ArrayKey>() <= 24);
const _: () = assert!(size_of::<KnownItemEntry>() <= 32);
const _: () = assert!(size_of::<KnownElementEntry>() <= 16);
