#![allow(clippy::pub_use)]

//! Template-parameter operations: capture-free substitution today,
//! standin-replacement (inference), bound reconciliation, and expansion
//! as later stages of generics.md land.
//!
//! Generic abstractions and their template parameters are described in
//! the `World` trait ([`crate::world::TemplateParameter`],
//! [`crate::world::Variance`]); this module operates *on* types whose
//! atoms reference those parameters.

mod reconcile;
mod standin;
mod substitute;

pub use self::reconcile::reconcile;
pub use self::standin::Bound;
pub use self::standin::BoundKind;
pub use self::standin::GenericTemplate;
pub use self::standin::StandinOptions;
pub use self::standin::TemplateKey;
pub use self::standin::TemplateResult;
pub use self::standin::TemplateState;
pub use self::standin::standin;
pub use self::substitute::substitute;
