//! Scalar synthesis: `int | string | float | bool` → `scalar`.

use crate::ElementId;
use crate::prelude::BOOL;
use crate::prelude::FLOAT;
use crate::prelude::INT;
use crate::prelude::SCALAR;
use crate::prelude::STRING;

/// When the union contains all four general primitives (`int`, `string`,
/// `float`, `bool`), collapse them to `scalar`. Refined / literal
/// forms alone don't trigger the collapse: only the general
/// unspecified forms count. Other scalar elements (literals,
/// refinements, class-like-strings) remain independent and are left
/// to subtype absorption.
pub(in crate::join) fn apply_scalar_synthesis(elements: &mut Vec<ElementId>) {
    use crate::element::simd::contains;
    let has_int = contains(elements, INT);
    let has_string = contains(elements, STRING);
    let has_float = contains(elements, FLOAT);
    let has_bool = contains(elements, BOOL);
    if !(has_int && has_string && has_float && has_bool) {
        return;
    }

    elements.retain(|e| *e != INT && *e != STRING && *e != FLOAT && *e != BOOL);
    let pos = elements.binary_search(&SCALAR).unwrap_or_else(|p| p);
    elements.insert(pos, SCALAR);
}
