use core::mem::size_of;

use mago_atom::Atom;

use crate::TypeId;

/// `class-string`, `interface-string`, `enum-string`, `trait-string`, and
/// their refined forms (`class-string<Foo>`, `class-string<T>`, the literal
/// `"App\\Foo"` typed as a class-string, …).
///
/// `kind` identifies which family of class-like name this represents; the
/// `specifier` carries the rest. The shared `kind` field is factored out so
/// the specifier enum stays tight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassLikeStringInfo {
    pub kind: ClassLikeKind,
    pub specifier: ClassLikeStringSpecifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum ClassLikeKind {
    Class,
    Interface,
    Enum,
    Trait,
}

/// What is known about the value of this class-like-string beyond its kind.
///
/// `Generic` carries just a constraint type. The constraint itself contains
/// a [`GenericParameterInfo`](crate::payload::GenericParameterInfo) element that
/// names the template parameter and its scope, so we don't repeat that
/// information here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ClassLikeStringSpecifier {
    Any,
    Literal { value: Atom },
    OfType { constraint: TypeId },
    Generic { constraint: TypeId },
}

const _: () = assert!(size_of::<ClassLikeStringSpecifier>() <= 16, "size budget exceeded");
const _: () = assert!(size_of::<ClassLikeStringInfo>() <= 24, "size budget exceeded");
const _: () = assert!(size_of::<ClassLikeKind>() == 1, "size budget exceeded");

impl ClassLikeKind {
    #[inline]
    #[must_use] 
    pub const fn as_str(self) -> &'static str {
        match self {
            ClassLikeKind::Class => "class-string",
            ClassLikeKind::Interface => "interface-string",
            ClassLikeKind::Enum => "enum-string",
            ClassLikeKind::Trait => "trait-string",
        }
    }
}

impl core::fmt::Display for ClassLikeStringInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match &self.specifier {
            ClassLikeStringSpecifier::Any => f.write_str(self.kind.as_str()),
            ClassLikeStringSpecifier::Literal { value } => write!(f, "class-string('{}')", value.as_str()),
            ClassLikeStringSpecifier::OfType { constraint } => {
                write!(f, "{}<{}>", self.kind.as_str(), constraint.as_ref())
            }
            ClassLikeStringSpecifier::Generic { constraint } => {
                write!(f, "{}<{}>", self.kind.as_str(), constraint.as_ref())
            }
        }
    }
}
