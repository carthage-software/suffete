#![allow(clippy::pub_use, clippy::absolute_paths, clippy::missing_docs_in_private_items)]

//! Suffete: a standalone PHP type system.
//!
//! See `README.md` for what this crate is, what it is not, and its (highly unstable) status.

extern crate alloc;

pub mod builder;
pub mod cast;
pub mod compatibility;
pub mod element;
pub mod expand;
pub mod handle;
pub mod hierarchy;
pub mod inspect;
pub mod interner;
pub mod join;
pub mod lattice;
pub mod meet;
pub mod predicates;
pub mod prelude;
pub mod serialize;
pub mod subtract;
pub mod template;
pub mod transform;
pub mod ty;
pub mod typed;
pub mod widen;
pub mod world;

pub use crate::builder::TypeBuilder;
pub use crate::element::Element;
pub use crate::element::ElementId;
pub use crate::element::ElementKind;
pub use crate::element::ElementListId;
pub use crate::ty::FlowFlags;
pub use crate::ty::Type;
pub use crate::ty::TypeId;
pub use crate::ty::TypeListId;
