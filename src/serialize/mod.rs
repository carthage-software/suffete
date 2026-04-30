//! Structural serialization for [`TypeId`].
//!
//! `TypeId` is a handle into a process-global interner; its bit pattern
//! depends on the order in which types were interned. Serializing the
//! handle directly is therefore meaningless across processes (and even
//! across runs of the same process). This module instead produces a
//! [`SerializableType`] — a self-contained structural representation
//! that can round-trip through any byte format.
//!
//! # Use it manually
//!
//! ```ignore
//! let serial = my_type.to_serializable();
//! // ... persist `serial` somewhere ...
//! let restored: TypeId = serial.intern();  // re-interned in the local arena
//! ```
//!
//! # Serde
//!
//! With the `serde` Cargo feature enabled, [`SerializableType`] gains
//! `Serialize`/`Deserialize` derives, and a blanket impl on [`TypeId`]
//! delegates round-tripping through [`TypeId::to_serializable`] and
//! [`SerializableType::intern`].
//!
//! # Identity contract
//!
//! Round-tripping preserves **structural content**, not handle bits.
//! After `let id2 = id.to_serializable().intern();`:
//!
//! - `id.content_eq(id2) == true` always.
//! - `id == id2` iff the local interner ended up assigning the same
//!   slot — true within the same process, not guaranteed across.
//!
//! Consumers caching across runs should store [`SerializableType`], not
//! [`TypeId`], and call [`SerializableType::intern`] after deserialising.

use mago_atom::Atom;
use mago_span::Span;

use crate::ElementId;
use crate::ElementKind;
use crate::FlowFlags;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::CallableAlias;
use crate::element::payload::CallableInfo;
use crate::element::payload::ClassLikeKind;
use crate::element::payload::ClassLikeStringInfo;
use crate::element::payload::ClassLikeStringSpecifier;
use crate::element::payload::ConditionalInfo;
use crate::element::payload::DefiningEntity;
use crate::element::payload::DerivedInfo;
use crate::element::payload::EnumInfo;
use crate::element::payload::GenericParameterInfo;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::element::payload::IterableInfo;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownElementEntry;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::KnownPropertyEntry;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::element::payload::MixedInfo;
use crate::element::payload::NameSelector;
use crate::element::payload::ObjectFlags;
use crate::element::payload::ObjectInfo;
use crate::element::payload::ObjectShapeFlags;
use crate::element::payload::ObjectShapeInfo;
use crate::element::payload::ParamFlags;
use crate::element::payload::ParamInfo;
use crate::element::payload::ResourceInfo;
use crate::element::payload::Signature;
use crate::element::payload::SignatureFlags;
use crate::element::payload::SymbolReference;
use crate::element::payload::Truthiness;
use crate::element::payload::VariableInfo;
use crate::element::payload::Visibility;
use crate::element::payload::scalar::FloatInfo;
use crate::element::payload::scalar::IntInfo;
use crate::element::payload::scalar::IntRange;
use crate::element::payload::scalar::LiteralFloat;
use crate::element::payload::scalar::StringCasing;
use crate::element::payload::scalar::StringInfo;
use crate::element::payload::scalar::StringLiteral;
use crate::interner::interner;

mod reference {
    pub use crate::element::payload::GlobalReference;
    pub use crate::element::payload::MemberReference;
}

/// Self-contained structural form of a [`TypeId`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableType {
    pub elements: Vec<SerializableElement>,
    /// Raw [`FlowFlags`] bits.
    pub flags: u16,
    /// Consumer meta byte from [`TypeId::meta`].
    pub meta: u8,
}

