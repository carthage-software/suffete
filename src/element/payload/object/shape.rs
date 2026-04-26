use std::mem::size_of;

use mago_atom::Atom;

use crate::TypeId;
use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned `&'static [KnownPropertyEntry]`:
    /// [`ObjectShapeInfo`] known properties.
    KnownPropertiesId
}

/// `object{a: int, b?: string}`, optionally sealed.
///
/// Unlike keyed-array sealing (which is encoded by absence of a rest type),
/// object shapes have no rest type at all, so sealing is a real flag because
/// `object{a: int}` and `object{a: int, ...}` are both expressible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectShapeInfo {
    pub known_properties: Option<KnownPropertiesId>,
    pub flags: ObjectShapeFlags,
}

/// One entry in an object shape's known-properties list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownPropertyEntry {
    pub name: Atom,
    pub value: TypeId,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObjectShapeFlags(u8);

impl ObjectShapeFlags {
    const SEALED: u8 = 1 << 0;

    #[inline]
    pub const fn sealed(self) -> bool {
        self.0 & Self::SEALED != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_sealed(self, on: bool) -> Self {
        Self(if on { self.0 | Self::SEALED } else { self.0 & !Self::SEALED })
    }
}

const _: () = assert!(size_of::<ObjectShapeInfo>() <= 8);
const _: () = assert!(size_of::<KnownPropertyEntry>() <= 16);
