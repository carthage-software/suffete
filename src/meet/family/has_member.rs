//! `HasMethod` and `HasProperty` family meet: compose into an
//! [`Intersected`](crate::ElementKind::Intersected) wrapper.

use crate::ElementId;
use crate::interner::interner;

/// `HasMethod(m₁) ∧ HasMethod(m₂)`. When the names match, returns the
/// shared atom. Otherwise wraps both as conjuncts of an `Intersected`.
pub(in crate::meet) fn has_method_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_has_method(a);
    let b_info = *i.get_has_method(b);
    if a_info.method_name == b_info.method_name {
        return Some(a);
    }
    Some(ElementId::intersected(a, &[b]))
}

/// `HasMethod(m) ∧ HasProperty(p)`: orthogonal predicates compose
/// through the [`Intersected`](crate::ElementKind::Intersected) wrapper.
pub(in crate::meet) fn has_method_property_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    Some(ElementId::intersected(a, &[b]))
}

/// `HasProperty(p₁) ∧ HasProperty(p₂)` ; same structure as has-method.
pub(in crate::meet) fn has_property_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_has_property(a);
    let b_info = *i.get_has_property(b);
    if a_info.property_name == b_info.property_name {
        return Some(a);
    }
    Some(ElementId::intersected(a, &[b]))
}
