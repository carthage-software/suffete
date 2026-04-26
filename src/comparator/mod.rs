//! The subtype comparator and its supporting types.
//!
//! [`is_subtype`] is the entry point. Callers supply two [`TypeId`]s, a
//! mutable [`SubtypeContext`] (for accumulated coercions, template bounds,
//! and replacement suggestions), and an implementation of [`Codebase`] (the
//! handful of class / member / template lookups the spec rules consult).
//!
//! The current implementation covers the universal axioms (refl / Bot / Top
//! from spec §4.1 and §4.2) and the union dispatch (Union-L / Union-R from
//! §4.3). Family-specific rules (scalar lattice, object hierarchy, callable
//! variance, generic-parameter relational identity, etc.) are added rule by
//! rule; what isn't implemented returns `false` conservatively.

mod codebase;
mod context;
mod subtype;

pub use self::codebase::Codebase;
pub use self::codebase::NullCodebase;
pub use self::context::SubtypeContext;
pub use self::subtype::is_subtype;
