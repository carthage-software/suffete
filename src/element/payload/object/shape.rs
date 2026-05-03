#![allow(clippy::arithmetic_side_effects)]

use core::mem::size_of;

use mago_atom::Atom;

use crate::TypeId;
use crate::handle::define_handle;
use crate::typed::Typed;

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
    #[must_use]
    pub const fn sealed(self) -> bool {
        self.0 & Self::SEALED != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_sealed(self, on: bool) -> Self {
        Self(if on { self.0 | Self::SEALED } else { self.0 & !Self::SEALED })
    }
}

const _: () = assert!(size_of::<ObjectShapeInfo>() <= 16, "size budget exceeded");
const _: () = assert!(size_of::<KnownPropertyEntry>() <= 24, "size budget exceeded");

impl core::fmt::Display for ObjectShapeInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let i = crate::interner::interner();
        f.write_str("object{")?;
        let mut first = true;
        if let Some(known_id) = self.known_properties {
            for entry in i.get_known_properties(known_id) {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                f.write_str(entry.name.as_str())?;
                if entry.optional {
                    f.write_str("?")?;
                }
                f.write_str(": ")?;
                core::fmt::Display::fmt(&entry.value, f)?;
            }
        }
        if !self.flags.sealed() {
            if !first {
                f.write_str(", ")?;
            }
            f.write_str("...")?;
        }
        f.write_str("}")
    }
}

impl ObjectShapeInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(self, indent: usize) -> String {
        let i = crate::interner::interner();
        let entries = self.known_properties.map_or(&[] as &[KnownPropertyEntry], |id| i.get_known_properties(id));
        if entries.is_empty() {
            return if self.flags.sealed() { String::from("object{}") } else { String::from("object{...}") };
        }

        let mut out = String::from("object{\n");
        let inner = indent + 2;
        let pad = " ".repeat(inner);
        for entry in entries {
            out.push_str(&pad);
            out.push_str(entry.name.as_str());
            if entry.optional {
                out.push('?');
            }

            out.push_str(": ");
            out.push_str(&entry.value.pretty_with_indent(inner));
            out.push_str(",\n");
        }

        if !self.flags.sealed() {
            out.push_str(&pad);
            out.push_str("...,\n");
        }

        out.push_str(&" ".repeat(indent));
        out.push('}');
        out
    }
}
