//! Float family: `float`, `literal-float`, float literals.
//!
//! `Float` accepts any Float-kind input; `UnspecifiedLiteral` accepts
//! `Literal(_)` and itself; concrete literals only fit themselves
//! (reflexivity handles that).
//!
//! `int` and any int-family element refine general `Float` via PHP's
//! implicit int-to-float coercion. Refined float forms (`UnspecifiedLiteral`,
//! `Literal(_)`) do NOT accept ints — coercing `1` to `Literal(1.0)` would
//! pin a runtime value the int wasn't carrying.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::FloatInfo;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    let i = interner();
    let container_info = *i.get_float(container);

    // int <: general float (PHP coerces int to float).
    if input.kind() == ElementKind::Int {
        return matches!(container_info, FloatInfo::Unspecified);
    }

    if input.kind() != ElementKind::Float {
        return false;
    }

    let input_info = *i.get_float(input);
    matches!(
        (input_info, container_info),
        (_, FloatInfo::Unspecified)
            | (FloatInfo::Literal(_) | FloatInfo::UnspecifiedLiteral, FloatInfo::UnspecifiedLiteral),
    )
}
