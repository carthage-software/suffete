//! The type lattice and its three relations.
//!
//! Suffete's type system forms a partially-ordered lattice. This module
//! exposes three operations on it:
//!
//! - [`refines`] — `a <: b` (every value of `a` is a value of `b`).
//! - [`generalizes`] — `a :> b` (every value of `b` is a value of `a`),
//!   the reverse of [`refines`].
//! - [`intersects`] — `a ∩ b ≠ ∅` (there exists a value in both `a` and
//!   `b`).
//!
//! All three take a [`LatticeContext`] (side-effect accumulator for
//! coercions, template bounds, etc.) and a [`Codebase`] (class hierarchy
//! lookups, member existence checks, template metadata).
//!
//! Per-family rules live in [`family`]; each [`crate::ElementKind`] family
//! owns its refinement and (eventually) intersection logic in a dedicated
//! submodule.

mod codebase;
mod context;
pub mod family;
mod intersects;
mod refines;

pub use self::codebase::Codebase;
pub use self::codebase::NullCodebase;
pub use self::context::LatticeContext;
pub use self::intersects::intersects;
pub use self::refines::generalizes;
pub use self::refines::refines;
