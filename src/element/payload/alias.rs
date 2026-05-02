use core::mem::size_of;

use mago_atom::Atom;

/// A user-defined `@type` alias: `class_name :: alias_name`.
///
/// Resolves through the world (Γ) to the alias body. Two aliases defined
/// identically in different classes are denotationally equivalent; the source
/// names are retained only for diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AliasInfo {
    pub class_name: Atom,
    pub alias_name: Atom,
}

const _: () = assert!(size_of::<AliasInfo>() <= 16, "size budget exceeded");

impl core::fmt::Display for AliasInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}::{}", self.class_name.as_str(), self.alias_name.as_str())
    }
}
