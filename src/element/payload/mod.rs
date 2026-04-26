//! Payload structs for non-trivial [`Element`](crate::Element) kinds.
//!
//! Every payload here obeys the lean-by-design contract: ≤ 24 bytes per slot
//! (or ≤ 32 for entries that inline an [`ArrayKey`]), enforced by
//! `const _: () = assert!(size_of::<…>() <= …)`. Anything that would push past
//! the budget gets pulled out into its own interned slice. The
//! `define_handle!` macro from [`crate::handle`] mints the slim 4-byte
//! `NonZeroU32` newtype, declared next to the payload that uses it.

mod alias;
mod array;
mod conditional;
mod defining_entity;
mod derived;
mod generic_parameter;
mod iterable;
mod mixed;
mod object;
mod reference;
mod resource;
mod variable;

pub mod callable;
pub mod scalar;

pub use self::alias::AliasInfo;
pub use self::array::ArrayKey;
pub use self::array::KeyedArrayFlags;
pub use self::array::KeyedArrayInfo;
pub use self::array::KnownElementEntry;
pub use self::array::KnownElementsId;
pub use self::array::KnownItemEntry;
pub use self::array::KnownItemsId;
pub use self::array::ListFlags;
pub use self::array::ListInfo;
pub use self::callable::CallableAlias;
pub use self::callable::CallableAliasId;
pub use self::callable::CallableInfo;
pub use self::callable::ParamFlags;
pub use self::callable::ParamInfo;
pub use self::callable::ParamListId;
pub use self::callable::Signature;
pub use self::callable::SignatureFlags;
pub use self::callable::SignatureId;
pub use self::conditional::ConditionalInfo;
pub use self::defining_entity::DefiningEntity;
pub use self::defining_entity::DefiningEntityId;
pub use self::derived::DerivedInfo;
pub use self::derived::Visibility;
pub use self::generic_parameter::GenericParameterInfo;
pub use self::iterable::IterableInfo;
pub use self::mixed::MixedInfo;
pub use self::mixed::Truthiness;
pub use self::object::EnumInfo;
pub use self::object::HasMethodInfo;
pub use self::object::HasPropertyInfo;
pub use self::object::KnownPropertiesId;
pub use self::object::KnownPropertyEntry;
pub use self::object::ObjectFlags;
pub use self::object::ObjectInfo;
pub use self::object::ObjectShapeFlags;
pub use self::object::ObjectShapeInfo;
pub use self::reference::GlobalReference;
pub use self::reference::MemberReference;
pub use self::reference::NameSelector;
pub use self::reference::SymbolReference;
pub use self::resource::ResourceInfo;
pub use self::scalar::BoundFlags;
pub use self::scalar::ClassLikeKind;
pub use self::scalar::ClassLikeStringInfo;
pub use self::scalar::ClassLikeStringSpecifier;
pub use self::scalar::FloatInfo;
pub use self::scalar::IntInfo;
pub use self::scalar::IntRange;
pub use self::scalar::IntRangeId;
pub use self::scalar::LiteralFloat;
pub use self::scalar::StringCasing;
pub use self::scalar::StringInfo;
pub use self::scalar::StringLiteral;
pub use self::scalar::StringRefinementFlags;
pub use self::variable::VariableInfo;
