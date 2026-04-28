use std::mem::size_of;

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

const _: () = assert!(size_of::<AliasInfo>() <= 16);

impl std::fmt::Display for AliasInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.class_name.as_str(), self.alias_name.as_str())
    }
}
