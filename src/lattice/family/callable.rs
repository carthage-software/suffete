//! Callable family: `callable`, `Closure`, plain function pointers, with
//! optional signatures (parameters + return type). Not implemented yet.
//!
//! Future rules will cover the standard variance pattern: contravariant
//! parameters, covariant return.

use crate::ElementId;

pub fn refines(_input: ElementId, _container: ElementId) -> bool {
    false
}
