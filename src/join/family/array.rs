#![allow(clippy::arithmetic_side_effects)]

//! Array-family join: keyed-array param merge, shape merge, shape
//! collapse, empty-array overwrite, and int-keyed → list rewrite.

use core::num::NonZeroU32;

use crate::ElementId;
use crate::ElementKind;
use crate::TypeId;
use crate::element::payload::ArrayKey;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownElementEntry;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::interner::interner;
use crate::prelude::EMPTY_ARRAY;
use crate::prelude::TYPE_ARRAY_KEY;
use crate::prelude::TYPE_MIXED;
use crate::prelude::TYPE_NEVER;

/// Merge multiple unsealed keyed arrays with the same `non_empty` flag
/// into a single keyed array with unioned key+value parameters. Sealed
/// keyed arrays (with `known_items`) are left to
/// [`apply_merge_array_shapes`].
pub(in crate::join) fn apply_merge_keyed_array_params(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut groups: std::collections::HashMap<bool, Vec<usize>> = std::collections::HashMap::default();
    for (idx, &el) in elements.iter().enumerate() {
        if el.kind() != ElementKind::Array {
            continue;
        }
        let info = *i.get_array(el);
        if info.known_items.is_some() || info.key_param.is_none() || info.value_param.is_none() {
            continue;
        }
        groups.entry(info.flags.non_empty()).or_default().push(idx);
    }

    let mut to_remove: alloc::collections::BTreeSet<usize> = alloc::collections::BTreeSet::default();
    for (non_empty, indices) in &groups {
        if indices.len() < 2 {
            continue;
        }
        let mut key_elements: Vec<ElementId> = Vec::new();
        let mut value_elements: Vec<ElementId> = Vec::new();
        for &idx in indices {
            let info = *i.get_array(elements[idx]);
            if let (Some(kp), Some(vp)) = (info.key_param, info.value_param) {
                key_elements.extend_from_slice(kp.as_ref().elements);
                value_elements.extend_from_slice(vp.as_ref().elements);
            }
        }
        let key_canon = super::super::compute(&key_elements);
        let value_canon = super::super::compute(&value_elements);
        let key_ty = i.intern_type(&key_canon, crate::FlowFlags::EMPTY);
        let value_ty = i.intern_type(&value_canon, crate::FlowFlags::EMPTY);
        let merged_array = ElementId::keyed_unsealed(key_ty, value_ty, *non_empty);
        elements[indices[0]] = merged_array;
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

/// When the union has more than `threshold` array shapes, replace
/// them all with the general `array<array-key, mixed>` form.
pub(in crate::join) fn apply_array_shape_collapse(elements: &mut Vec<ElementId>, threshold: u16) {
    let shape_count = elements
        .iter()
        .filter(|e| matches!(e.kind(), ElementKind::Array | ElementKind::List) && **e != EMPTY_ARRAY)
        .count();

    if shape_count as u32 <= u32::from(threshold) {
        return;
    }

    let i = interner();
    elements.retain(|e| !(matches!(e.kind(), ElementKind::Array | ElementKind::List) && *e != EMPTY_ARRAY));
    let general = i.intern_array(KeyedArrayInfo {
        key_param: Some(TYPE_ARRAY_KEY),
        value_param: Some(TYPE_MIXED),
        known_items: None,
        intersections: None,
        flags: KeyedArrayFlags::default(),
    });

    let pos = elements.binary_search(&general).unwrap_or_else(|p| p);
    elements.insert(pos, general);
}

/// Drop `EMPTY_ARRAY` from the union when another `Array` or `List`
/// atom is present.
pub(in crate::join) fn apply_overwrite_empty_array(elements: &mut Vec<ElementId>) {
    if !crate::element::simd::any_of_kind(elements, ElementKind::Array)
        && !crate::element::simd::any_of_kind(elements, ElementKind::List)
    {
        return;
    }

    let has_other_array =
        elements.iter().any(|e| *e != EMPTY_ARRAY && matches!(e.kind(), ElementKind::Array | ElementKind::List));
    if has_other_array {
        elements.retain(|e| *e != EMPTY_ARRAY);
    }
}

/// Detect keyed-array atoms whose `known_items` use contiguous integer
/// keys `0..n-1` (and whose key/value rest types are absent or
/// list-compatible) and rewrite them as `List` atoms.
pub(in crate::join) fn apply_rewrite_int_keyed_to_list(elements: &mut [ElementId]) {
    let i = interner();
    for el in elements.iter_mut() {
        if el.kind() != ElementKind::Array {
            continue;
        }
        let info = *i.get_array(*el);
        if info.key_param.is_some() {
            continue;
        }
        let Some(known_id) = info.known_items else {
            continue;
        };
        let entries = i.get_known_items(known_id);
        let mut indexed: Vec<(i64, &KnownItemEntry)> = Vec::with_capacity(entries.len());
        let mut all_int = true;
        for entry in entries {
            match entry.key {
                ArrayKey::Int(n) => indexed.push((n, entry)),
                _ => {
                    all_int = false;
                    break;
                }
            }
        }
        if !all_int {
            continue;
        }
        indexed.sort_by_key(|(n, _)| *n);
        if !(0..indexed.len()).all(|idx| indexed[idx].0 == idx as i64) {
            continue;
        }

        let known_elements: Vec<KnownElementEntry> = indexed
            .iter()
            .map(|(n, entry)| KnownElementEntry { index: *n as u32, value: entry.value, optional: entry.optional })
            .collect();
        let known_count = NonZeroU32::new(known_elements.len() as u32);
        let list_info = ListInfo {
            element_type: info.value_param.unwrap_or(TYPE_NEVER),
            known_elements: Some(i.intern_known_elements(&known_elements)),
            known_count,
            intersections: None,
            flags: ListFlags::default().with_non_empty(info.flags.non_empty()),
        };
        *el = i.intern_list(list_info);
    }
}

/// When the union contains multiple keyed-array atoms that share at
/// least one literal key, fold them into a single shape whose value
/// at every shared key is the union of the source values.
pub(in crate::join) fn apply_merge_array_shapes(elements: &mut Vec<ElementId>) {
    let i = interner();
    let mut shapes: Vec<usize> = elements
        .iter()
        .enumerate()
        .filter_map(|(idx, el)| {
            (el.kind() == ElementKind::Array && i.get_array(*el).known_items.is_some()).then_some(idx)
        })
        .collect();

    if shapes.len() < 2 {
        return;
    }

    let head_idx = shapes.remove(0);
    let head_info = *i.get_array(elements[head_idx]);
    let Some(head_known_id) = head_info.known_items else { return };
    let mut new_known: Vec<KnownItemEntry> = i.get_known_items(head_known_id).to_vec();
    let mut absorbed: Vec<usize> = Vec::new();
    let mut accumulated_non_empty = head_info.flags.non_empty();

    for &shape_idx in &shapes {
        let other = *i.get_array(elements[shape_idx]);
        if other.key_param != head_info.key_param || other.value_param != head_info.value_param {
            continue;
        }
        let Some(other_known_id) = other.known_items else { continue };
        let other_entries = i.get_known_items(other_known_id);
        let shares_key = other_entries.iter().any(|o| new_known.iter().any(|e| e.key == o.key));
        if !shares_key {
            continue;
        }

        for o_entry in other_entries {
            if let Some(existing) = new_known.iter_mut().find(|e| e.key == o_entry.key) {
                let mut elems: Vec<ElementId> = existing.value.as_ref().elements.to_vec();
                elems.extend_from_slice(o_entry.value.as_ref().elements);
                existing.value = TypeId::union(&elems);
                existing.optional = existing.optional || o_entry.optional;
            } else {
                new_known.push(*o_entry);
            }
        }
        accumulated_non_empty = accumulated_non_empty || other.flags.non_empty();
        absorbed.push(shape_idx);
    }

    if absorbed.is_empty() {
        return;
    }

    new_known.sort_by_key(|e| e.key);
    let merged_info = KeyedArrayInfo {
        known_items: Some(i.intern_known_items(&new_known)),
        flags: KeyedArrayFlags::default().with_non_empty(accumulated_non_empty),
        ..head_info
    };
    elements[head_idx] = i.intern_array(merged_info);

    let mut absorbed_set: alloc::collections::BTreeSet<usize> = absorbed.into_iter().collect();
    let mut idx = 0;
    elements.retain(|_| {
        let keep = !absorbed_set.remove(&idx);
        idx += 1;
        keep
    });
}
