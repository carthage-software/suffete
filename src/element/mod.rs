#![allow(clippy::pub_use)]

//! A single element of a [`Type`](crate::Type), and everything it carries.

mod id;
mod kind;
mod value;

pub(crate) mod simd;

pub mod payload;

pub use self::id::ElementId;
pub use self::id::ElementListId;
pub use self::id::reconstruct_with_intersections;
pub use self::kind::ElementKind;
pub use self::simd::any_of_kind;
pub use self::simd::contains;
pub use self::simd::count_of_kind;
pub use self::value::Element;
