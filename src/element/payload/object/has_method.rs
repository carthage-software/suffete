use std::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;

/// "Some object that has a method named `M`", produced by `method_exists`
/// narrowing. Subtype of [`ObjectInfo`](super::ObjectInfo) `Any`; subtype of a
/// specific [`ObjectInfo`](super::ObjectInfo) iff the codebase confirms that
/// class has the method.
///
/// Carries an optional intersection list so structural narrowings can chain
/// (`HasMethod(foo) & HasMethod(bar)`) without needing an outer
/// [`ObjectInfo`](super::ObjectInfo) wrapper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HasMethodInfo {
    pub method_name: Atom,
    pub intersections: Option<ElementListId>,
}

const _: () = assert!(size_of::<HasMethodInfo>() <= 16);

impl std::fmt::Display for HasMethodInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "has-method<'{}'>", self.method_name.as_str())?;
        super::render_intersection_chain(self.intersections, f)
    }
}
