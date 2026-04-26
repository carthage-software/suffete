//! Float family: `float`, `literal-float`, float literals.
//!
//! `Float` accepts any Float-kind input; `UnspecifiedLiteral` accepts
//! `Literal(_)` and itself; concrete literals only fit themselves
//! (reflexivity handles that).

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::FloatInfo;
use crate::interner::interner;

pub fn refines(input: ElementId, container: ElementId) -> bool {
    if input.kind() != ElementKind::Float {
        return false;
    }

    let i = interner();
    let container_info = *i.get_float(container);
    let input_info = *i.get_float(input);

    matches!(
        (input_info, container_info),
        (_, FloatInfo::Unspecified)
            | (FloatInfo::Literal(_) | FloatInfo::UnspecifiedLiteral, FloatInfo::UnspecifiedLiteral),
    )
}
