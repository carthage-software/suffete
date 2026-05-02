//! Float family: `float`, `literal-float`, float literals.
//!
//! `Float` accepts any Float-kind input; `UnspecifiedLiteral` accepts
//! `Literal(_)` and itself; concrete literals only fit themselves
//! (reflexivity handles that).
//!
//! `int` and `float` are disjoint at the value-set level: the runtime
//! types are distinct, and `is_float($x)` is `false` for an int. PHP's
//! implicit int→float coercion at parameter binding is a callsite
//! convenience, not a subtype relation, so it is intentionally not
//! modeled here. Use a separate "assignable" predicate if a downstream
//! consumer needs the coercion view.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::FloatInfo;
use crate::interner::interner;

#[inline]
#[must_use]
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
