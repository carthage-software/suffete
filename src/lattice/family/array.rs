//! Array family: keyed arrays, lists, sealed shapes. Not implemented yet.
//!
//! Future rules will cover empty-array as a subtype of every list/keyed
//! shape, sealed-list shape subtyping by element-position containment,
//! keyed-array subtyping by key/value union containment, and so on.

use crate::ElementId;

pub fn refines(_input: ElementId, _container: ElementId) -> bool {
    false
}
