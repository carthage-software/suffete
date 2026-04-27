//! The type lattice and its three relations.
//!
//! Suffete's type system forms a partially-ordered lattice. This module
//! exposes three operations on it:
//!
//! - [`refines`] — `a <: b` (every value of `a` is a value of `b`).
//! - [`generalizes`] — `a :> b` (every value of `b` is a value of `a`),
//!   the reverse of [`refines`].
//! - [`overlaps`] — `a ∩ b ≠ ∅` (there exists a value in both `a` and
//!   `b`). The boolean overlap question; the type-returning meet
//!   (greatest lower bound) lives in [`crate::meet`].
//!
//! Each takes a [`World`](crate::world::World) (class hierarchy lookups,
//! member existence checks, template metadata), a [`LatticeOptions`] value
//! (caller-set knobs like `ignore_null`), and writes diagnostics into a
//! `&mut LatticeReport` (the [`CoercionCauses`] bitset and an optional
//! replacement [`TypeId`]).
//!
//! Per-family rules live in [`family`]; each [`crate::ElementKind`] family
//! owns its refinement and (eventually) overlap logic in a dedicated
//! submodule.

pub mod family;
mod options;
mod overlaps;
mod refines;
mod report;

pub use self::options::LatticeOptions;
pub use self::overlaps::overlaps;
pub use self::refines::generalizes;
pub use self::refines::refines;
pub use self::report::CoercionCauses;
pub use self::report::LatticeReport;
