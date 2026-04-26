//! Per-family lattice rule helpers.
//!
//! Each submodule owns the refinement (`<:`) and overlap (`∩ ≠ ∅`) rules
//! for one element family. The top-level [`refines`](crate::lattice::refines)
//! and [`overlaps`](crate::lattice::overlaps) entry points dispatch into
//! these by container kind (refines) or pair of kinds (overlaps).
//!
//! Each family file follows the same shape:
//!
//! - `refines(input, container)` — returns `true` iff `input <: container`
//!   under that family's rules. Reflexivity, Bot (`never <: anything`), and
//!   Top (`anything <: mixed`) are the entry points' responsibility, never
//!   each family's.
//! - `overlaps(a, b)` — returns `true` iff `a ∩ b ≠ ∅`. Symmetric.
//!
//! Stub modules return `false` conservatively until their rules land.

pub mod array;
pub mod array_key;
pub mod bool;
pub mod callable;
pub mod class_like_string;
pub mod float;
pub mod generic;
pub mod int;
pub mod iterable;
pub mod mixed;
pub mod numeric;
pub mod object;
pub mod reference;
pub mod resource;
pub mod scalar;
pub mod string;
