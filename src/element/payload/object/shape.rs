use std::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;
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
///
/// Carries an optional intersection list so structural narrowings can
/// chain (e.g. `object{a: int} & HasMethod(foo)`) without needing an
/// outer [`ObjectInfo`](super::ObjectInfo) wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectShapeInfo {
    pub known_properties: Option<KnownPropertiesId>,
    pub intersections: Option<ElementListId>,
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

const _: () = assert!(size_of::<ObjectShapeInfo>() <= 16);
const _: () = assert!(size_of::<KnownPropertyEntry>() <= 24);

impl std::fmt::Display for ObjectShapeInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                std::fmt::Display::fmt(&entry.value, f)?;
            }
        }
        if !self.flags.sealed() {
            if !first {
                f.write_str(", ")?;
            }
            f.write_str("...")?;
        }
        f.write_str("}")?;
        super::render_intersection_chain(self.intersections, f)
    }
}

impl ObjectShapeInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        // First-cut: identical to compact rendering. Multi-line indented
        // form for shapes with many properties is a future precision win.
        let _ = indent;
        self.to_string()
    }
}
