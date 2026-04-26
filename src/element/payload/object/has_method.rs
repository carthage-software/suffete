use std::mem::size_of;

use mago_atom::Atom;

/// "Some object that has a method named `M`", produced by `method_exists`
/// narrowing. Subtype of [`ObjectInfo`](super::ObjectInfo) `Any`; subtype of a
/// specific [`ObjectInfo`](super::ObjectInfo) iff the codebase confirms that
/// class has the method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HasMethodInfo {
    pub method_name: Atom,
}

const _: () = assert!(size_of::<HasMethodInfo>() == 8);
