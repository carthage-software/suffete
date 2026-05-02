#![allow(clippy::pub_use)]

//! The user-facing type: a union of one or more [`Element`](crate::Element)s
//! plus flow flags.

mod flags;
mod id;
mod value;

pub use self::flags::FlowFlags;
pub use self::id::TypeId;
pub use self::id::TypeListId;
pub use self::value::Type;
