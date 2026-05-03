//! `HasMethod` / `HasProperty` family subtract: equal-name pairs
//! collapse to bottom; otherwise return `a & !b` via the
//! [`Intersected`](crate::ElementKind::Intersected) wrapper.

use crate::ElementId;
use crate::FlowFlags;
use crate::interner::interner;

pub(in crate::subtract) fn has_method_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_has_method(a);
    let b_info = *i.get_has_method(b);
    if a_info.method_name == b_info.method_name {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    Some(vec![ElementId::intersected(a, &[neg])])
}

pub(in crate::subtract) fn has_property_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_has_property(a);
    let b_info = *i.get_has_property(b);
    if a_info.property_name == b_info.property_name {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    Some(vec![ElementId::intersected(a, &[neg])])
}
