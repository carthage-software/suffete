use std::num::NonZeroU32;

use crate::ElementKind;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::EnumInfo;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::element::payload::scalar::LiteralFloat;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::handle::define_handle;

/// An interned handle to a single [`Element`](crate::Element).
///
/// Layout: 32 bits, niche-optimized via `NonZeroU32`. The high 6 bits hold the
/// [`ElementKind`] tag (1..=63). The low 26 bits hold the per-kind arena slot
/// (0..=2^26-1, ≈67M).
///
/// Two `ElementId`s compare equal iff they refer to the same canonical
/// element; this is the interner's contract. Equality is one `u32` compare,
/// hashing is trivial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ElementId(NonZeroU32);

impl ElementId {
    const KIND_BITS: u32 = 6;
    const SLOT_BITS: u32 = u32::BITS - Self::KIND_BITS;
    const SLOT_MASK: u32 = (1u32 << Self::SLOT_BITS) - 1;

    /// Maximum addressable slot per kind. Each per-kind arena tops out here.
    pub const MAX_SLOT: u32 = Self::SLOT_MASK;

    /// Construct an `ElementId` from a kind and slot. `slot` must fit in
    /// [`Self::MAX_SLOT`]; in release builds this is unchecked.
    #[inline]
    pub const fn new(kind: ElementKind, slot: u32) -> Self {
        debug_assert!(slot <= Self::MAX_SLOT, "element slot overflow");
        let raw = ((kind as u32) << Self::SLOT_BITS) | (slot & Self::SLOT_MASK);
        // SAFETY: `kind as u32 >= 1` (discriminants start at 1), so the shifted
        // kind contributes a non-zero high bit, making the whole value non-zero.
        unsafe { Self(NonZeroU32::new_unchecked(raw)) }
    }

    #[inline]
    pub const fn kind(self) -> ElementKind {
        let tag = (self.0.get() >> Self::SLOT_BITS) as u8;
        // SAFETY: every `ElementId` is constructed from a valid `ElementKind`
        // discriminant (1..=63 fits in 6 bits) via `Self::new`.
        unsafe { std::mem::transmute::<u8, ElementKind>(tag) }
    }

    #[inline]
    pub const fn slot(self) -> u32 {
        self.0.get() & Self::SLOT_MASK
    }

    /// Resolve this handle to a borrowed [`Element`](crate::Element) view via
    /// the process-global interner.
    ///
    /// Trivial-kind elements (no arena entry) return their tag-only variant
    /// directly; payload-bearing kinds return the variant wrapping a
    /// `&'static` reference into the matching per-kind arena.
    ///
    /// # Panics
    ///
    /// Panics for a payload-bearing kind whose slot is unset (which can only
    /// happen if the handle was forged or constructed before boot ran for the
    /// well-known constants in question).
    #[inline]
    pub fn view(self) -> crate::Element {
        use crate::Element;
        let i = crate::interner::interner();
        match self.kind() {
            ElementKind::Null => Element::Null,
            ElementKind::Never => Element::Never,
            ElementKind::Void => Element::Void,
            ElementKind::Placeholder => Element::Placeholder,
            ElementKind::Bool => Element::Bool,
            ElementKind::True => Element::True,
            ElementKind::False => Element::False,
            ElementKind::Scalar => Element::Scalar,
            ElementKind::Numeric => Element::Numeric,
            ElementKind::ArrayKey => Element::ArrayKey,
            ElementKind::ObjectAny => Element::ObjectAny,

            ElementKind::Mixed => Element::Mixed(i.get_mixed(self)),
            ElementKind::Int => Element::Int(i.get_int(self)),
            ElementKind::Float => Element::Float(i.get_float(self)),
            ElementKind::String => Element::String(i.get_string(self)),
            ElementKind::ClassLikeString => Element::ClassLikeString(i.get_class_like_string(self)),
            ElementKind::Object => Element::Object(i.get_object(self)),
            ElementKind::Enum => Element::Enum(i.get_enum(self)),
            ElementKind::ObjectShape => Element::ObjectShape(i.get_object_shape(self)),
            ElementKind::HasMethod => Element::HasMethod(i.get_has_method(self)),
            ElementKind::HasProperty => Element::HasProperty(i.get_has_property(self)),
            ElementKind::Array => Element::Array(i.get_array(self)),
            ElementKind::List => Element::List(i.get_list(self)),
            ElementKind::Iterable => Element::Iterable(i.get_iterable(self)),
            ElementKind::Callable => Element::Callable(i.get_callable(self)),
            ElementKind::Resource => Element::Resource(i.get_resource(self)),
            ElementKind::GenericParameter => Element::GenericParameter(i.get_generic_parameter(self)),
            ElementKind::Variable => Element::Variable(i.get_variable(self)),
            ElementKind::Reference => Element::Reference(i.get_reference(self)),
            ElementKind::MemberReference => Element::MemberReference(i.get_member_reference(self)),
            ElementKind::GlobalReference => Element::GlobalReference(i.get_global_reference(self)),
            ElementKind::Alias => Element::Alias(i.get_alias(self)),
            ElementKind::Conditional => Element::Conditional(i.get_conditional(self)),
            ElementKind::Derived => Element::Derived(i.get_derived(self)),
        }
    }

