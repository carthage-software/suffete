//! `numeric` container: `int | float | numeric-string | numeric literals`.
//!
//! A general `string` is NOT numeric (only `numeric-string` and string
//! literals that parse as numbers are).

use crate::ElementId;
use crate::ElementKind;
use crate::interner::interner;
use crate::lattice::family::string;

#[inline]
#[must_use]
pub fn refines(input: ElementId, _container: ElementId) -> bool {
    match input.kind() {
        ElementKind::Int | ElementKind::Float => true,
        ElementKind::String => {
            let info = *interner().get_string(input);
            string::input_is_numeric(info)
        }
        _ => false,
    }
}
