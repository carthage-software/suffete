use std::mem::size_of;

use mago_atom::Atom;

/// A PHP enum, optionally narrowed to a single case.
///
/// `case = None` denotes "any case of this enum"; `case = Some(name)` is a
/// single literal case.
///
/// Enums are implicitly `final` in PHP: `enum E` admits no subclass.
/// `enum(E) & has-method<'render'>` therefore adds no information — if
/// `E` declares `render`, the world already knows; if it doesn't, the
/// intersection is uninhabited and collapses to `never`. So enums
/// intentionally carry no intersection slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EnumInfo {
    pub name: Atom,
    pub case: Option<Atom>,
}

const _: () = assert!(size_of::<EnumInfo>() <= 24);

impl std::fmt::Display for EnumInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.case {
            Some(case) => write!(f, "enum({}::{})", self.name.as_str(), case.as_str()),
            None => write!(f, "enum({})", self.name.as_str()),
        }
    }
}
