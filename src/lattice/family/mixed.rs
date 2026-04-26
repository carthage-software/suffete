//! Mixed family: vanilla `mixed`, `non-null-mixed`, `truthy-mixed`,
//! `falsy-mixed`, `isset-from-loop`. Not implemented yet (only the Top
//! axiom for vanilla `mixed` is handled by the dispatcher).

use crate::ElementId;

pub fn refines(_input: ElementId, _container: ElementId) -> bool {
    false
}
