use std::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;
use crate::TypeListId;

/// `Foo`, `Foo<int>`, `Foo<int>&Bar`: an unresolved class-like name with
/// optional type arguments and intersection partners.
///
/// Payload of `ElementKind::Reference`. By far the most common indirection
/// element, which is why it has its own per-kind arena rather than sharing
/// one with the rarer member/global references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolReference {
    pub name: Atom,
    pub type_args: Option<TypeListId>,
    pub intersections: Option<ElementListId>,
}

/// `Foo::CONST`, `Foo::*`, `Foo::PREFIX_*`: a class-like constant reference.
///
/// Payload of `ElementKind::MemberReference`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberReference {
    pub class_like_name: Atom,
    pub selector: NameSelector,
}

/// A reference to a global constant, optionally via wildcard selector.
///
/// Payload of `ElementKind::GlobalReference`. Exists as a newtype rather than
/// reusing [`NameSelector`] directly so the per-kind arena's element type is
/// nominally distinct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlobalReference {
    pub selector: NameSelector,
}

/// How a member or global reference picks one or more matching names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NameSelector {
    Identifier(Atom),
    StartsWith(Atom),
    EndsWith(Atom),
    Contains(Atom),
    Wildcard,
}

const _: () = assert!(size_of::<SymbolReference>() <= 24);
const _: () = assert!(size_of::<MemberReference>() <= 24);
const _: () = assert!(size_of::<GlobalReference>() <= 16);
const _: () = assert!(size_of::<NameSelector>() <= 16);

impl std::fmt::Display for NameSelector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NameSelector::Identifier(a) => f.write_str(a.as_str()),
            NameSelector::StartsWith(a) => write!(f, "{}*", a.as_str()),
            NameSelector::EndsWith(a) => write!(f, "*{}", a.as_str()),
            NameSelector::Contains(a) => write!(f, "*{}*", a.as_str()),
            NameSelector::Wildcard => f.write_str("*"),
        }
    }
}

impl std::fmt::Display for SymbolReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())?;
        if let Some(args_id) = self.type_args {
            let i = crate::interner::interner();
            f.write_str("<")?;
            for (idx, &arg) in i.get_type_list(args_id).iter().enumerate() {
                if idx > 0 {
                    f.write_str(", ")?;
                }
                std::fmt::Display::fmt(&arg, f)?;
            }
            f.write_str(">")?;
        }
        super::object::render_intersection_chain(self.intersections, f)
    }
}

impl SymbolReference {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}

impl std::fmt::Display for MemberReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}::{}", self.class_like_name.as_str(), self.selector)
    }
}

impl std::fmt::Display for GlobalReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.selector, f)
    }
}
