use core::mem::size_of;

use mago_atom::Atom;

use super::DefiningEntityId;
use crate::TypeId;

/// A reference to a `@template T` parameter that is *in scope*: the analyzer
/// is inside the class or function that declares `T`.
///
/// `T` denotes the same value-set across all uses of `T` in the same scope
/// (relational identity); this is what makes a template parameter different
/// from `mixed`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenericParameterInfo {
    pub name: Atom,
    pub defining_entity: DefiningEntityId,
    pub constraint: TypeId,
}

const _: () = assert!(size_of::<GenericParameterInfo>() <= 24, "size budget exceeded");

impl core::fmt::Display for GenericParameterInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let entity = crate::interner::interner().get_defining_entity(self.defining_entity);
        write!(f, "'{}.{} extends {}", self.name.as_str(), entity, self.constraint)
    }
}
