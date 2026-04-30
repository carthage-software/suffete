//! Prelude: well-known [`ElementId`]s and [`TypeId`]s, fixed at compile time.
//!
//! Trivial-kind elements (those whose [`ElementKind`] alone determines their
//! identity, like `null`, `never`, `void`, `placeholder`, `true`, `false`,
//! `scalar`, `numeric`, `array-key`) live entirely in their tag and use slot
//! `0`. They never hit an arena.
//!
//! Non-trivial well-known elements (e.g. `int`, `positive-int`, `0`, `""`,
//! `non-empty-string`) are pre-assigned slot indices in their per-kind arena,
//! starting at `0` and assigned in declaration order. The boot routine
//! populates exactly those slots first so the constants below resolve.
//!
//! Pre-canonicalized well-known unions (e.g. `int|float`, `null|string`,
//! `-1|0|1`) get fixed [`TypeId`]s for the same reason: lookup-free identity
//! for the most common types in any PHP world. `TypeId` slots are 1-based
//! (slot `0` is reserved as the `NonZero` niche).
//!
//! There is intentionally no well-known `Closure` element. A bare `\Closure`
//! type (the class with no known signature) is represented as
//! [`ObjectInfo`](crate::payload::ObjectInfo) `Named(\Closure)` and has no
//! fixed `ElementId`; its name is an `Atom` interned at runtime.

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;

const TRIVIAL_SLOT: u32 = 0;

pub const NULL: ElementId = ElementId::new(ElementKind::Null, TRIVIAL_SLOT);
pub const NEVER: ElementId = ElementId::new(ElementKind::Never, TRIVIAL_SLOT);
pub const VOID: ElementId = ElementId::new(ElementKind::Void, TRIVIAL_SLOT);
pub const PLACEHOLDER: ElementId = ElementId::new(ElementKind::Placeholder, TRIVIAL_SLOT);

pub const TRUE: ElementId = ElementId::new(ElementKind::True, TRIVIAL_SLOT);
pub const FALSE: ElementId = ElementId::new(ElementKind::False, TRIVIAL_SLOT);

pub const SCALAR: ElementId = ElementId::new(ElementKind::Scalar, TRIVIAL_SLOT);
pub const NUMERIC: ElementId = ElementId::new(ElementKind::Numeric, TRIVIAL_SLOT);
pub const ARRAY_KEY: ElementId = ElementId::new(ElementKind::ArrayKey, TRIVIAL_SLOT);

pub const MIXED: ElementId = ElementId::new(ElementKind::Mixed, 0);
pub const NON_NULL_MIXED: ElementId = ElementId::new(ElementKind::Mixed, 1);
pub const TRUTHY_MIXED: ElementId = ElementId::new(ElementKind::Mixed, 2);
pub const FALSY_MIXED: ElementId = ElementId::new(ElementKind::Mixed, 3);
pub const ISSET_FROM_LOOP: ElementId = ElementId::new(ElementKind::Mixed, 4);

pub const BOOL: ElementId = ElementId::new(ElementKind::Bool, TRIVIAL_SLOT);

pub const OBJECT: ElementId = ElementId::new(ElementKind::ObjectAny, TRIVIAL_SLOT);

pub const INT: ElementId = ElementId::new(ElementKind::Int, 0);
pub const POSITIVE_INT: ElementId = ElementId::new(ElementKind::Int, 1);
pub const NEGATIVE_INT: ElementId = ElementId::new(ElementKind::Int, 2);
pub const NON_POSITIVE_INT: ElementId = ElementId::new(ElementKind::Int, 3);
pub const NON_NEGATIVE_INT: ElementId = ElementId::new(ElementKind::Int, 4);
pub const LITERAL_INT: ElementId = ElementId::new(ElementKind::Int, 5);
pub const INT_ZERO: ElementId = ElementId::new(ElementKind::Int, 6);
pub const INT_ONE: ElementId = ElementId::new(ElementKind::Int, 7);
pub const INT_MINUS_ONE: ElementId = ElementId::new(ElementKind::Int, 8);
pub const NON_ZERO_INT: ElementId = ElementId::new(ElementKind::Int, 9);

pub const FLOAT: ElementId = ElementId::new(ElementKind::Float, 0);
pub const LITERAL_FLOAT: ElementId = ElementId::new(ElementKind::Float, 1);

