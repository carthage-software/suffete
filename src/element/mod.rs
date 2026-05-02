#![allow(clippy::pub_use)]

//! A single element of a [`Type`](crate::Type), and everything it carries.

mod id;
mod kind;
mod value;

pub mod payload;

pub use self::id::ElementId;
pub use self::id::ElementListId;
pub use self::kind::ElementKind;
pub use self::value::Element;
