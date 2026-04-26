use std::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;
use crate::TypeListId;

/// `Foo`, `Foo<int>`, `Foo&Bar`, `static`, `$this`.
///
/// `type_args` and `intersections` are interned slice handles so two object
/// elements with the same nominal class + same generic args share storage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectInfo {
    pub name: Atom,
    pub type_args: Option<TypeListId>,
    pub intersections: Option<ElementListId>,
    pub flags: ObjectFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObjectFlags(u8);

impl ObjectFlags {
    const IS_STATIC: u8 = 1 << 0;
    const IS_THIS: u8 = 1 << 1;
    const REMAPPED_PARAMETERS: u8 = 1 << 2;

    #[inline]
    pub const fn is_static(self) -> bool {
        self.0 & Self::IS_STATIC != 0
    }

    #[inline]
    pub const fn is_this(self) -> bool {
        self.0 & Self::IS_THIS != 0
    }

    #[inline]
    pub const fn remapped_parameters(self) -> bool {
        self.0 & Self::REMAPPED_PARAMETERS != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_is_static(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_STATIC } else { self.0 & !Self::IS_STATIC })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_this(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_THIS } else { self.0 & !Self::IS_THIS })
    }

    #[inline]
    #[must_use]
    pub const fn with_remapped_parameters(self, on: bool) -> Self {
        Self(if on { self.0 | Self::REMAPPED_PARAMETERS } else { self.0 & !Self::REMAPPED_PARAMETERS })
    }
}

// `mago_atom::Atom` is 8 bytes (it wraps `ustr::Ustr`, a thin pointer), so
// `ObjectInfo` aligns to 8 and lands at 24 bytes total. That's our budget.
const _: () = assert!(size_of::<ObjectInfo>() <= 24);
