#![allow(clippy::arithmetic_side_effects)]

use core::mem::size_of;
use core::num::NonZeroU32;

use mago_atom::Atom;

use crate::ElementListId;
use crate::TypeId;
use crate::handle::define_handle;

define_handle! {
    /// Handle to an interned `&'static [KnownItemEntry]`:
    /// [`KeyedArrayInfo`] known items.
    KnownItemsId
}

define_handle! {
    /// Handle to an interned `&'static [KnownElementEntry]`:
    /// [`ListInfo`] known elements.
    KnownElementsId
}

/// A literal key in a keyed-array shape.
///
/// `Const` carries an unresolved `Class::CONSTANT` reference; resolution
/// happens during the population phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ArrayKey {
    Int(i64),
    String(Atom),
    Const { class: Atom, name: Atom },
}

/// One entry in a keyed-array shape's known items list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownItemEntry {
    pub key: ArrayKey,
    pub value: TypeId,
    pub optional: bool,
}

/// One entry in a list shape's known elements list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KnownElementEntry {
    pub index: u32,
    pub value: TypeId,
    pub optional: bool,
}

/// `array<K, V>`, `array{a: int, ...}`, `array{}`.
///
/// `key_param` / `value_param` describe the rest type when present.
/// `known_items` is a sorted, interned list of fixed entries.
///
/// "Sealed" is the absence of a rest type: `key_param` and `value_param` both
/// `None` means the shape admits no extra entries beyond `known_items`. There
/// is intentionally no separate `sealed` flag.
///
/// `intersections` carries `&conjunct` narrowings the way
/// [`ObjectInfo`](super::ObjectInfo) does. Subtract-driven complement
/// narrowing of the form "this array except the values of `array<K2,
/// V2>`" is expressed as a [`Negated`](crate::element::payload::NegatedInfo)
/// conjunct here ; refines/overlaps/meet walk the chain just as they
/// do for objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyedArrayInfo {
    pub key_param: Option<TypeId>,
    pub value_param: Option<TypeId>,
    pub known_items: Option<KnownItemsId>,
    pub intersections: Option<ElementListId>,
    pub flags: KeyedArrayFlags,
}

impl KeyedArrayInfo {
    /// `true` iff this shape admits no entries beyond its known items.
    #[inline]
    #[must_use]
    pub const fn is_sealed(&self) -> bool {
        self.key_param.is_none() && self.value_param.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct KeyedArrayFlags(u8);

impl KeyedArrayFlags {
    const NON_EMPTY: u8 = 1 << 0;

    #[inline]
    #[must_use]
    pub const fn non_empty(self) -> bool {
        self.0 & Self::NON_EMPTY != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_non_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::NON_EMPTY } else { self.0 & !Self::NON_EMPTY })
    }
}

/// `list<T>`, `non-empty-list<T>`, `list{0: int, 1: string, ...}`.
///
/// `intersections` carries `&conjunct` narrowings the way
/// [`ObjectInfo`](super::ObjectInfo) does. Subtract-driven complement
/// narrowing of the form "this list except the values of `list<S>`"
/// is expressed as a [`Negated`](crate::element::payload::NegatedInfo)
/// conjunct here ; refines/overlaps/meet walk the chain just as they
/// do for objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ListInfo {
    pub element_type: TypeId,
    pub known_elements: Option<KnownElementsId>,
    pub intersections: Option<ElementListId>,
    pub known_count: Option<NonZeroU32>,
    pub flags: ListFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ListFlags(u8);

impl ListFlags {
    const NON_EMPTY: u8 = 1 << 0;

    #[inline]
    #[must_use]
    pub const fn non_empty(self) -> bool {
        self.0 & Self::NON_EMPTY != 0
    }

    #[inline]
    #[must_use]
    pub const fn with_non_empty(self, on: bool) -> Self {
        Self(if on { self.0 | Self::NON_EMPTY } else { self.0 & !Self::NON_EMPTY })
    }
}

