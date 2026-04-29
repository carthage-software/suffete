//! `HasMethod` and `HasProperty` family meet: compose into an
//! intersection-bearing form.

use crate::ElementId;
use crate::element::payload::HasMethodInfo;
use crate::element::payload::HasPropertyInfo;
use crate::interner::interner;

/// `HasMethod(m₁) ∧ HasMethod(m₂)` → a single `HasMethod` with the
/// other conjunct stored on the intersection list. Same shape mago
/// uses, so the comparator already knows how to read it.
pub(in crate::meet) fn has_method_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_has_method(a);
    let b_info = *i.get_has_method(b);
    if a_info.method_name == b_info.method_name && a_info.intersections.is_none() && b_info.intersections.is_none() {
        return Some(a);
    }

    let mut participants: Vec<ElementId> = collect_has_method_conjuncts(a, a_info);
    participants.extend(collect_has_method_conjuncts(b, b_info));
    participants.sort();
    participants.dedup();

    let head_elem = participants.remove(0);
    let head_info = *i.get_has_method(head_elem);
    let intersections = if participants.is_empty() { None } else { Some(i.intern_element_list(&participants)) };
    Some(i.intern_has_method(HasMethodInfo { method_name: head_info.method_name, intersections }))
}

/// `HasMethod(m) ∧ HasProperty(p)` → a `HasMethod` carrying the
/// has-property as an extra conjunct. The two predicates are
/// orthogonal — a class can declare both — so the meet composes
/// rather than collapsing.
pub(in crate::meet) fn has_method_property_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let (method_atom, property_atom) =
        if a.kind() == crate::ElementKind::HasMethod { (a, b) } else { (b, a) };
    let method_info = *i.get_has_method(method_atom);
    let property_info = *i.get_has_property(property_atom);

    let mut participants: Vec<ElementId> = collect_has_method_conjuncts(method_atom, method_info);
    let property_head = i.intern_has_property(HasPropertyInfo {
        property_name: property_info.property_name,
        intersections: None,
    });
    participants.push(property_head);
    if let Some(id) = property_info.intersections {
        participants.extend_from_slice(i.get_element_list(id));
    }
    participants.sort();
    participants.dedup();

    // Pick the canonical-smallest method as head so the operation is
    // commutative; non-method conjuncts (incl. the has-property) are
    // appended to the intersection list.
    let mut method_parts: Vec<ElementId> = Vec::new();
    let mut other_parts: Vec<ElementId> = Vec::new();
    for elem in participants {
        if elem.kind() == crate::ElementKind::HasMethod {
            method_parts.push(elem);
        } else {
            other_parts.push(elem);
        }
    }
    let head = method_parts.remove(0);
    let head_info = *i.get_has_method(head);
    let mut conjuncts = method_parts;
    conjuncts.extend(other_parts);
    let intersections = if conjuncts.is_empty() { None } else { Some(i.intern_element_list(&conjuncts)) };
    Some(i.intern_has_method(HasMethodInfo { method_name: head_info.method_name, intersections }))
}

/// `HasProperty(p₁) ∧ HasProperty(p₂)` — same structure as has-method.
pub(in crate::meet) fn has_property_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_has_property(a);
    let b_info = *i.get_has_property(b);
    if a_info.property_name == b_info.property_name && a_info.intersections.is_none() && b_info.intersections.is_none()
    {
        return Some(a);
    }

    let mut participants: Vec<ElementId> = collect_has_property_conjuncts(a, a_info);
    participants.extend(collect_has_property_conjuncts(b, b_info));
    participants.sort();
    participants.dedup();

    let head_elem = participants.remove(0);
    let head_info = *i.get_has_property(head_elem);
    let intersections = if participants.is_empty() { None } else { Some(i.intern_element_list(&participants)) };
    Some(i.intern_has_property(HasPropertyInfo { property_name: head_info.property_name, intersections }))
}

fn collect_has_method_conjuncts(elem: ElementId, info: HasMethodInfo) -> Vec<ElementId> {
    let i = interner();
    let mut out: Vec<ElementId> = Vec::new();
    let head = i.intern_has_method(HasMethodInfo { method_name: info.method_name, intersections: None });
    out.push(head);
    if let Some(id) = info.intersections {
        out.extend_from_slice(i.get_element_list(id));
    }

    let _ = elem;
    out
}

fn collect_has_property_conjuncts(elem: ElementId, info: HasPropertyInfo) -> Vec<ElementId> {
    let i = interner();
    let mut out: Vec<ElementId> = Vec::new();
    let head = i.intern_has_property(HasPropertyInfo { property_name: info.property_name, intersections: None });
    out.push(head);
    if let Some(id) = info.intersections {
        out.extend_from_slice(i.get_element_list(id));
    }

    let _ = elem;
    out
}
