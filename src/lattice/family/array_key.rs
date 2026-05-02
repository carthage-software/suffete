//! `array-key` container: `int | string | class-like-string` fit. Floats
//! and bools are explicitly NOT array keys.

use crate::ElementId;
use crate::ElementKind;

#[inline]
#[must_use] 
pub const fn refines(input: ElementId, _container: ElementId) -> bool {
    matches!(input.kind(), ElementKind::Int | ElementKind::String | ElementKind::ClassLikeString)
}
