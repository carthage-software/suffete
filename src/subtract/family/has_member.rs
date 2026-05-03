//! `HasMethod` / `HasProperty` family subtract: equal-name pairs
//! collapse to bottom; otherwise attach `Negated(b)` so the lattice
//! preserves the narrowing.

use crate::ElementId;
use crate::FlowFlags;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::interner::interner;

pub(in crate::subtract) fn has_method_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_has_method(a);
    let b_info = *i.get_has_method(b);
    if a_info.method_name == b_info.method_name && a_info.intersections == b_info.intersections {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    let merged = merge_intersections(a_info.intersections, neg);
    Some(vec![i.intern_has_method(HasMethodInfo { method_name: a_info.method_name, intersections: Some(merged) })])
}

pub(in crate::subtract) fn has_property_minus(a: ElementId, b: ElementId) -> Option<Vec<ElementId>> {
    let i = interner();
    let a_info = *i.get_has_property(a);
    let b_info = *i.get_has_property(b);
    if a_info.property_name == b_info.property_name && a_info.intersections == b_info.intersections {
        return Some(Vec::new());
    }

    let b_t = i.intern_type(&[b], FlowFlags::EMPTY);
    let neg = ElementId::negated(b_t);
    let merged = merge_intersections(a_info.intersections, neg);
    Some(vec![
        i.intern_has_property(HasPropertyInfo { property_name: a_info.property_name, intersections: Some(merged) }),
    ])
}

#[inline]
fn merge_intersections(existing: Option<crate::ElementListId>, extra: ElementId) -> crate::ElementListId {
    let i = interner();
    let mut conjuncts: Vec<ElementId> = existing.map(|id| i.get_element_list(id).to_vec()).unwrap_or_default();
    if !conjuncts.contains(&extra) {
        conjuncts.push(extra);
    }

    conjuncts.sort();
    i.intern_element_list(&conjuncts)
}
