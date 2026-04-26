use std::mem::size_of;

use mago_atom::Atom;

/// A PHP enum, optionally narrowed to a single case.
///
/// `case = None` denotes "any case of this enum"; `case = Some(name)` is a
/// single literal case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumInfo {
    pub name: Atom,
    pub case: Option<Atom>,
}

const _: () = assert!(size_of::<EnumInfo>() <= 24);
