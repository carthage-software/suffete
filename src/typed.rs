//! The [`Typed`] trait + dispatch enums [`View`] / [`Handle`].
//!
//! Implementations of [`Display`] and [`Typed`] for each concrete type
//! ([`Type`], [`TypeId`], [`Element`], [`ElementId`]) and for each
//! payload struct (`IntInfo`, `ObjectInfo`, etc.) live alongside the
//! type they're for, **not** here. This module owns only the trait
//! contract and the two dispatch enums.

use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result as FmtResult;

use crate::Element;
use crate::ElementId;
use crate::Type;
use crate::TypeId;

/// Common surface for everything in the type system: compact rendering
/// (via the supertrait [`Display`]), pretty multi-line rendering, and
/// generic intersection-conjunct access.
///
/// No method has a default body — every implementor states an answer
/// for every method, so adding a method to the trait is a compile-time
/// fail-loud across the crate.
pub trait Typed: Display {
    /// Multi-line, indented rendering with the given starting indent.
    /// May reduce to single-line for trivial inputs; the contract is
    /// "may break across lines", not "always does".
    fn pretty_with_indent(&self, indent: usize) -> String;

    /// Convenience: pretty rendering at indent zero.
    fn pretty(&self) -> String {
        self.pretty_with_indent(0)
    }

    /// `&` conjuncts this thing intersects with. Empty slice when none.
    fn intersection_types(&self) -> &'static [ElementId];

    /// `true` iff at least one intersection conjunct is present.
    fn has_intersection_types(&self) -> bool;

    /// `true` iff this kind of value supports intersections at all
    /// (regardless of whether the current instance has any).
    fn can_be_intersected(&self) -> bool;

    /// `true` iff the rendered form is large enough to benefit from
    /// multiline pretty formatting when used as a generic parameter.
    fn is_complex(&self) -> bool;
}

/// Borrowed view: either a [`Type`] or an [`Element`], borrowed from
/// the interner.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum View<'a> {
    Type(&'a Type),
    Element(&'a Element),
}

/// Owned handle: either a [`TypeId`] or an [`ElementId`]. Both are
/// `Copy`, so `Handle` is too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Handle {
    Type(TypeId),
    Element(ElementId),
}

impl<'a> From<&'a Type> for View<'a> {
    fn from(t: &'a Type) -> Self {
        View::Type(t)
    }
}

impl<'a> From<&'a Element> for View<'a> {
    fn from(e: &'a Element) -> Self {
        View::Element(e)
    }
}

impl From<TypeId> for Handle {
    fn from(id: TypeId) -> Self {
        Handle::Type(id)
    }
}

impl From<ElementId> for Handle {
    fn from(id: ElementId) -> Self {
        Handle::Element(id)
    }
}

impl Display for View<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            View::Type(t) => Display::fmt(t, f),
            View::Element(e) => Display::fmt(e, f),
        }
    }
}

impl Display for Handle {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Handle::Type(t) => Display::fmt(t, f),
            Handle::Element(e) => Display::fmt(e, f),
        }
    }
}

impl Typed for View<'_> {
    fn pretty_with_indent(&self, indent: usize) -> String {
        match self {
            View::Type(t) => Typed::pretty_with_indent(*t, indent),
            View::Element(e) => Typed::pretty_with_indent(*e, indent),
        }
    }

    fn intersection_types(&self) -> &'static [ElementId] {
        match self {
            View::Type(t) => Typed::intersection_types(*t),
            View::Element(e) => Typed::intersection_types(*e),
        }
    }

    fn has_intersection_types(&self) -> bool {
        match self {
            View::Type(t) => Typed::has_intersection_types(*t),
            View::Element(e) => Typed::has_intersection_types(*e),
        }
    }

    fn can_be_intersected(&self) -> bool {
        match self {
            View::Type(t) => Typed::can_be_intersected(*t),
            View::Element(e) => Typed::can_be_intersected(*e),
        }
    }

    fn is_complex(&self) -> bool {
        match self {
            View::Type(t) => Typed::is_complex(*t),
            View::Element(e) => Typed::is_complex(*e),
        }
    }
}

impl Typed for Handle {
    fn pretty_with_indent(&self, indent: usize) -> String {
        match self {
            Handle::Type(t) => Typed::pretty_with_indent(t, indent),
            Handle::Element(e) => Typed::pretty_with_indent(e, indent),
        }
    }

    fn intersection_types(&self) -> &'static [ElementId] {
        match self {
            Handle::Type(t) => Typed::intersection_types(t),
            Handle::Element(e) => Typed::intersection_types(e),
        }
    }

    fn has_intersection_types(&self) -> bool {
        match self {
            Handle::Type(t) => Typed::has_intersection_types(t),
            Handle::Element(e) => Typed::has_intersection_types(e),
        }
    }

    fn can_be_intersected(&self) -> bool {
        match self {
            Handle::Type(t) => Typed::can_be_intersected(t),
            Handle::Element(e) => Typed::can_be_intersected(e),
        }
    }

    fn is_complex(&self) -> bool {
        match self {
            Handle::Type(t) => Typed::is_complex(t),
            Handle::Element(e) => Typed::is_complex(e),
        }
    }
}