pub const STRING: ElementId = ElementId::new(ElementKind::String, 0);
pub const NON_EMPTY_STRING: ElementId = ElementId::new(ElementKind::String, 1);
pub const TRUTHY_STRING: ElementId = ElementId::new(ElementKind::String, 2);
pub const LOWERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 3);
pub const UPPERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 4);
pub const NON_EMPTY_LOWERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 5);
pub const NON_EMPTY_UPPERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 6);
pub const TRUTHY_LOWERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 7);
pub const TRUTHY_UPPERCASE_STRING: ElementId = ElementId::new(ElementKind::String, 8);
pub const NUMERIC_STRING: ElementId = ElementId::new(ElementKind::String, 9);
pub const TRUTHY_NUMERIC_STRING: ElementId = ElementId::new(ElementKind::String, 10);
pub const CALLABLE_STRING: ElementId = ElementId::new(ElementKind::String, 11);
pub const LOWERCASE_CALLABLE_STRING: ElementId = ElementId::new(ElementKind::String, 12);
pub const UPPERCASE_CALLABLE_STRING: ElementId = ElementId::new(ElementKind::String, 13);
pub const LITERAL_STRING: ElementId = ElementId::new(ElementKind::String, 14);
pub const NON_EMPTY_LITERAL_STRING: ElementId = ElementId::new(ElementKind::String, 15);
pub const EMPTY_STRING: ElementId = ElementId::new(ElementKind::String, 16);

pub const CLASS_STRING: ElementId = ElementId::new(ElementKind::ClassLikeString, 0);
pub const INTERFACE_STRING: ElementId = ElementId::new(ElementKind::ClassLikeString, 1);
pub const ENUM_STRING: ElementId = ElementId::new(ElementKind::ClassLikeString, 2);
pub const TRAIT_STRING: ElementId = ElementId::new(ElementKind::ClassLikeString, 3);

pub const RESOURCE: ElementId = ElementId::new(ElementKind::Resource, 0);
pub const OPEN_RESOURCE: ElementId = ElementId::new(ElementKind::Resource, 1);
pub const CLOSED_RESOURCE: ElementId = ElementId::new(ElementKind::Resource, 2);

pub const ITERABLE_MIXED_MIXED: ElementId = ElementId::new(ElementKind::Iterable, 0);
pub const EMPTY_ARRAY: ElementId = ElementId::new(ElementKind::Array, 0);
pub const ARRAY_KEY_MIXED: ElementId = ElementId::new(ElementKind::Array, 1);
pub const CALLABLE: ElementId = ElementId::new(ElementKind::Callable, 0);

pub const TYPE_NULL: TypeId = TypeId::from_slot(1);
pub const TYPE_NEVER: TypeId = TypeId::from_slot(2);
pub const TYPE_VOID: TypeId = TypeId::from_slot(3);
pub const TYPE_MIXED: TypeId = TypeId::from_slot(4);
pub const TYPE_BOOL: TypeId = TypeId::from_slot(5);
pub const TYPE_TRUE: TypeId = TypeId::from_slot(6);
pub const TYPE_FALSE: TypeId = TypeId::from_slot(7);
pub const TYPE_INT: TypeId = TypeId::from_slot(8);
pub const TYPE_FLOAT: TypeId = TypeId::from_slot(9);
pub const TYPE_STRING: TypeId = TypeId::from_slot(10);
pub const TYPE_OBJECT: TypeId = TypeId::from_slot(11);
pub const TYPE_SCALAR: TypeId = TypeId::from_slot(12);
pub const TYPE_NUMERIC: TypeId = TypeId::from_slot(13);
pub const TYPE_ARRAY_KEY: TypeId = TypeId::from_slot(14);
pub const TYPE_CALLABLE: TypeId = TypeId::from_slot(15);

pub const TYPE_INT_OR_FLOAT: TypeId = TypeId::from_slot(16);
pub const TYPE_INT_OR_STRING: TypeId = TypeId::from_slot(17);
pub const TYPE_NULL_OR_SCALAR: TypeId = TypeId::from_slot(18);
pub const TYPE_NULL_OR_STRING: TypeId = TypeId::from_slot(19);
pub const TYPE_NULL_OR_INT: TypeId = TypeId::from_slot(20);
pub const TYPE_NULL_OR_FLOAT: TypeId = TypeId::from_slot(21);
pub const TYPE_NULL_OR_OBJECT: TypeId = TypeId::from_slot(22);
pub const TYPE_MINUS_ONE_ZERO_ONE: TypeId = TypeId::from_slot(23);

/// Number of `TypeId` slots reserved for well-known types. The interner allocates
/// from `WELL_KNOWN_TYPE_COUNT + 1` onward.
pub const WELL_KNOWN_TYPE_COUNT: u32 = 23;
