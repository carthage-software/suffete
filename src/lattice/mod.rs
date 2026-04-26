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
//! Each takes a [`World`](crate::world::World) (class hierarchy lookups,
//! member existence checks, template metadata), a [`LatticeOptions`] value
//! (caller-set knobs like `ignore_null`), and writes diagnostics into a
//! `&mut LatticeReport` (`type_coerced` and friends).
//!
//! Per-family rules live in [`family`]; each [`crate::ElementKind`] family
//! owns its refinement and (eventually) intersection logic in a dedicated
//! submodule.

pub mod family;
mod intersects;
mod options;
mod refines;
mod report;

pub use self::intersects::intersects;
pub use self::options::LatticeOptions;
pub use self::refines::generalizes;
pub use self::refines::refines;
pub use self::report::LatticeReport;
