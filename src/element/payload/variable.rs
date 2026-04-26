use std::mem::size_of;

use mago_atom::Atom;

/// A binding-scope template variable (`${T}` in PHPDoc) used during template
/// inference.
///
/// `Variable` atoms are local to a single inference call and do not survive
/// into stored types: once inference completes, every `Variable(T)` is
/// substituted with the inferred type for `T`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VariableInfo {
    pub name: Atom,
}

const _: () = assert!(size_of::<VariableInfo>() == 8);
