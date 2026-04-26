//! Payloads for the object family: nominal, shape, enum, has-method, has-property.
//!
//! Five separate [`ElementKind`](crate::ElementKind)s, one file each. Despite
//! sharing the conceptual umbrella "object", they answer different questions
//! and dispatch independently.

mod enumeration;
mod has_method;
mod has_property;
mod named;
mod shape;

pub use self::enumeration::EnumInfo;
pub use self::has_method::HasMethodInfo;
pub use self::has_property::HasPropertyInfo;
pub use self::named::ObjectFlags;
pub use self::named::ObjectInfo;
pub use self::shape::KnownPropertiesId;
pub use self::shape::KnownPropertyEntry;
pub use self::shape::ObjectShapeFlags;
pub use self::shape::ObjectShapeInfo;
