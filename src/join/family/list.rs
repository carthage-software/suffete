#![allow(clippy::arithmetic_side_effects)]

//! List-family join: merge unsealed lists of the same `non_empty`
//! flag into a single list whose element type is the type-union of
//! theirs.

use crate::ElementId;
use crate::ElementKind;
use crate::interner::interner;

/// Merge multiple unsealed lists with the same `non_empty` flag into a
/// single list whose element type is the type-union of theirs. Sealed
/// lists (those with `known_elements`) and lists with differing
/// `non_empty` flags are left alone.
pub(in crate::join) fn apply_merge_list_element_types(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut groups: std::collections::HashMap<bool, (Vec<usize>, Vec<crate::ElementId>)> =
        std::collections::HashMap::default();
    for (idx, &el) in elements.iter().enumerate() {
        if el.kind() != ElementKind::List {
            continue;
        }
        let info = *i.get_list(el);
        if info.known_elements.is_some() {
            continue;
        }
        let entry = groups.entry(info.flags.non_empty()).or_default();
        entry.0.push(idx);
        entry.1.push(el);
    }

    let mut to_remove: alloc::collections::BTreeSet<usize> = alloc::collections::BTreeSet::default();
    for (non_empty, (indices, _)) in &groups {
        if indices.len() < 2 {
            continue;
        }
        let mut merged_elements: Vec<ElementId> = Vec::new();
        for &idx in indices {
            let info = *i.get_list(elements[idx]);
            merged_elements.extend_from_slice(info.element_type.as_ref().elements);
        }
        let merged = super::super::compute(&merged_elements);
        let union_ty = i.intern_type(&merged, crate::FlowFlags::EMPTY);
        let merged_list = ElementId::list(union_ty, *non_empty);
        elements[indices[0]] = merged_list;
        for &idx in &indices[1..] {
            to_remove.insert(idx);
        }
    }

    if to_remove.is_empty() {
        return;
    }
    let mut idx = 0;
    elements.retain(|_| {
        let keep = !to_remove.contains(&idx);
        idx += 1;
        keep
    });
}
