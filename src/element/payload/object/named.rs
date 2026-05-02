#![allow(clippy::arithmetic_side_effects)]

use core::mem::size_of;

use mago_atom::Atom;

use crate::ElementListId;
use crate::TypeListId;

/// `Foo`, `Foo<int>`, `Foo&Bar`, `static`, `$this`.
///
/// `type_args` and `intersections` are interned slice handles so two
/// object elements with the same nominal class + same generic args
/// share storage. Post-subtract narrowing of the form "Foo except
/// instances of D" is expressed as a `Negated(D)` conjunct inside
/// `intersections`, not via a dedicated `excluded` field ; see
/// [`crate::element::payload::NegatedInfo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ObjectInfo {
    pub name: Atom,
    pub type_args: Option<TypeListId>,
    pub intersections: Option<ElementListId>,
    pub flags: ObjectFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ObjectFlags(u8);

impl ObjectFlags {
    const IS_STATIC: u8 = 1 << 0;
    const IS_THIS: u8 = 1 << 1;
    const REMAPPED_PARAMETERS: u8 = 1 << 2;

    #[inline]
    #[must_use] 
    pub const fn is_static(self) -> bool {
        self.0 & Self::IS_STATIC != 0
    }

    #[inline]
    #[must_use] 
    pub const fn is_this(self) -> bool {
        self.0 & Self::IS_THIS != 0
    }

    #[inline]
    #[must_use] 
    pub const fn remapped_parameters(self) -> bool {
        self.0 & Self::REMAPPED_PARAMETERS != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_is_static(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_STATIC } else { self.0 & !Self::IS_STATIC })
    }

    #[inline]
    #[must_use]
    pub const fn with_is_this(self, on: bool) -> Self {
        Self(if on { self.0 | Self::IS_THIS } else { self.0 & !Self::IS_THIS })
    }

    #[inline]
    #[must_use]
    pub const fn with_remapped_parameters(self, on: bool) -> Self {
        Self(if on { self.0 | Self::REMAPPED_PARAMETERS } else { self.0 & !Self::REMAPPED_PARAMETERS })
    }
}

// `Atom` is 8 bytes (it wraps `ustr::Ustr`, a thin pointer), so
// `ObjectInfo` aligns to 8 and lands at 24 bytes total. That's our budget.
const _: () = assert!(size_of::<ObjectInfo>() <= 24, "size budget exceeded");

impl core::fmt::Display for ObjectInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.flags.is_this() {
            f.write_str("$this(")?;
        }
        f.write_str(self.name.as_str())?;
        if let Some(args_id) = self.type_args {
            f.write_str("<")?;
            let i = crate::interner::interner();
            for (idx, &arg) in i.get_type_list(args_id).iter().enumerate() {
                if idx > 0 {
                    f.write_str(", ")?;
                }
                core::fmt::Display::fmt(&arg, f)?;
            }
            f.write_str(">")?;
        }
        super::render_intersection_chain(self.intersections, f)?;
        #[allow(clippy::else_if_without_else)]
        if self.flags.is_this() {
            f.write_str(")")?;
        } else if self.flags.is_static() {
            f.write_str("&static")?;
        }
        Ok(())
    }
}

impl ObjectInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        use crate::typed::Typed;
        let i = crate::interner::interner();
        let mut out = String::new();
        if self.flags.is_this() {
            out.push_str("$this(");
        }
        out.push_str(self.name.as_str());
        if let Some(args_id) = self.type_args {
            let args = i.get_type_list(args_id);
            let any_complex = args.iter().any(crate::typed::Typed::is_complex);
            if any_complex {
                let inner_indent = indent + 2;
                let inner_pad = " ".repeat(inner_indent);
                out.push_str("<\n");
                for (idx, &arg) in args.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(",\n");
                    }
                    out.push_str(&inner_pad);
                    out.push_str(&arg.pretty_with_indent(inner_indent));
                }
                out.push_str(",\n");
                out.push_str(&" ".repeat(indent));
                out.push('>');
            } else {
                out.push('<');
                for (idx, &arg) in args.iter().enumerate() {
                    if idx > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&arg.pretty_with_indent(indent));
                }
                out.push('>');
            }
        }
        // Intersections rendered the same as compact for now.
        if let Some(id) = self.intersections {
            for &conjunct in i.get_element_list(id) {
                let s = conjunct.pretty_with_indent(indent);
                if conjunct.has_intersection_types() {
                    out.push_str("&(");
                    out.push_str(&s);
                    out.push(')');
                } else {
                    out.push('&');
                    out.push_str(&s);
                }
            }
        }
        #[allow(clippy::else_if_without_else)]
        if self.flags.is_this() {
            out.push(')');
        } else if self.flags.is_static() {
            out.push_str("&static");
        }
        out
    }
}
