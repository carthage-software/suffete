//! Float-family join: literal-count collapse.

use crate::ElementId;
use crate::ElementKind;
use crate::element::payload::scalar::FloatInfo;
use crate::interner::interner;
use crate::prelude::FLOAT;

/// Drop float literals and add the broad `float` form when the
/// literal count exceeds `threshold`.
pub(in crate::join) fn apply_float_literal_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    if elements.contains(&FLOAT) {
        return;
    }

    let i = interner();
    let count = elements
        .iter()
        .filter(|e| e.kind() == ElementKind::Float && matches!(i.get_float(**e), FloatInfo::Literal(_)))
        .count();

    if count as u32 <= u32::from(threshold) {
        return;
    }

    elements.retain(|e| !(e.kind() == ElementKind::Float && matches!(i.get_float(*e), FloatInfo::Literal(_))));
    let pos = elements.binary_search(&FLOAT).unwrap_or_else(|p| p);
    elements.insert(pos, FLOAT);
}