// Adding `intersections: Option<ElementListId>` (4 bytes) to both
// pushes them up by 8 bytes after alignment. The budgets reflect the
// new ceilings; both still fit within their natural cache lines.
const _: () = assert!(size_of::<KeyedArrayInfo>() <= 32, "size budget exceeded");
const _: () = assert!(size_of::<ListInfo>() <= 32, "size budget exceeded");
const _: () = assert!(size_of::<ArrayKey>() <= 24, "size budget exceeded");
const _: () = assert!(size_of::<KnownItemEntry>() <= 40, "size budget exceeded");
const _: () = assert!(size_of::<KnownElementEntry>() <= 24, "size budget exceeded");

impl core::fmt::Display for ArrayKey {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ArrayKey::Int(n) => write!(f, "{n}"),
            ArrayKey::String(a) => write!(f, "'{}'", a.as_str()),
            ArrayKey::Const { class, name } => write!(f, "{}::{}", class.as_str(), name.as_str()),
        }
    }
}

impl core::fmt::Display for KeyedArrayInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let i = crate::interner::interner();
        if let Some(known_id) = self.known_items {
            f.write_str("array{")?;
            let mut first = true;
            for entry in i.get_known_items(known_id) {
                if !first {
                    f.write_str(", ")?;
                }

                first = false;
                core::fmt::Display::fmt(&entry.key, f)?;
                if entry.optional {
                    f.write_str("?")?;
                }

                f.write_str(": ")?;
                core::fmt::Display::fmt(&entry.value, f)?;
            }

            if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
                if !first {
                    f.write_str(", ")?;
                }
                write!(f, "...<{}, {}>", k, v)?;
            }
            f.write_str("}")?;
        } else if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
            let head = if self.flags.non_empty() { "non-empty-array" } else { "array" };
            write!(f, "{head}<{k}, {v}>")?;
        } else {
            f.write_str("array{}")?;
        }

        super::object::render_intersection_chain(self.intersections, f)
    }
}

impl KeyedArrayInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        use crate::typed::Typed;
        let i = crate::interner::interner();

        if let Some(known_id) = self.known_items {
            let entries = i.get_known_items(known_id);
            if entries.is_empty() && self.key_param.is_none() && self.value_param.is_none() {
                return String::from("array{}");
            }

            let mut out = String::from("array{\n");
            let inner = indent + 2;
            let pad = " ".repeat(inner);
            for entry in entries {
                out.push_str(&pad);
                out.push_str(&entry.key.to_string());
                if entry.optional {
                    out.push('?');
                }
                out.push_str(": ");
                out.push_str(&entry.value.pretty_with_indent(inner));
                out.push_str(",\n");
            }
            if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
                out.push_str(&pad);
                out.push_str("...");
                if k.is_complex() || v.is_complex() {
                    let inner2 = inner + 2;
                    let pad2 = " ".repeat(inner2);
                    out.push_str("<\n");
                    out.push_str(&pad2);
                    out.push_str(&k.pretty_with_indent(inner2));
                    out.push_str(",\n");
                    out.push_str(&pad2);
                    out.push_str(&v.pretty_with_indent(inner2));
                    out.push_str(",\n");
                    out.push_str(&pad);
                    out.push('>');
                } else {
                    out.push('<');
                    out.push_str(&k.pretty_with_indent(inner));
                    out.push_str(", ");
                    out.push_str(&v.pretty_with_indent(inner));
                    out.push('>');
                }
                out.push_str(",\n");
            }
            out.push_str(&" ".repeat(indent));
            out.push('}');
            append_intersection_chain_pretty(self.intersections, indent, &mut out);
            return out;
        }

        let mut out = if let (Some(k), Some(v)) = (self.key_param, self.value_param) {
            let head = if self.flags.non_empty() { "non-empty-array" } else { "array" };
            if k.is_complex() || v.is_complex() {
                let inner = indent + 2;
                let pad = " ".repeat(inner);
                format!(
                    "{head}<\n{pad}{},\n{pad}{},\n{}>",
                    k.pretty_with_indent(inner),
                    v.pretty_with_indent(inner),
                    " ".repeat(indent),
                )
            } else {
                format!("{head}<{}, {}>", k.pretty_with_indent(indent), v.pretty_with_indent(indent))
            }
        } else {
            String::from("array{}")
        };
        append_intersection_chain_pretty(self.intersections, indent, &mut out);
        out
    }
}