/// Self-contained structural form of one element within a
/// [`SerializableType`].
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerializableElement {
    Null,
    Never,
    Void,
    Placeholder,
    Mixed {
        non_null: bool,
        is_empty: bool,
        truthiness: SerializableTruthiness,
    },
    Bool,
    True,
    False,
    Int(SerializableInt),
    Float(SerializableFloat),
    String(SerializableString),
    ClassLikeString {
        kind: SerializableClassLikeKind,
        specifier: SerializableClassLikeSpecifier,
    },
    Scalar,
    Numeric,
    ArrayKey,
    Object {
        name: Atom,
        type_args: Option<Vec<SerializableType>>,
        intersections: Option<Vec<SerializableElement>>,
        is_static: bool,
        is_this: bool,
        remapped_parameters: bool,
    },
    Enum {
        name: Atom,
        case: Option<Atom>,
    },
    ObjectShape {
        known_properties: Vec<SerializableKnownProperty>,
        intersections: Option<Vec<SerializableElement>>,
        sealed: bool,
    },
    HasMethod {
        method_name: Atom,
        intersections: Option<Vec<SerializableElement>>,
    },
    HasProperty {
        property_name: Atom,
        intersections: Option<Vec<SerializableElement>>,
    },
    Array {
        key_param: Option<Box<SerializableType>>,
        value_param: Option<Box<SerializableType>>,
        known_items: Vec<SerializableKnownItem>,
        non_empty: bool,
    },
    List {
        element_type: Box<SerializableType>,
        known_elements: Vec<SerializableKnownElement>,
        known_count: Option<u32>,
        non_empty: bool,
    },
    Iterable {
        key_type: Box<SerializableType>,
        value_type: Box<SerializableType>,
        intersections: Option<Vec<SerializableElement>>,
    },
    Callable(SerializableCallable),
    Resource(SerializableResource),
    GenericParameter {
        name: Atom,
        defining_entity: SerializableDefiningEntity,
        constraint: Box<SerializableType>,
    },
    Variable {
        name: Atom,
    },
    Reference {
        name: Atom,
        type_args: Option<Vec<SerializableType>>,
        intersections: Option<Vec<SerializableElement>>,
    },
    MemberReference {
        class_like_name: Atom,
        selector: SerializableNameSelector,
    },
    GlobalReference {
        selector: SerializableNameSelector,
    },
    Alias {
        class_name: Atom,
        alias_name: Atom,
    },
    Conditional {
        subject: Box<SerializableType>,
        target: Box<SerializableType>,
        then: Box<SerializableType>,
        otherwise: Box<SerializableType>,
        negated: bool,
    },
    Derived(SerializableDerived),
    ObjectAny,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableTruthiness {
    Undetermined,
    Truthy,
    Falsy,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableInt {
    Unspecified,
    UnspecifiedLiteral,
    Literal(i64),
    Range { lower: Option<i64>, upper: Option<i64> },
    NonZero,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableFloat {
    Unspecified,
    UnspecifiedLiteral,
    Literal(f64),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableString {
    pub literal: SerializableStringLiteral,
    pub casing: SerializableStringCasing,
    pub is_numeric: bool,
    pub is_truthy: bool,
    pub is_non_empty: bool,
    pub is_callable: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableStringLiteral {
    None,
    Unspecified,
    Value(Atom),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableStringCasing {
    Unspecified,
    Lowercase,
    Uppercase,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableClassLikeKind {
    Class,
    Interface,
    Enum,
    Trait,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerializableClassLikeSpecifier {
    Any,
    Literal { value: Atom },
    OfType { constraint: Box<SerializableType> },
    Generic { constraint: Box<SerializableType> },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableResource {
    Any,
    Open,
    Closed,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableKnownProperty {
    pub name: Atom,
    pub value: SerializableType,
    pub optional: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableKnownItem {
    pub key: SerializableArrayKey,
    pub value: SerializableType,
    pub optional: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableArrayKey {
    Int(i64),
    String(Atom),
    Const { class: Atom, name: Atom },
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableKnownElement {
    pub index: u32,
    pub value: SerializableType,
    pub optional: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerializableCallable {
    Any,
    Signature(SerializableSignature),
    Closure(SerializableSignature),
    Alias(SerializableCallableAlias),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableSignature {
    pub return_type: SerializableType,
    pub throws: Option<SerializableType>,
    pub parameters: Vec<SerializableParam>,
    pub is_variadic: bool,
    pub is_pure: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct SerializableParam {
    pub name: Atom,
    pub type_: SerializableType,
    pub has_default: bool,
    pub by_reference: bool,
    pub variadic: bool,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableCallableAlias {
    Function(Atom),
    Method { class: Atom, method: Atom },
    Closure(Span),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableDefiningEntity {
    ClassLike(Atom),
    Method { class: Atom, method: Atom },
    Function(Atom),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerializableNameSelector {
    Identifier(Atom),
    StartsWith(Atom),
    EndsWith(Atom),
    Contains(Atom),
    Wildcard,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SerializableDerived {
    KeyOf(Box<SerializableType>),
    ValueOf(Box<SerializableType>),
    PropertiesOf {
        target: Box<SerializableType>,
        visibility: Option<SerializableVisibility>,
    },
    IndexAccess {
        target: Box<SerializableType>,
        index: Box<SerializableType>,
    },
    IntMask(Vec<SerializableType>),
    IntMaskOf(Box<SerializableType>),
    TemplateType {
        object: Box<SerializableType>,
        class_name: Box<SerializableType>,
        template_name: Box<SerializableType>,
    },
    New(Box<SerializableType>),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum SerializableVisibility {
    Public,
    Protected,
    Private,
}

impl TypeId {
    /// Build a self-contained structural mirror of `self` suitable for
    /// persistence. See the [`crate::serialize`] module docs for the
    /// identity contract.
    pub fn to_serializable(self) -> SerializableType {
        let view = self.as_ref();
        SerializableType {
            elements: view.elements.iter().map(|&e| encode_element(e)).collect(),
            flags: self.flags().bits(),
            meta: self.meta(),
        }
    }
}

impl SerializableType {
    /// Re-intern this structural form into the process-global interner
    /// and return a fresh [`TypeId`]. The result `.content_eq()` the
    /// original handle that produced this `SerializableType`.
    pub fn intern(&self) -> TypeId {
        let elements: Vec<ElementId> = self.elements.iter().map(decode_element).collect();
        let id = interner().intern_type(&elements, FlowFlags::from_bits(self.flags));
        id.with_meta(self.meta)
    }
}

impl ElementId {
    /// Build a self-contained structural mirror of `self` suitable for
    /// persistence. Round-trips through [`SerializableElement::intern`]
    /// preserving structural content (handle bits are not preserved
    /// across processes; see [`crate::serialize`] module docs).
    pub fn to_serializable(self) -> SerializableElement {
        encode_element(self)
    }
}

impl SerializableElement {
    /// Re-intern this structural form into the process-global interner
    /// and return a fresh [`ElementId`]. Equivalent (structurally) to
    /// the original element that produced this `SerializableElement`.
    pub fn intern(&self) -> ElementId {
        decode_element(self)
    }
}

fn encode_type(ty: TypeId) -> SerializableType {
    ty.to_serializable()
}

fn encode_element(elem: ElementId) -> SerializableElement {
    let i = interner();
    match elem.kind() {
        ElementKind::Null => SerializableElement::Null,
        ElementKind::Never => SerializableElement::Never,
        ElementKind::Void => SerializableElement::Void,
        ElementKind::Placeholder => SerializableElement::Placeholder,
        ElementKind::Bool => SerializableElement::Bool,
        ElementKind::True => SerializableElement::True,
        ElementKind::False => SerializableElement::False,
        ElementKind::Scalar => SerializableElement::Scalar,
        ElementKind::Numeric => SerializableElement::Numeric,
        ElementKind::ArrayKey => SerializableElement::ArrayKey,
        ElementKind::ObjectAny => SerializableElement::ObjectAny,

        ElementKind::Mixed => {
            let info = *i.get_mixed(elem);
            SerializableElement::Mixed {
                non_null: info.is_non_null(),
                is_empty: info.is_empty(),
                truthiness: encode_truthiness(info.truthiness()),
            }
        }
        ElementKind::Int => SerializableElement::Int(encode_int(*i.get_int(elem))),
        ElementKind::Float => SerializableElement::Float(encode_float(*i.get_float(elem))),
        ElementKind::String => SerializableElement::String(encode_string(i.get_string(elem))),
        ElementKind::ClassLikeString => {
            let info = *i.get_class_like_string(elem);
            SerializableElement::ClassLikeString {
                kind: encode_class_like_kind(info.kind),
                specifier: encode_class_like_specifier(info.specifier),
            }
        }
        ElementKind::Object => {
            let info = *i.get_object(elem);
            SerializableElement::Object {
                name: info.name,
                type_args: info.type_args.map(|id| i.get_type_list(id).iter().map(|&t| encode_type(t)).collect()),
                intersections: info
                    .intersections
                    .map(|id| i.get_element_list(id).iter().map(|&e| encode_element(e)).collect()),
                is_static: info.flags.is_static(),
                is_this: info.flags.is_this(),
                remapped_parameters: info.flags.remapped_parameters(),
            }
        }
        ElementKind::Enum => {
            let info = *i.get_enum(elem);
            SerializableElement::Enum { name: info.name, case: info.case }
        }
        ElementKind::ObjectShape => {
            let info = *i.get_object_shape(elem);
            let known: Vec<SerializableKnownProperty> = info
                .known_properties
                .map(|id| {
                    i.get_known_properties(id)
                        .iter()
                        .map(|p| SerializableKnownProperty {
                            name: p.name,
                            value: encode_type(p.value),
                            optional: p.optional,
                        })
                        .collect()
                })
                .unwrap_or_default();
            SerializableElement::ObjectShape {
                known_properties: known,
                intersections: encode_intersections(info.intersections),
                sealed: info.flags.sealed(),
            }
        }
        ElementKind::HasMethod => {
            let info = i.get_has_method(elem);
            SerializableElement::HasMethod {
                method_name: info.method_name,
                intersections: encode_intersections(info.intersections),
            }
        }
        ElementKind::HasProperty => {
            let info = i.get_has_property(elem);
            SerializableElement::HasProperty {
                property_name: info.property_name,
                intersections: encode_intersections(info.intersections),
            }
        }
        ElementKind::Array => {
            let info = *i.get_array(elem);
            let known_items: Vec<SerializableKnownItem> = info
                .known_items
                .map(|id| {
                    i.get_known_items(id)
                        .iter()
                        .map(|e| SerializableKnownItem {
                            key: encode_array_key(e.key),
                            value: encode_type(e.value),
                            optional: e.optional,
                        })
                        .collect()
                })
                .unwrap_or_default();
            SerializableElement::Array {
                key_param: info.key_param.map(|t| Box::new(encode_type(t))),
                value_param: info.value_param.map(|t| Box::new(encode_type(t))),
                known_items,
                non_empty: info.flags.non_empty(),
            }
        }
        ElementKind::List => {
            let info = *i.get_list(elem);
            let known_elements: Vec<SerializableKnownElement> = info
                .known_elements
                .map(|id| {
                    i.get_known_elements(id)
                        .iter()
                        .map(|e| SerializableKnownElement {
                            index: e.index,
                            value: encode_type(e.value),
                            optional: e.optional,
                        })
                        .collect()
                })
                .unwrap_or_default();
            SerializableElement::List {
                element_type: Box::new(encode_type(info.element_type)),
                known_elements,
                known_count: info.known_count.map(std::num::NonZeroU32::get),
                non_empty: info.flags.non_empty(),
            }
        }
        ElementKind::Iterable => {
            let info = *i.get_iterable(elem);
            SerializableElement::Iterable {
                key_type: Box::new(encode_type(info.key_type)),
                value_type: Box::new(encode_type(info.value_type)),
                intersections: info
                    .intersections
                    .map(|id| i.get_element_list(id).iter().map(|&e| encode_element(e)).collect()),
            }
        }
        ElementKind::Callable => SerializableElement::Callable(encode_callable(*i.get_callable(elem))),
        ElementKind::Resource => SerializableElement::Resource(encode_resource(*i.get_resource(elem))),
        ElementKind::GenericParameter => {
            let info = i.get_generic_parameter(elem);
            SerializableElement::GenericParameter {
                name: info.name,
                defining_entity: encode_defining_entity(*i.get_defining_entity(info.defining_entity)),
                constraint: Box::new(encode_type(info.constraint)),
            }
        }
        ElementKind::Variable => SerializableElement::Variable { name: i.get_variable(elem).name },
        ElementKind::Reference => {
            let info = *i.get_reference(elem);
            SerializableElement::Reference {
                name: info.name,
                type_args: info.type_args.map(|id| i.get_type_list(id).iter().map(|&t| encode_type(t)).collect()),
                intersections: info
                    .intersections
                    .map(|id| i.get_element_list(id).iter().map(|&e| encode_element(e)).collect()),
            }
        }
        ElementKind::MemberReference => {
            let info = *i.get_member_reference(elem);
            SerializableElement::MemberReference {
                class_like_name: info.class_like_name,
                selector: encode_name_selector(info.selector),
            }
        }
        ElementKind::GlobalReference => {
            let info = *i.get_global_reference(elem);
            SerializableElement::GlobalReference { selector: encode_name_selector(info.selector) }
        }
        ElementKind::Alias => {
            let info = i.get_alias(elem);
            SerializableElement::Alias { class_name: info.class_name, alias_name: info.alias_name }
        }
        ElementKind::Conditional => {
            let info = *i.get_conditional(elem);
            SerializableElement::Conditional {
                subject: Box::new(encode_type(info.subject)),
                target: Box::new(encode_type(info.target)),
                then: Box::new(encode_type(info.then)),
                otherwise: Box::new(encode_type(info.otherwise)),
                negated: info.negated,
            }
        }
        ElementKind::Derived => SerializableElement::Derived(encode_derived(*i.get_derived(elem))),
    }
}

fn encode_intersections(intersections: Option<crate::ElementListId>) -> Option<Vec<SerializableElement>> {
    intersections.map(|id| interner().get_element_list(id).iter().map(|&e| encode_element(e)).collect())
}

fn decode_intersections(intersections: Option<&[SerializableElement]>) -> Option<crate::ElementListId> {
    intersections.map(|conjuncts| {
        let elements: Vec<ElementId> = conjuncts.iter().map(decode_element).collect();
        interner().intern_element_list(&elements)
    })
}

fn encode_truthiness(t: Truthiness) -> SerializableTruthiness {
    match t {
        Truthiness::Undetermined => SerializableTruthiness::Undetermined,
        Truthiness::Truthy => SerializableTruthiness::Truthy,
        Truthiness::Falsy => SerializableTruthiness::Falsy,
    }
}

fn encode_int(info: IntInfo) -> SerializableInt {
    match info {
        IntInfo::Unspecified => SerializableInt::Unspecified,
        IntInfo::UnspecifiedLiteral => SerializableInt::UnspecifiedLiteral,
        IntInfo::Literal(n) => SerializableInt::Literal(n),
        IntInfo::Range(rid) => {
            let r = interner().get_int_range(rid);
            SerializableInt::Range { lower: r.lower(), upper: r.upper() }
        }
        IntInfo::NonZero => SerializableInt::NonZero,
    }
}

fn encode_float(info: FloatInfo) -> SerializableFloat {
    match info {
        FloatInfo::Unspecified => SerializableFloat::Unspecified,
        FloatInfo::UnspecifiedLiteral => SerializableFloat::UnspecifiedLiteral,
        FloatInfo::Literal(lit) => SerializableFloat::Literal(lit.value()),
    }
}

fn encode_string(info: &StringInfo) -> SerializableString {
    SerializableString {
        literal: match info.literal {
            StringLiteral::None => SerializableStringLiteral::None,
            StringLiteral::Unspecified => SerializableStringLiteral::Unspecified,
            StringLiteral::Value(a) => SerializableStringLiteral::Value(a),
        },
        casing: match info.casing {
            StringCasing::Unspecified => SerializableStringCasing::Unspecified,
            StringCasing::Lowercase => SerializableStringCasing::Lowercase,
            StringCasing::Uppercase => SerializableStringCasing::Uppercase,
        },
        is_numeric: info.flags.is_numeric(),
        is_truthy: info.flags.is_truthy(),
        is_non_empty: info.flags.is_non_empty(),
        is_callable: info.flags.is_callable(),
    }
}

fn encode_class_like_kind(k: ClassLikeKind) -> SerializableClassLikeKind {
    match k {
        ClassLikeKind::Class => SerializableClassLikeKind::Class,
        ClassLikeKind::Interface => SerializableClassLikeKind::Interface,
        ClassLikeKind::Enum => SerializableClassLikeKind::Enum,
        ClassLikeKind::Trait => SerializableClassLikeKind::Trait,
    }
}

fn encode_class_like_specifier(s: ClassLikeStringSpecifier) -> SerializableClassLikeSpecifier {
    match s {
        ClassLikeStringSpecifier::Any => SerializableClassLikeSpecifier::Any,
        ClassLikeStringSpecifier::Literal { value } => SerializableClassLikeSpecifier::Literal { value },
        ClassLikeStringSpecifier::OfType { constraint } => {
            SerializableClassLikeSpecifier::OfType { constraint: Box::new(encode_type(constraint)) }
        }
        ClassLikeStringSpecifier::Generic { constraint } => {
            SerializableClassLikeSpecifier::Generic { constraint: Box::new(encode_type(constraint)) }
        }
    }
}

fn encode_array_key(k: ArrayKey) -> SerializableArrayKey {
    match k {
        ArrayKey::Int(n) => SerializableArrayKey::Int(n),
        ArrayKey::String(a) => SerializableArrayKey::String(a),
        ArrayKey::Const { class, name } => SerializableArrayKey::Const { class, name },
    }
}

fn encode_callable(info: CallableInfo) -> SerializableCallable {
    let i = interner();
    match info {
        CallableInfo::Any => SerializableCallable::Any,
        CallableInfo::Signature(sid) => SerializableCallable::Signature(encode_signature(*i.get_signature(sid))),
        CallableInfo::Closure(sid) => SerializableCallable::Closure(encode_signature(*i.get_signature(sid))),
        CallableInfo::Alias(aid) => SerializableCallable::Alias(encode_callable_alias(*i.get_callable_alias(aid))),
    }
}

fn encode_signature(sig: Signature) -> SerializableSignature {
    let i = interner();
    let parameters: Vec<SerializableParam> = sig
        .parameters
        .map(|pid| {
            i.get_param_list(pid)
                .iter()
                .map(|p| SerializableParam {
                    name: p.name,
                    type_: encode_type(p.type_),
                    has_default: p.flags.has_default(),
                    by_reference: p.flags.by_reference(),
                    variadic: p.flags.variadic(),
                })
                .collect()
        })
        .unwrap_or_default();
    SerializableSignature {
        return_type: encode_type(sig.return_type),
        throws: sig.throws.map(encode_type),
        parameters,
        is_variadic: sig.flags.is_variadic(),
        is_pure: sig.flags.is_pure(),
    }
}

fn encode_callable_alias(alias: CallableAlias) -> SerializableCallableAlias {
    match alias {
        CallableAlias::Function(name) => SerializableCallableAlias::Function(name),
        CallableAlias::Method { class, method } => SerializableCallableAlias::Method { class, method },
        CallableAlias::Closure(span) => SerializableCallableAlias::Closure(span),
    }
}

fn encode_resource(info: ResourceInfo) -> SerializableResource {
    match info {
        ResourceInfo::Any => SerializableResource::Any,
        ResourceInfo::Open => SerializableResource::Open,
        ResourceInfo::Closed => SerializableResource::Closed,
    }
}

fn encode_defining_entity(e: DefiningEntity) -> SerializableDefiningEntity {
    match e {
        DefiningEntity::ClassLike(name) => SerializableDefiningEntity::ClassLike(name),
        DefiningEntity::Method { class, method } => SerializableDefiningEntity::Method { class, method },
        DefiningEntity::Function(name) => SerializableDefiningEntity::Function(name),
    }
}

fn encode_name_selector(s: NameSelector) -> SerializableNameSelector {
    match s {
        NameSelector::Identifier(a) => SerializableNameSelector::Identifier(a),
        NameSelector::StartsWith(a) => SerializableNameSelector::StartsWith(a),
        NameSelector::EndsWith(a) => SerializableNameSelector::EndsWith(a),
        NameSelector::Contains(a) => SerializableNameSelector::Contains(a),
        NameSelector::Wildcard => SerializableNameSelector::Wildcard,
    }
}

fn encode_derived(info: DerivedInfo) -> SerializableDerived {
    let i = interner();
    match info {
        DerivedInfo::KeyOf(t) => SerializableDerived::KeyOf(Box::new(encode_type(t))),
        DerivedInfo::ValueOf(t) => SerializableDerived::ValueOf(Box::new(encode_type(t))),
        DerivedInfo::PropertiesOf { target, visibility } => SerializableDerived::PropertiesOf {
            target: Box::new(encode_type(target)),
            visibility: visibility.map(encode_visibility),
        },
        DerivedInfo::IndexAccess { target, index } => SerializableDerived::IndexAccess {
            target: Box::new(encode_type(target)),
            index: Box::new(encode_type(index)),
        },
        DerivedInfo::IntMask(list_id) => {
            SerializableDerived::IntMask(i.get_type_list(list_id).iter().map(|&t| encode_type(t)).collect())
        }
        DerivedInfo::IntMaskOf(t) => SerializableDerived::IntMaskOf(Box::new(encode_type(t))),
        DerivedInfo::TemplateType { object, class_name, template_name } => SerializableDerived::TemplateType {
            object: Box::new(encode_type(object)),
            class_name: Box::new(encode_type(class_name)),
            template_name: Box::new(encode_type(template_name)),
        },
        DerivedInfo::New(t) => SerializableDerived::New(Box::new(encode_type(t))),
    }
}

fn encode_visibility(v: Visibility) -> SerializableVisibility {
    match v {
        Visibility::Public => SerializableVisibility::Public,
        Visibility::Protected => SerializableVisibility::Protected,
        Visibility::Private => SerializableVisibility::Private,
    }
}

fn decode_type(t: &SerializableType) -> TypeId {
    t.intern()
}

fn decode_element(elem: &SerializableElement) -> ElementId {
    let i = interner();
    match elem {
        SerializableElement::Null => crate::prelude::NULL,
        SerializableElement::Never => crate::prelude::NEVER,
        SerializableElement::Void => crate::prelude::VOID,
        SerializableElement::Placeholder => crate::prelude::PLACEHOLDER,
        SerializableElement::Bool => crate::prelude::BOOL,
        SerializableElement::True => crate::prelude::TRUE,
        SerializableElement::False => crate::prelude::FALSE,
        SerializableElement::Scalar => crate::prelude::SCALAR,
        SerializableElement::Numeric => crate::prelude::NUMERIC,
        SerializableElement::ArrayKey => crate::prelude::ARRAY_KEY,
        SerializableElement::ObjectAny => crate::prelude::OBJECT,
        SerializableElement::Mixed { non_null, is_empty, truthiness } => {
            let info = MixedInfo::default()
                .with_is_non_null(*non_null)
                .with_is_empty(*is_empty)
                .with_truthiness(decode_truthiness(*truthiness));
            i.intern_mixed(info)
        }
        SerializableElement::Int(s) => i.intern_int(decode_int(*s)),
        SerializableElement::Float(s) => i.intern_float(decode_float(*s)),
        SerializableElement::String(s) => i.intern_string(decode_string(s)),
        SerializableElement::ClassLikeString { kind, specifier } => i.intern_class_like_string(ClassLikeStringInfo {
            kind: decode_class_like_kind(*kind),
            specifier: decode_class_like_specifier(specifier),
        }),
        SerializableElement::Object { name, type_args, intersections, is_static, is_this, remapped_parameters } => {
            let type_args_id =
                type_args.as_ref().map(|args| i.intern_type_list(&args.iter().map(decode_type).collect::<Vec<_>>()));
            let intersections_id = intersections
                .as_ref()
                .map(|conjuncts| i.intern_element_list(&conjuncts.iter().map(decode_element).collect::<Vec<_>>()));
            let flags = ObjectFlags::default()
                .with_is_static(*is_static)
                .with_is_this(*is_this)
                .with_remapped_parameters(*remapped_parameters);
            i.intern_object(ObjectInfo {
                name: *name,
                type_args: type_args_id,
                intersections: intersections_id,
                excluded: None,
                flags,
            })
        }
        SerializableElement::Enum { name, case } => i.intern_enum(EnumInfo { name: *name, case: *case }),
        SerializableElement::ObjectShape { known_properties, intersections, sealed } => {
            let known_id = if known_properties.is_empty() {
                None
            } else {
                let entries: Vec<KnownPropertyEntry> = known_properties
                    .iter()
                    .map(|p| KnownPropertyEntry { name: p.name, value: decode_type(&p.value), optional: p.optional })
                    .collect();
                Some(i.intern_known_properties(&entries))
            };
            i.intern_object_shape(ObjectShapeInfo {
                known_properties: known_id,
                intersections: decode_intersections(intersections.as_deref()),
                flags: ObjectShapeFlags::default().with_sealed(*sealed),
            })
        }
        SerializableElement::HasMethod { method_name, intersections } => i.intern_has_method(HasMethodInfo {
            method_name: *method_name,
            intersections: decode_intersections(intersections.as_deref()),
        }),
        SerializableElement::HasProperty { property_name, intersections } => i.intern_has_property(HasPropertyInfo {
            property_name: *property_name,
            intersections: decode_intersections(intersections.as_deref()),
        }),
        SerializableElement::Array { key_param, value_param, known_items, non_empty } => {
            let known_id = if known_items.is_empty() {
                None
            } else {
                let entries: Vec<KnownItemEntry> = known_items
                    .iter()
                    .map(|e| KnownItemEntry {
                        key: decode_array_key(e.key),
                        value: decode_type(&e.value),
                        optional: e.optional,
                    })
                    .collect();
                Some(i.intern_known_items(&entries))
            };
            i.intern_array(KeyedArrayInfo {
                key_param: key_param.as_ref().map(|t| decode_type(t)),
                value_param: value_param.as_ref().map(|t| decode_type(t)),
                known_items: known_id,
                flags: KeyedArrayFlags::default().with_non_empty(*non_empty),
            })
        }
        SerializableElement::List { element_type, known_elements, known_count, non_empty } => {
            let known_id = if known_elements.is_empty() {
                None
            } else {
                let entries: Vec<KnownElementEntry> = known_elements
                    .iter()
                    .map(|e| KnownElementEntry { index: e.index, value: decode_type(&e.value), optional: e.optional })
                    .collect();
                Some(i.intern_known_elements(&entries))
            };
            i.intern_list(ListInfo {
                element_type: decode_type(element_type),
                known_elements: known_id,
                known_count: known_count.and_then(std::num::NonZeroU32::new),
                flags: ListFlags::default().with_non_empty(*non_empty),
            })
        }
        SerializableElement::Iterable { key_type, value_type, intersections } => {
            let intersections_id = intersections
                .as_ref()
                .map(|conjuncts| i.intern_element_list(&conjuncts.iter().map(decode_element).collect::<Vec<_>>()));
            i.intern_iterable(IterableInfo {
                key_type: decode_type(key_type),
                value_type: decode_type(value_type),
                intersections: intersections_id,
            })
        }
        SerializableElement::Callable(c) => i.intern_callable(decode_callable(c)),
        SerializableElement::Resource(r) => i.intern_resource(decode_resource(*r)),
        SerializableElement::GenericParameter { name, defining_entity, constraint } => {
            let entity_id = i.intern_defining_entity(decode_defining_entity(*defining_entity));
            i.intern_generic_parameter(GenericParameterInfo {
                name: *name,
                defining_entity: entity_id,
                constraint: decode_type(constraint),
                intersections: None,
            })
        }
        SerializableElement::Variable { name } => i.intern_variable(VariableInfo { name: *name }),
        SerializableElement::Reference { name, type_args, intersections } => {
            let type_args_id =
                type_args.as_ref().map(|args| i.intern_type_list(&args.iter().map(decode_type).collect::<Vec<_>>()));
            let intersections_id = intersections
                .as_ref()
                .map(|conjuncts| i.intern_element_list(&conjuncts.iter().map(decode_element).collect::<Vec<_>>()));
            i.intern_reference(SymbolReference {
                name: *name,
                type_args: type_args_id,
                intersections: intersections_id,
            })
        }
        SerializableElement::MemberReference { class_like_name, selector } => {
            i.intern_member_reference(reference::MemberReference {
                class_like_name: *class_like_name,
                selector: decode_name_selector(selector),
            })
        }
        SerializableElement::GlobalReference { selector } => {
            i.intern_global_reference(reference::GlobalReference { selector: decode_name_selector(selector) })
        }
        SerializableElement::Alias { class_name, alias_name } => {
            i.intern_alias(crate::element::payload::AliasInfo { class_name: *class_name, alias_name: *alias_name })
        }
        SerializableElement::Conditional { subject, target, then, otherwise, negated } => {
            i.intern_conditional(ConditionalInfo {
                subject: decode_type(subject),
                target: decode_type(target),
                then: decode_type(then),
                otherwise: decode_type(otherwise),
                negated: *negated,
            })
        }
        SerializableElement::Derived(d) => i.intern_derived(decode_derived(d)),
    }
}

fn decode_truthiness(t: SerializableTruthiness) -> Truthiness {
    match t {
        SerializableTruthiness::Undetermined => Truthiness::Undetermined,
        SerializableTruthiness::Truthy => Truthiness::Truthy,
        SerializableTruthiness::Falsy => Truthiness::Falsy,
    }
}

fn decode_int(s: SerializableInt) -> IntInfo {
    match s {
        SerializableInt::Unspecified => IntInfo::Unspecified,
        SerializableInt::UnspecifiedLiteral => IntInfo::UnspecifiedLiteral,
        SerializableInt::Literal(n) => IntInfo::Literal(n),
        SerializableInt::Range { lower, upper } => {
            let id = interner().intern_int_range(IntRange::new(lower, upper));
            IntInfo::Range(id)
        }
        SerializableInt::NonZero => IntInfo::NonZero,
    }
}

fn decode_float(s: SerializableFloat) -> FloatInfo {
    match s {
        SerializableFloat::Unspecified => FloatInfo::Unspecified,
        SerializableFloat::UnspecifiedLiteral => FloatInfo::UnspecifiedLiteral,
        SerializableFloat::Literal(v) => FloatInfo::Literal(LiteralFloat::new(v)),
    }
}

fn decode_string(s: &SerializableString) -> StringInfo {
    use crate::element::payload::scalar::StringRefinementFlags;
    let literal = match s.literal {
        SerializableStringLiteral::None => StringLiteral::None,
        SerializableStringLiteral::Unspecified => StringLiteral::Unspecified,
        SerializableStringLiteral::Value(a) => StringLiteral::Value(a),
    };
    let casing = match s.casing {
        SerializableStringCasing::Unspecified => StringCasing::Unspecified,
        SerializableStringCasing::Lowercase => StringCasing::Lowercase,
        SerializableStringCasing::Uppercase => StringCasing::Uppercase,
    };
    let flags = StringRefinementFlags::EMPTY
        .with_is_numeric(s.is_numeric)
        .with_is_truthy(s.is_truthy)
        .with_is_non_empty(s.is_non_empty)
        .with_is_callable(s.is_callable);
    StringInfo { literal, casing, flags }
}

fn decode_class_like_kind(k: SerializableClassLikeKind) -> ClassLikeKind {
    match k {
        SerializableClassLikeKind::Class => ClassLikeKind::Class,
        SerializableClassLikeKind::Interface => ClassLikeKind::Interface,
        SerializableClassLikeKind::Enum => ClassLikeKind::Enum,
        SerializableClassLikeKind::Trait => ClassLikeKind::Trait,
    }
}

fn decode_class_like_specifier(s: &SerializableClassLikeSpecifier) -> ClassLikeStringSpecifier {
    match s {
        SerializableClassLikeSpecifier::Any => ClassLikeStringSpecifier::Any,
        SerializableClassLikeSpecifier::Literal { value } => ClassLikeStringSpecifier::Literal { value: *value },
        SerializableClassLikeSpecifier::OfType { constraint } => {
            ClassLikeStringSpecifier::OfType { constraint: decode_type(constraint) }
        }
        SerializableClassLikeSpecifier::Generic { constraint } => {
            ClassLikeStringSpecifier::Generic { constraint: decode_type(constraint) }
        }
    }
}

fn decode_array_key(k: SerializableArrayKey) -> ArrayKey {
    match k {
        SerializableArrayKey::Int(n) => ArrayKey::Int(n),
        SerializableArrayKey::String(a) => ArrayKey::String(a),
        SerializableArrayKey::Const { class, name } => ArrayKey::Const { class, name },
    }
}

fn decode_callable(c: &SerializableCallable) -> CallableInfo {
    let i = interner();
    match c {
        SerializableCallable::Any => CallableInfo::Any,
        SerializableCallable::Signature(s) => CallableInfo::Signature(i.intern_signature(decode_signature(s))),
        SerializableCallable::Closure(s) => CallableInfo::Closure(i.intern_signature(decode_signature(s))),
        SerializableCallable::Alias(a) => CallableInfo::Alias(i.intern_callable_alias(decode_callable_alias(*a))),
    }
}

fn decode_signature(s: &SerializableSignature) -> Signature {
    let i = interner();
    let parameters = if s.parameters.is_empty() {
        None
    } else {
        let entries: Vec<ParamInfo> = s
            .parameters
            .iter()
            .map(|p| ParamInfo {
                name: p.name,
                type_: decode_type(&p.type_),
                flags: ParamFlags::EMPTY
                    .with_has_default(p.has_default)
                    .with_by_reference(p.by_reference)
                    .with_variadic(p.variadic),
            })
            .collect();
        Some(i.intern_param_list(&entries))
    };
    Signature {
        return_type: decode_type(&s.return_type),
        throws: s.throws.as_ref().map(decode_type),
        parameters,
        flags: SignatureFlags::EMPTY.with_is_variadic(s.is_variadic).with_is_pure(s.is_pure),
    }
}

fn decode_callable_alias(a: SerializableCallableAlias) -> CallableAlias {
    match a {
        SerializableCallableAlias::Function(name) => CallableAlias::Function(name),
        SerializableCallableAlias::Method { class, method } => CallableAlias::Method { class, method },
        SerializableCallableAlias::Closure(span) => CallableAlias::Closure(span),
    }
}

fn decode_resource(r: SerializableResource) -> ResourceInfo {
    match r {
        SerializableResource::Any => ResourceInfo::Any,
        SerializableResource::Open => ResourceInfo::Open,
        SerializableResource::Closed => ResourceInfo::Closed,
    }
}

fn decode_defining_entity(e: SerializableDefiningEntity) -> DefiningEntity {
    match e {
        SerializableDefiningEntity::ClassLike(name) => DefiningEntity::ClassLike(name),
        SerializableDefiningEntity::Method { class, method } => DefiningEntity::Method { class, method },
        SerializableDefiningEntity::Function(name) => DefiningEntity::Function(name),
    }
}

fn decode_name_selector(s: &SerializableNameSelector) -> NameSelector {
    match s {
        SerializableNameSelector::Identifier(a) => NameSelector::Identifier(*a),
        SerializableNameSelector::StartsWith(a) => NameSelector::StartsWith(*a),
        SerializableNameSelector::EndsWith(a) => NameSelector::EndsWith(*a),
        SerializableNameSelector::Contains(a) => NameSelector::Contains(*a),
        SerializableNameSelector::Wildcard => NameSelector::Wildcard,
    }
}

fn decode_derived(d: &SerializableDerived) -> DerivedInfo {
    let i = interner();
    match d {
        SerializableDerived::KeyOf(t) => DerivedInfo::KeyOf(decode_type(t)),
        SerializableDerived::ValueOf(t) => DerivedInfo::ValueOf(decode_type(t)),
        SerializableDerived::PropertiesOf { target, visibility } => {
            DerivedInfo::PropertiesOf { target: decode_type(target), visibility: visibility.map(decode_visibility) }
        }
        SerializableDerived::IndexAccess { target, index } => {
            DerivedInfo::IndexAccess { target: decode_type(target), index: decode_type(index) }
        }
        SerializableDerived::IntMask(types) => {
            let ids: Vec<TypeId> = types.iter().map(decode_type).collect();
            DerivedInfo::IntMask(i.intern_type_list(&ids))
        }
        SerializableDerived::IntMaskOf(t) => DerivedInfo::IntMaskOf(decode_type(t)),
        SerializableDerived::TemplateType { object, class_name, template_name } => DerivedInfo::TemplateType {
            object: decode_type(object),
            class_name: decode_type(class_name),
            template_name: decode_type(template_name),
        },
        SerializableDerived::New(t) => DerivedInfo::New(decode_type(t)),
    }
}

fn decode_visibility(v: SerializableVisibility) -> Visibility {
    match v {
        SerializableVisibility::Public => Visibility::Public,
        SerializableVisibility::Protected => Visibility::Protected,
        SerializableVisibility::Private => Visibility::Private,
    }
}

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::Deserialize;
    use serde::Deserializer;
    use serde::Serialize;
    use serde::Serializer;

    impl Serialize for TypeId {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            self.to_serializable().serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for TypeId {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            Ok(SerializableType::deserialize(d)?.intern())
        }
    }

    impl Serialize for ElementId {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            self.to_serializable().serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for ElementId {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            Ok(SerializableElement::deserialize(d)?.intern())
        }
    }
}