    /// Intern an integer literal element (`IntInfo::Literal(value)`).
    #[inline]
    pub fn int_literal(value: i64) -> Self {
        crate::interner::interner().intern_int(IntInfo::Literal(value))
    }

    /// Intern a bounded integer range (`IntInfo::Range`). Either bound may be
    /// `None`, denoting open (`-∞` or `+∞`).
    pub fn int_range(lower: Option<i64>, upper: Option<i64>) -> Self {
        let i = crate::interner::interner();
        let range = i.intern_int_range(IntRange::new(lower, upper));
        i.intern_int(IntInfo::Range(range))
    }

    /// Intern a float literal element (`FloatInfo::Literal(value)`).
    #[inline]
    pub fn float_literal(value: f64) -> Self {
        crate::interner::interner().intern_float(FloatInfo::Literal(LiteralFloat::new(value)))
    }

    /// Intern a string literal element with a known value, no casing
    /// constraint, no refinement flags.
    pub fn string_literal(value: &str) -> Self {
        let info = StringInfo {
            literal: StringLiteral::Value(mago_atom::atom(value)),
            casing: StringCasing::Unspecified,
            flags: StringRefinementFlags::EMPTY,
        };
        crate::interner::interner().intern_string(info)
    }

    /// Intern a named object element with no type arguments, no
    /// intersections, and default flags (`is_static = false`,
    /// `is_this = false`, `remapped_parameters = false`).
    pub fn object_named(name: &str) -> Self {
        let info = ObjectInfo {
            name: mago_atom::atom(name),
            type_args: None,
            intersections: None,
            flags: ObjectFlags::default(),
        };
        crate::interner::interner().intern_object(info)
    }

    /// Intern an enum element ("any case of enum `name`").
    pub fn enum_any(name: &str) -> Self {
        let info = EnumInfo { name: mago_atom::atom(name), case: None };
        crate::interner::interner().intern_enum(info)
    }

    /// Intern an enum-case element (`name::case`).
    pub fn enum_case(name: &str, case: &str) -> Self {
        let info = EnumInfo { name: mago_atom::atom(name), case: Some(mago_atom::atom(case)) };
        crate::interner::interner().intern_enum(info)
    }

    /// Intern a literal class-string element (`class-string<Foo>` with a
    /// concrete name).
    pub fn class_string_literal(name: &str) -> Self {
        let info = ClassLikeStringInfo {
            kind: ClassLikeKind::Class,
            specifier: ClassLikeStringSpecifier::Literal { value: mago_atom::atom(name) },
        };
        crate::interner::interner().intern_class_like_string(info)
    }
}

define_handle! {
    /// Handle to an interned `&'static [ElementId]`. Used by payloads that
    /// carry a sequence of elements (object intersections, iterable
    /// intersections, etc.).
    ElementListId
}
