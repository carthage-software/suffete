use core::fmt::Display;
use core::fmt::Formatter;
use core::fmt::Result as FmtResult;

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::AliasInfo;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DerivedInfo;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::GlobalReference;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::element::payload::IntersectedInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::ListInfo;
use crate::element::payload::MemberReference;
use crate::element::payload::MixedInfo;
use crate::element::payload::NegatedInfo;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeInfo;
use crate::element::payload::ResourceInfo;
use crate::element::payload::SymbolReference;
use crate::element::payload::VariableInfo;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::StringInfo;
use crate::typed::Typed;

/// A borrowed view into an [`ElementId`](crate::ElementId)'s payload.
///
/// Returned by [`ElementId::view`](crate::ElementId::view). Trivial-kind
/// variants (no payload) are unit-like; payload-bearing variants carry a
/// `&'static` reference into the matching per-kind arena.
///
/// `Element` is `Copy` (every variant is either tag-only or a thin pointer),
/// so consumers can match-then-discard freely.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Element {
    Null,
    Never,
    Void,
    Placeholder,
    Bool,
    True,
    False,
    Scalar,
    Numeric,
    ArrayKey,
    ObjectAny,
    Mixed(&'static MixedInfo),
    Int(&'static IntInfo),
    Float(&'static FloatInfo),
    String(&'static StringInfo),
    ClassLikeString(&'static ClassLikeStringInfo),
    Object(&'static ObjectInfo),
    Enum(&'static EnumInfo),
    ObjectShape(&'static ObjectShapeInfo),
    HasMethod(&'static HasMethodInfo),
    HasProperty(&'static HasPropertyInfo),
    Array(&'static KeyedArrayInfo),
    List(&'static ListInfo),
    Iterable(&'static IterableInfo),
    Callable(&'static CallableInfo),
    Resource(&'static ResourceInfo),
    GenericParameter(&'static GenericParameterInfo),
    Variable(&'static VariableInfo),
    Reference(&'static SymbolReference),
    MemberReference(&'static MemberReference),
    GlobalReference(&'static GlobalReference),
    Alias(&'static AliasInfo),
    Conditional(&'static ConditionalInfo),
    Derived(&'static DerivedInfo),
    Negated(&'static NegatedInfo),
    Intersected(&'static IntersectedInfo),
}

impl Element {
    #[inline]
    #[must_use]
    pub const fn kind(&self) -> ElementKind {
        match self {
            Element::Null => ElementKind::Null,
            Element::Never => ElementKind::Never,
            Element::Void => ElementKind::Void,
            Element::Placeholder => ElementKind::Placeholder,
            Element::Bool => ElementKind::Bool,
            Element::True => ElementKind::True,
            Element::False => ElementKind::False,
            Element::Scalar => ElementKind::Scalar,
            Element::Numeric => ElementKind::Numeric,
            Element::ArrayKey => ElementKind::ArrayKey,
            Element::ObjectAny => ElementKind::ObjectAny,
            Element::Mixed(_) => ElementKind::Mixed,
            Element::Int(_) => ElementKind::Int,
            Element::Float(_) => ElementKind::Float,
            Element::String(_) => ElementKind::String,
            Element::ClassLikeString(_) => ElementKind::ClassLikeString,
            Element::Object(_) => ElementKind::Object,
            Element::Enum(_) => ElementKind::Enum,
            Element::ObjectShape(_) => ElementKind::ObjectShape,
            Element::HasMethod(_) => ElementKind::HasMethod,
            Element::HasProperty(_) => ElementKind::HasProperty,
            Element::Array(_) => ElementKind::Array,
            Element::List(_) => ElementKind::List,
            Element::Iterable(_) => ElementKind::Iterable,
            Element::Callable(_) => ElementKind::Callable,
            Element::Resource(_) => ElementKind::Resource,
            Element::GenericParameter(_) => ElementKind::GenericParameter,
            Element::Variable(_) => ElementKind::Variable,
            Element::Reference(_) => ElementKind::Reference,
            Element::MemberReference(_) => ElementKind::MemberReference,
            Element::GlobalReference(_) => ElementKind::GlobalReference,
            Element::Alias(_) => ElementKind::Alias,
            Element::Conditional(_) => ElementKind::Conditional,
            Element::Derived(_) => ElementKind::Derived,
            Element::Negated(_) => ElementKind::Negated,
            Element::Intersected(_) => ElementKind::Intersected,
        }
    }
}

impl Display for Element {
    #[inline]
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Element::Null => f.write_str("null"),
            Element::Never => f.write_str("never"),
            Element::Void => f.write_str("void"),
            Element::Placeholder => f.write_str("_"),
            Element::Bool => f.write_str("bool"),
            Element::True => f.write_str("true"),
            Element::False => f.write_str("false"),
            Element::Scalar => f.write_str("scalar"),
            Element::Numeric => f.write_str("numeric"),
            Element::ArrayKey => f.write_str("array-key"),
            Element::ObjectAny => f.write_str("object"),
            Element::Mixed(info) => Display::fmt(*info, f),
            Element::Int(info) => Display::fmt(*info, f),
            Element::Float(info) => Display::fmt(*info, f),
            Element::String(info) => Display::fmt(*info, f),
            Element::ClassLikeString(info) => Display::fmt(*info, f),
            Element::Object(info) => Display::fmt(*info, f),
            Element::Enum(info) => Display::fmt(*info, f),
            Element::ObjectShape(info) => Display::fmt(*info, f),
            Element::HasMethod(info) => Display::fmt(*info, f),
            Element::HasProperty(info) => Display::fmt(*info, f),
            Element::Array(info) => Display::fmt(*info, f),
            Element::List(info) => Display::fmt(*info, f),
            Element::Iterable(info) => Display::fmt(*info, f),
            Element::Callable(info) => Display::fmt(*info, f),
            Element::Resource(info) => Display::fmt(*info, f),
            Element::GenericParameter(info) => Display::fmt(*info, f),
            Element::Variable(info) => Display::fmt(*info, f),
            Element::Reference(info) => Display::fmt(*info, f),
            Element::MemberReference(info) => Display::fmt(*info, f),
            Element::GlobalReference(info) => Display::fmt(*info, f),
            Element::Alias(info) => Display::fmt(*info, f),
            Element::Conditional(info) => Display::fmt(*info, f),
            Element::Derived(info) => Display::fmt(*info, f),
            Element::Negated(info) => Display::fmt(*info, f),
            Element::Intersected(info) => Display::fmt(*info, f),
        }
    }
}

impl Typed for Element {
    #[inline]
    fn pretty(&self) -> String {
        self.pretty_with_indent(0)
    }

    #[inline]
    fn pretty_with_indent(&self, indent: usize) -> String {
        match self {
            Element::Object(info) => info.pretty_with_indent(indent),
            Element::ObjectShape(info) => info.pretty_with_indent(indent),
            Element::Array(info) => info.pretty_with_indent(indent),
            Element::List(info) => info.pretty_with_indent(indent),
            Element::Iterable(info) => info.pretty_with_indent(indent),
            Element::Callable(info) => info.pretty_with_indent(indent),
            Element::Reference(info) => info.pretty_with_indent(indent),
            Element::Conditional(info) => info.pretty_with_indent(indent),
            Element::Derived(info) => info.pretty_with_indent(indent),
            Element::Intersected(info) => info.pretty_with_indent(indent),
            // All other kinds: identical to Display.
            _ => self.to_string(),
        }
    }

    #[inline]
    fn intersection_types(&self) -> &'static [ElementId] {
        let Element::Intersected(info) = self else { return &[] };
        crate::interner::interner().get_element_list(info.conjuncts)
    }

    #[inline]
    fn has_intersection_types(&self) -> bool {
        !self.intersection_types().is_empty()
    }

    #[inline]
    fn can_be_intersected(&self) -> bool {
        !matches!(self.kind(), ElementKind::Intersected)
    }

    #[inline]
    fn is_complex(&self) -> bool {
        match self {
            Element::ObjectShape(_)
            | Element::Array(_)
            | Element::List(_)
            | Element::Callable(_)
            | Element::Intersected(_) => true,
            Element::Object(info) => info.type_args.is_some(),
            Element::Reference(info) => info.type_args.is_some(),
            Element::Iterable(_) => true,
            _ => false,
        }
    }
}
