use std::num::NonZeroU32;

use crate::Element;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::DefiningEntity;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownElementEntry;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::Signature;
use crate::element::payload::SignatureFlags;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::element::payload::scalar::LiteralFloat;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::element::payload::scalar::StringRefinementFlags;
use crate::handle::define_handle;
use crate::interner::interner;

/// `true` iff `s` parses as an integer or float — used to derive the
/// `is_numeric` flag on literal strings.
fn is_numeric_string(s: &str) -> bool {
    s.parse::<i64>().is_ok() || s.parse::<f64>().is_ok()
}
use crate::prelude::*;
use crate::typed::Typed;

/// An interned handle to a single [`Element`](crate::Element).
///
/// Layout: 32 bits, niche-optimized via `NonZeroU32`. The high 6 bits hold the
/// [`ElementKind`] tag (1..=63). The low 26 bits hold the per-kind arena slot
/// (0..=2^26-1, ≈67M).
///
/// Two `ElementId`s compare equal iff they refer to the same canonical
/// element; this is the interner's contract. Equality is one `u32` compare,
/// hashing is trivial.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    pub fn view(self) -> Element {
        let i = interner();
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
        interner().intern_int(IntInfo::Literal(value))
    }

    /// Intern a bounded integer range (`IntInfo::Range`). Either bound may be
    /// `None`, denoting open (`-∞` or `+∞`).
    pub fn int_range(lower: Option<i64>, upper: Option<i64>) -> Self {
        let i = interner();
        let range = i.intern_int_range(IntRange::new(lower, upper));
        i.intern_int(IntInfo::Range(range))
    }

    /// Intern a float literal element (`FloatInfo::Literal(value)`).
    #[inline]
    pub fn float_literal(value: f64) -> Self {
        interner().intern_float(FloatInfo::Literal(LiteralFloat::new(value)))
    }

    /// Intern a string literal element. Refinement properties
    /// (`is_numeric`, `is_truthy`, `is_non_empty`) and casing are derived
    /// from the value: `"hello"` is non-empty and truthy and lowercase;
    /// `""` is none of those; `"123"` is numeric and truthy.
    pub fn string_literal(value: &str) -> Self {
        let is_numeric = is_numeric_string(value);
        let is_non_empty = is_numeric || !value.is_empty();
        let is_truthy = is_non_empty && value != "0";
        let has_lower = value.chars().any(|c| c.is_ascii_lowercase());
        let has_upper = value.chars().any(|c| c.is_ascii_uppercase());
        let casing = if has_lower && !has_upper {
            StringCasing::Lowercase
        } else if has_upper && !has_lower {
            StringCasing::Uppercase
        } else {
            StringCasing::Unspecified
        };
        let info = StringInfo {
            literal: StringLiteral::Value(mago_atom::atom(value)),
            casing,
            flags: StringRefinementFlags::EMPTY
                .with_is_numeric(is_numeric)
                .with_is_truthy(is_truthy)
                .with_is_non_empty(is_non_empty),
        };
        interner().intern_string(info)
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
        interner().intern_object(info)
    }

    /// Intern an enum element ("any case of enum `name`").
    pub fn enum_any(name: &str) -> Self {
        let info = EnumInfo { name: mago_atom::atom(name), case: None };
        interner().intern_enum(info)
    }

    /// Intern an enum-case element (`name::case`).
    pub fn enum_case(name: &str, case: &str) -> Self {
        let info = EnumInfo { name: mago_atom::atom(name), case: Some(mago_atom::atom(case)) };
        interner().intern_enum(info)
    }

    /// Intern a literal class-string element (`class-string<Foo>` with a
    /// concrete name).
    pub fn class_string_literal(name: &str) -> Self {
        let info = ClassLikeStringInfo {
            kind: ClassLikeKind::Class,
            specifier: ClassLikeStringSpecifier::Literal { value: mago_atom::atom(name) },
        };
        interner().intern_class_like_string(info)
    }

    /// Intern an `iterable<key, value>` element with no intersections.
    pub fn iterable(key_type: TypeId, value_type: TypeId) -> Self {
        let info = IterableInfo { key_type, value_type, intersections: None };
        interner().intern_iterable(info)
    }

    /// Intern a `list<element>` (or `non-empty-list<element>`) element with
    /// no fixed-position elements.
    pub fn list(element_type: TypeId, non_empty: bool) -> Self {
        let info = ListInfo {
            element_type,
            known_elements: None,
            known_count: None,
            flags: ListFlags::default().with_non_empty(non_empty),
        };
        interner().intern_list(info)
    }

    /// Intern a sealed list element (`list{0: T0, 1: T1, ...}`) with the
    /// given known entries and no rest element type.
    pub fn sealed_list(elements: &[KnownElementEntry], non_empty: bool) -> Self {
        let i = interner();
        let known_count = NonZeroU32::new(elements.len() as u32);
        let info = ListInfo {
            element_type: TYPE_NEVER,
            known_elements: Some(i.intern_known_elements(elements)),
            known_count,
            flags: ListFlags::default().with_non_empty(non_empty),
        };
        i.intern_list(info)
    }

    /// Intern an unsealed keyed-array element (`array<K, V>` /
    /// `non-empty-array<K, V>`) with no known fixed entries.
    pub fn keyed_unsealed(key_type: TypeId, value_type: TypeId, non_empty: bool) -> Self {
        let info = KeyedArrayInfo {
            key_param: Some(key_type),
            value_param: Some(value_type),
            known_items: None,
            flags: KeyedArrayFlags::default().with_non_empty(non_empty),
        };
        interner().intern_array(info)
    }

    /// Intern a sealed keyed-array element (`array{a: int, b: string, ...}`)
    /// with the given known entries and no rest type.
    pub fn keyed_sealed(items: &[KnownItemEntry], non_empty: bool) -> Self {
        let i = interner();
        let known = i.intern_known_items(items);
        let info = KeyedArrayInfo {
            key_param: None,
            value_param: None,
            known_items: Some(known),
            flags: KeyedArrayFlags::default().with_non_empty(non_empty),
        };
        i.intern_array(info)
    }

    /// Intern an `Any` callable (`callable` with no signature info).
    pub fn callable_any() -> Self {
        interner().intern_callable(CallableInfo::Any)
    }

    /// Intern a `callable(...)` with a "mixed" signature: parameters
    /// unspecified, return type `mixed`, no `throws`. Common test fixture.
    pub fn callable_mixed() -> Self {
        let i = interner();
        let sig = i.intern_signature(Signature {
            parameters: None,
            return_type: TYPE_MIXED,
            throws: None,
            flags: SignatureFlags::EMPTY,
        });

        i.intern_callable(CallableInfo::Signature(sig))
    }

    /// Intern a `Closure(...)` with the same "mixed" signature as
    /// [`callable_mixed`](Self::callable_mixed) but tagged as a closure.
    pub fn closure_mixed() -> Self {
        let i = interner();
        let sig = i.intern_signature(Signature {
            parameters: None,
            return_type: TYPE_MIXED,
            throws: None,
            flags: SignatureFlags::EMPTY,
        });
        i.intern_callable(CallableInfo::Closure(sig))
    }

    /// Intern a generic parameter element (a reference to `@template T`).
    /// `defining_entity` qualifies the parameter so two `T`s declared in
    /// different scopes stay distinct. `constraint` is the upper bound;
    /// pass [`TYPE_MIXED`] for an unbounded parameter.
    pub fn generic_parameter(name: &str, defining_entity: DefiningEntity, constraint: TypeId) -> Self {
        let i = interner();
        let entity_id = i.intern_defining_entity(defining_entity);
        let info = GenericParameterInfo {
            name: mago_atom::atom(name),
            defining_entity: entity_id,
            constraint,
            intersections: None,
        };

        i.intern_generic_parameter(info)
    }

    /// `&` conjuncts this element is intersected with, if it supports
    /// intersections. Returns an empty slice for elements that don't
    /// support intersections, or that support them but have none.
    ///
    /// Element kinds that support intersections: `Object`, `Iterable`,
    /// `ObjectShape`, `HasMethod`, `HasProperty`, `GenericParameter`,
    /// `Reference`. Everything else returns `&[]`.
    pub fn intersection_types(self) -> &'static [ElementId] {
        let i = interner();
        let id = match self.kind() {
            ElementKind::Object => i.get_object(self).intersections,
            ElementKind::Iterable => i.get_iterable(self).intersections,
            ElementKind::ObjectShape => i.get_object_shape(self).intersections,
            ElementKind::HasMethod => i.get_has_method(self).intersections,
            ElementKind::HasProperty => i.get_has_property(self).intersections,
            ElementKind::GenericParameter => i.get_generic_parameter(self).intersections,
            ElementKind::Reference => i.get_reference(self).intersections,
            _ => return &[],
        };

        match id {
            Some(list_id) => i.get_element_list(list_id),
            None => &[],
        }
    }

    /// `true` iff this element has at least one intersection conjunct.
    #[inline]
    pub fn has_intersection_types(self) -> bool {
        !self.intersection_types().is_empty()
    }

    /// `true` iff this element's kind supports intersections at all
    /// (regardless of whether the current instance has any).
    #[inline]
    pub const fn can_be_intersected(self) -> bool {
        matches!(
            self.kind(),
            ElementKind::Object
                | ElementKind::Iterable
                | ElementKind::ObjectShape
                | ElementKind::HasMethod
                | ElementKind::HasProperty
                | ElementKind::GenericParameter
                | ElementKind::Reference
        )
    }
}

define_handle! {
    /// Handle to an interned `&'static [ElementId]`. Used by payloads that
    /// carry a sequence of elements (object intersections, iterable
    /// intersections, etc.).
    ElementListId
}

impl std::fmt::Display for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.view(), f)
    }
}

impl std::fmt::Debug for ElementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.debug_struct("ElementId")
                .field("raw", &format_args!("{:#010x}", self.0.get()))
                .field("kind", &self.kind())
                .field("slot", &self.slot())
                .field("display", &format_args!("{}", self))
                .finish()
        } else {
            write!(f, "ElementId({:#010x} = {:?}: {})", self.0.get(), self.kind(), self)
        }
    }
}

impl Typed for ElementId {
    fn pretty_with_indent(&self, indent: usize) -> String {
        Typed::pretty_with_indent(&self.view(), indent)
    }

    fn intersection_types(&self) -> &'static [ElementId] {
        ElementId::intersection_types(*self)
    }

    fn has_intersection_types(&self) -> bool {
        ElementId::has_intersection_types(*self)
    }

    fn can_be_intersected(&self) -> bool {
        ElementId::can_be_intersected(*self)
    }

    fn is_complex(&self) -> bool {
        Typed::is_complex(&self.view())
    }
}