impl core::fmt::Display for ListInfo {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let i = crate::interner::interner();
        if let Some(known_id) = self.known_elements {
            f.write_str("list{")?;
            let mut first = true;
            for entry in i.get_known_elements(known_id) {
                if !first {
                    f.write_str(", ")?;
                }
                first = false;
                write!(f, "{}", entry.index)?;
                if entry.optional {
                    f.write_str("?")?;
                }
                write!(f, ": {}", entry.value)?;
            }
            f.write_str("}")?;
        } else {
            let head = if self.flags.non_empty() { "non-empty-list" } else { "list" };
            write!(f, "{head}<{}>", self.element_type)?;
        }
        super::object::render_intersection_chain(self.intersections, f)
    }
}

impl ListInfo {
    #[inline]
    pub(crate) fn pretty_with_indent(&self, indent: usize) -> String {
        use crate::typed::Typed;
        let i = crate::interner::interner();

        if let Some(known_id) = self.known_elements {
            let entries = i.get_known_elements(known_id);
            if entries.is_empty() && self.element_type == crate::prelude::TYPE_NEVER {
                return String::from("list{}");
            }

            let mut out = String::from("list{\n");
            let inner = indent + 2;
            let pad = " ".repeat(inner);
            let has_optional = entries.iter().any(|e| e.optional);
            let include_index = entries.len() > 1 || has_optional;

            for entry in entries {
                out.push_str(&pad);
                if include_index {
                    out.push_str(&entry.index.to_string());
                    if entry.optional {
                        out.push('?');
                    }
                    out.push_str(": ");
                }
                out.push_str(&entry.value.pretty_with_indent(inner));
                out.push_str(",\n");
            }
            if self.element_type != crate::prelude::TYPE_NEVER {
                out.push_str(&pad);
                out.push_str("...");
                if self.element_type.is_complex() {
                    let inner2 = inner + 2;
                    out.push_str("<\n");
                    out.push_str(&" ".repeat(inner2));
                    out.push_str(&self.element_type.pretty_with_indent(inner2));
                    out.push_str(",\n");
                    out.push_str(&pad);
                    out.push('>');
                } else {
                    out.push('<');
                    out.push_str(&self.element_type.pretty_with_indent(inner));
                    out.push('>');
                }
                out.push_str(",\n");
            }
            out.push_str(&" ".repeat(indent));
            out.push('}');
            append_intersection_chain_pretty(self.intersections, indent, &mut out);
            return out;
        }

        let head = if self.flags.non_empty() { "non-empty-list" } else { "list" };
        let mut out = if self.element_type.is_complex() {
            let inner = indent + 2;
            format!(
                "{head}<\n{}{},\n{}>",
                " ".repeat(inner),
                self.element_type.pretty_with_indent(inner),
                " ".repeat(indent),
            )
        } else {
            format!("{head}<{}>", self.element_type.pretty_with_indent(indent))
        };
        append_intersection_chain_pretty(self.intersections, indent, &mut out);
        out
    }
}

/// Pretty-form companion to [`super::object::render_intersection_chain`]:
/// append the `&conjunct` chain from `intersections` to `out`, wrapping
/// each conjunct in `()` when it itself carries intersection types.
#[inline]
fn append_intersection_chain_pretty(intersections: Option<ElementListId>, indent: usize, out: &mut String) {
    use crate::typed::Typed;
    let Some(id) = intersections else { return };
    for &conjunct in crate::interner::interner().get_element_list(id) {
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
