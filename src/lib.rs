//! Suffete: a standalone PHP type system.
//!
//! See `README.md` for what this crate is, what it is not, and its (highly unstable) status.

pub mod element;
pub mod handle;
pub mod interner;
pub mod join;
pub mod lattice;
pub mod prelude;
pub mod ty;
pub mod world;

pub use crate::element::Element;
pub use crate::element::ElementId;
pub use crate::element::ElementKind;
pub use crate::element::ElementListId;
pub use crate::ty::FlowFlags;
pub use crate::ty::Type;
pub use crate::ty::TypeId;
pub use crate::ty::TypeListId;
