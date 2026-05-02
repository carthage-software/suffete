use core::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;

/// "Some object that has a property named `P`", produced by `property_exists`
/// narrowing. Symmetric to [`HasMethodInfo`](super::HasMethodInfo).
///
/// Carries an optional intersection list so structural narrowings can chain
/// without needing an outer [`ObjectInfo`](super::ObjectInfo) wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HasPropertyInfo {
    pub property_name: Atom,
    pub intersections: Option<ElementListId>,
}

const _: () = assert!(size_of::<HasPropertyInfo>() <= 16, "size budget exceeded");

impl core::fmt::Display for HasPropertyInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "has-property<'{}'>", self.property_name.as_str())?;
        super::render_intersection_chain(self.intersections, f)
    }
}
