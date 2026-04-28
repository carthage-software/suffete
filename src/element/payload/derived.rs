use std::mem::size_of;

use crate::TypeId;
use crate::TypeListId;

/// Type-level functions over other types: deferred computations that, given
/// the codebase, produce a concrete type.
///
/// Each variant is its own deferred operation. Largest payload here is
/// [`Self::TemplateType`] at three [`TypeId`]s (12 bytes), so the whole enum
/// lands at 16 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DerivedInfo {
    /// `key-of<T>`: the key type of an array-like or iterable `T`.
    KeyOf(TypeId),

    /// `value-of<T>`: the value type. For `BackedEnum` subclasses, the
    /// backing values.
    ValueOf(TypeId),

    /// `properties-of<T>`, `public-properties-of<T>`, etc.: for each
    /// property of class `T` (filtered by `visibility`), produce
    /// `array{prop_name: prop_type}`.
    PropertiesOf { target: TypeId, visibility: Option<Visibility> },

    /// `T[K]`: element access. For `array{a: int}`, `T['a']` resolves to `int`.
    IndexAccess { target: TypeId, index: TypeId },

    /// `int-mask<A::FLAG_FOO, A::FLAG_BAR>`: the set of integers formable by
    /// bitwise-OR-ing some subset of the listed literal-int values.
    IntMask(TypeListId),

    /// `int-mask-of<A::FLAG_*>`: `IntMask` over all members of a constant-
    /// wildcard family.
    IntMaskOf(TypeId),

    /// `template-type<$object, ClassName, T>`: given a value `$object` of
    /// some specialized class, look up the bound type for template `T` of
    /// `ClassName`.
    TemplateType { object: TypeId, class_name: TypeId, template_name: TypeId },

    /// `new<T>`: if `T` is `class-string<Foo>` or a literal class-string,
    /// produce `Foo` (the instance type).
    New(TypeId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum Visibility {
    Public,
    Protected,
    Private,
}

const _: () = assert!(size_of::<DerivedInfo>() <= 32);
const _: () = assert!(size_of::<Visibility>() == 1);

impl Visibility {
    pub const fn as_str(self) -> &'static str {
        match self {
            Visibility::Public => "public",
            Visibility::Protected => "protected",
            Visibility::Private => "private",
        }
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::fmt::Display for DerivedInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DerivedInfo::KeyOf(t) => write!(f, "key-of<{t}>"),
            DerivedInfo::ValueOf(t) => write!(f, "value-of<{t}>"),
            DerivedInfo::PropertiesOf { target, visibility } => match visibility {
                Some(v) => write!(f, "{}-properties-of<{target}>", v.as_str()),
                None => write!(f, "properties-of<{target}>"),
            },
            DerivedInfo::IndexAccess { target, index } => write!(f, "{target}[{index}]"),
            DerivedInfo::IntMask(list_id) => {
                f.write_str("int-mask<")?;
                let i = crate::interner::interner();
                for (idx, &t) in i.get_type_list(*list_id).iter().enumerate() {
                    if idx > 0 {
                        f.write_str(", ")?;
                    }
                    std::fmt::Display::fmt(&t, f)?;
                }
                f.write_str(">")
            }
            DerivedInfo::IntMaskOf(t) => write!(f, "int-mask-of<{t}>"),
            DerivedInfo::TemplateType { object, class_name, template_name } => {
                write!(f, "template-type<{object}, {class_name}, {template_name}>")
            }
            DerivedInfo::New(t) => write!(f, "new<{t}>"),
        }
    }
}

impl DerivedInfo {
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        let _ = indent;
        self.to_string()
    }
}
