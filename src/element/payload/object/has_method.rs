use core::mem::size_of;

use mago_atom::Atom;

/// "Some object that has a method named `M`", produced by `method_exists`
/// narrowing.
///
/// Subtype of [`ObjectInfo`](super::ObjectInfo) `Any`; subtype of a
/// specific [`ObjectInfo`](super::ObjectInfo) iff the world confirms that
/// class has the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HasMethodInfo {
    pub method_name: Atom,
}

const _: () = assert!(size_of::<HasMethodInfo>() <= 16, "size budget exceeded");

impl core::fmt::Display for HasMethodInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "has-method<'{}'>", self.method_name.as_str())
    }
}
