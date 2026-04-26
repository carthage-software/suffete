//! Template-parameter operations: capture-free substitution today,
//! standin-replacement (inference), bound reconciliation, and expansion
//! as later stages of generics.md land.
//!
//! Generic abstractions and their template parameters are described in
//! the `World` trait ([`crate::world::TemplateParameter`],
//! [`crate::world::Variance`]); this module operates *on* types whose
//! atoms reference those parameters.

mod substitute;

pub use self::substitute::substitute;
