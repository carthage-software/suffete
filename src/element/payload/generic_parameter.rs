use std::mem::size_of;

use mago_atom::Atom;

use super::DefiningEntityId;
use crate::ElementListId;
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
    pub intersections: Option<ElementListId>,
}

const _: () = assert!(size_of::<GenericParameterInfo>() <= 24);

impl std::fmt::Display for GenericParameterInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let entity = crate::interner::interner().get_defining_entity(self.defining_entity);
        let base = format!("'{}.{} extends {}", self.name.as_str(), entity, self.constraint);
        if let Some(id) = self.intersections {
            f.write_str("(")?;
            f.write_str(&base)?;
            f.write_str(")")?;
            for &conjunct in crate::interner::interner().get_element_list(id) {
                let s = conjunct.to_string();
                if conjunct.has_intersection_types() {
                    write!(f, "&({s})")?;
                } else {
                    write!(f, "&{s}")?;
                }
            }
            Ok(())
        } else {
            f.write_str(&base)
        }
    }
}
