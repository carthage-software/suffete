//! `List` and unsealed `Array` (keyed) family meet rules.

use crate::ElementId;
use crate::element::payload::ArrayKey;
use crate::element::payload::KeyedArrayFlags;
use crate::element::payload::KeyedArrayInfo;
use crate::element::payload::KnownItemEntry;
use crate::element::payload::ListFlags;
use crate::element::payload::ListInfo;
use crate::interner::interner;
use crate::lattice::LatticeOptions;
use crate::lattice::LatticeReport;
use crate::world::World;

/// `list<A> ∧ list<B>` is `list<A ∧ B>` (covariant). When either side
/// is non-empty the result is non-empty too. Sealed × sealed lists
/// merge index-wise; sealed × unsealed treats the unsealed side as
/// the rest type for indices beyond the sealed prefix.
pub(in crate::meet) fn list_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);

    if a_info.known_elements.is_some() || b_info.known_elements.is_some() {
        return sealed_list_meet(a_info, b_info, world, options, report);
    }

    let element_type = crate::meet::compute(a_info.element_type, b_info.element_type, world, options, report);
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();
    if non_empty && element_type == crate::prelude::TYPE_NEVER {
        return None;
    }

    let merged = ListInfo {
        element_type,
        known_elements: None,
        known_count: None,
        intersections: merge_intersections(a_info.intersections, b_info.intersections),
        flags: ListFlags::default().with_non_empty(non_empty),
    };

    let result = i.intern_list(merged);

    if crate::lattice::overlaps::is_uninhabited(result, world) { None } else { Some(result) }
}

#[inline]
fn sealed_list_meet<W: World>(
    a_info: ListInfo,
    b_info: ListInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    use crate::element::payload::KnownElementEntry;
    let i = interner();
    let a_entries: Vec<KnownElementEntry> =
        a_info.known_elements.map(|id| i.get_known_elements(id).to_vec()).unwrap_or_default();
    let b_entries: Vec<KnownElementEntry> =
        b_info.known_elements.map(|id| i.get_known_elements(id).to_vec()).unwrap_or_default();
    let max_index = a_entries.len().max(b_entries.len());
    let mut merged: Vec<KnownElementEntry> = Vec::with_capacity(max_index);
    for idx in 0..max_index {
        let a_entry = a_entries.get(idx).copied();
        let b_entry = b_entries.get(idx).copied();
        let (value, optional) = match (a_entry, b_entry) {
            (Some(ea), Some(eb)) => {
                (crate::meet::compute(ea.value, eb.value, world, options, report), ea.optional && eb.optional)
            }
            (Some(ea), None) => {
                let bv = b_info.element_type;
                (crate::meet::compute(ea.value, bv, world, options, report), ea.optional)
            }
            (None, Some(eb)) => {
                let av = a_info.element_type;
                (crate::meet::compute(av, eb.value, world, options, report), eb.optional)
            }
            (None, None) => continue,
        };

        if !optional && value == crate::prelude::TYPE_NEVER {
            return None;
        }

        merged.push(KnownElementEntry { index: idx as u32, value, optional });
    }

    let known_elements = if merged.is_empty() { None } else { Some(i.intern_known_elements(&merged)) };
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();
    let known_count = core::num::NonZeroU32::new(merged.len() as u32);
    let element_type = crate::meet::compute(a_info.element_type, b_info.element_type, world, options, report);
    let merged_info = ListInfo {
        element_type,
        known_elements,
        known_count,
        intersections: merge_intersections(a_info.intersections, b_info.intersections),
        flags: ListFlags::default().with_non_empty(non_empty),
    };

    let result = i.intern_list(merged_info);

    if crate::lattice::overlaps::is_uninhabited(result, world) { None } else { Some(result) }
}

/// `array{...} ∧ array{...}` for two sealed shapes: the result has the
/// union of keys; values at shared keys are met. Optional flags AND-merge
/// (a key is required iff it's required on both sides). Unsealed × unsealed
/// composes the open key/value parameters pointwise.
pub(in crate::meet) fn keyed_array_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);

    let (Some(a_known_id), Some(b_known_id)) = (a_info.known_items, b_info.known_items) else {
        return unsealed_keyed_array_meet(a_info, b_info, world, options, report);
    };

    let a_entries = i.get_known_items(a_known_id);
    let b_entries = i.get_known_items(b_known_id);

    let mut merged: alloc::collections::BTreeMap<ArrayKey, KnownItemEntry> = alloc::collections::BTreeMap::default();
    for entry in a_entries {
        merged.insert(entry.key, *entry);
    }

    for b_entry in b_entries {
        merged
            .entry(b_entry.key)
            .and_modify(|existing| {
                let value = crate::meet::compute(existing.value, b_entry.value, world, options, report);
                existing.value = value;
                existing.optional = existing.optional && b_entry.optional;
            })
            .or_insert(*b_entry);
    }

    let entries: Vec<KnownItemEntry> = merged.into_values().collect();
    let known_items = i.intern_known_items(&entries);
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();
    let merged_info = KeyedArrayInfo {
        key_param: None,
        value_param: None,
        known_items: Some(known_items),
        intersections: merge_intersections(a_info.intersections, b_info.intersections),
        flags: KeyedArrayFlags::default().with_non_empty(non_empty),
    };

    let result = i.intern_array(merged_info);

    if crate::lattice::overlaps::is_uninhabited(result, world) { None } else { Some(result) }
}

/// Concatenate two intersection-conjunct lists, deduplicating. Result
/// is `None` when both inputs are `None`, the single non-`None` input
/// when only one is set, and the merged-and-deduped list otherwise.
#[inline]
fn merge_intersections(
    a: Option<crate::ElementListId>,
    b: Option<crate::ElementListId>,
) -> Option<crate::ElementListId> {
    match (a, b) {
        (None, None) => None,
        (Some(id), None) | (None, Some(id)) => Some(id),
        (Some(a_id), Some(b_id)) if a_id == b_id => Some(a_id),
        (Some(a_id), Some(b_id)) => {
            let i = interner();
            let mut merged: Vec<ElementId> = i.get_element_list(a_id).to_vec();
            merged.extend_from_slice(i.get_element_list(b_id));
            merged.sort_unstable();
            merged.dedup();
            Some(i.intern_element_list(&merged))
        }
    }
}

/// `list<E> ∧ array<K, V>`: a list is an int-keyed array, so the meet
/// is a list whose element type is `E ∧ V` and whose key constraint
/// must be compatible with `int`. When `K` excludes integers
/// (e.g. `string`), the intersection is empty. The non-empty flag
/// OR-merges; an empty result on either axis collapses to `None`
/// when the result is forced non-empty.
pub(in crate::meet) fn list_array_meet<W: World>(
    a: ElementId,
    b: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let (list_atom, array_atom) = if a.kind() == crate::ElementKind::List { (a, b) } else { (b, a) };
    let list_info = *i.get_list(list_atom);
    let array_info = *i.get_array(array_atom);

    if list_info.known_elements.is_some() || array_info.known_items.is_some() {
        return None;
    }

    let non_empty = list_info.flags.non_empty() || array_info.flags.non_empty();

    let array_is_sealed_empty =
        array_info.key_param.is_none() && array_info.value_param.is_none() && array_info.known_items.is_none();
    if array_is_sealed_empty {
        return if list_info.flags.non_empty() { None } else { Some(crate::prelude::EMPTY_ARRAY) };
    }

    let key_compatible = array_info
        .key_param
        .is_none_or(|kp| crate::lattice::refines(crate::prelude::TYPE_INT, kp, world, options, report));

    if non_empty && !key_compatible {
        return None;
    }

    let array_value_param = array_info.value_param.unwrap_or(crate::prelude::TYPE_MIXED);
    let element_type = crate::meet::compute(list_info.element_type, array_value_param, world, options, report);

    if non_empty && element_type == crate::prelude::TYPE_NEVER {
        return None;
    }

    if !key_compatible {
        return Some(crate::prelude::EMPTY_ARRAY);
    }

    Some(i.intern_list(ListInfo {
        element_type,
        known_elements: None,
        known_count: None,
        intersections: None,
        flags: ListFlags::default().with_non_empty(non_empty),
    }))
}

/// `iterable<K, V> ∧ array<K', V', items?>` narrows the array's
/// key/value parameters with the iterable's. The result is still an
/// array shape (the array is the more refined family member); the
/// iterable side is consumed entirely. `known_items` value types
/// also narrow against the iterable's value type.
pub(in crate::meet) fn iterable_array_meet<W: World>(
    iterable: ElementId,
    array: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let it_info = *i.get_iterable(iterable);
    let arr_info = *i.get_array(array);

    let key_param = match arr_info.key_param {
        Some(ak) => Some(crate::meet::compute(ak, it_info.key_type, world, options, report)),
        None => Some(it_info.key_type),
    };

    let value_param = match arr_info.value_param {
        Some(av) => Some(crate::meet::compute(av, it_info.value_type, world, options, report)),
        None => Some(it_info.value_type),
    };

    let known_items = match arr_info.known_items {
        None => None,
        Some(id) => {
            let entries = i.get_known_items(id);
            let mut narrowed: Vec<KnownItemEntry> = Vec::with_capacity(entries.len());
            for entry in entries {
                let value = crate::meet::compute(entry.value, it_info.value_type, world, options, report);
                if value == crate::prelude::TYPE_NEVER && !entry.optional {
                    return None;
                }

                narrowed.push(KnownItemEntry { value, ..*entry });
            }

            Some(i.intern_known_items(&narrowed))
        }
    };

    if arr_info.flags.non_empty() {
        let key_empty = key_param.is_some_and(|t| t == crate::prelude::TYPE_NEVER);
        let value_empty = value_param.is_some_and(|t| t == crate::prelude::TYPE_NEVER);
        if key_empty || value_empty {
            return None;
        }
    }

    Some(i.intern_array(KeyedArrayInfo {
        key_param,
        value_param,
        known_items,
        intersections: None,
        flags: arr_info.flags,
    }))
}

/// `iterable<K, V> ∧ list<E>` narrows the list's element type by the
/// iterable's value type. A list has implicit `int` keys, so the
/// non-empty intersection only inhabits values when `int <: K`. When
/// `int` doesn't fit `K`, the lattice has no representation that
/// refines both sides structurally (the empty list `{[]}` is the
/// only shared value, but `list<never>` doesn't refine
/// `iterable<int(0), V>` because list keys are still `int`), so
/// the meet conservatively returns `None`. The matching overlap
/// rule reports the same to keep the lattice consistent.
pub(in crate::meet) fn iterable_list_meet<W: World>(
    iterable: ElementId,
    list: ElementId,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let it_info = *i.get_iterable(iterable);
    let list_info = *i.get_list(list);

    if !crate::lattice::refines(crate::prelude::TYPE_INT, it_info.key_type, world, options, report) {
        return None;
    }

    let element_type = crate::meet::compute(list_info.element_type, it_info.value_type, world, options, report);
    if list_info.flags.non_empty() && element_type == crate::prelude::TYPE_NEVER {
        return None;
    }

    Some(i.intern_list(ListInfo { element_type, ..list_info }))
}

#[inline]
fn unsealed_keyed_array_meet<W: World>(
    a_info: KeyedArrayInfo,
    b_info: KeyedArrayInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();

    // Sealed-empty (`[]`, no params, no known items) on either side
    // means the only inhabitant is the empty array. Meeting with a
    // non-empty side rules it out; otherwise the result is exactly
    // `[]` regardless of the other side's open key/value params.
    let a_sealed_empty = a_info.is_sealed() && a_info.known_items.is_none();
    let b_sealed_empty = b_info.is_sealed() && b_info.known_items.is_none();
    if a_sealed_empty || b_sealed_empty {
        return if non_empty { None } else { Some(crate::prelude::EMPTY_ARRAY) };
    }

    let key_param = match (a_info.key_param, b_info.key_param) {
        (Some(ak), Some(bk)) => Some(crate::meet::compute(ak, bk, world, options, report)),
        (Some(k), None) | (None, Some(k)) => Some(k),
        (None, None) => None,
    };

    let value_param = match (a_info.value_param, b_info.value_param) {
        (Some(av), Some(bv)) => Some(crate::meet::compute(av, bv, world, options, report)),
        (Some(v), None) | (None, Some(v)) => Some(v),
        (None, None) => None,
    };

    if non_empty {
        let key_empty = key_param.is_some_and(|t| t == crate::prelude::TYPE_NEVER);
        let value_empty = value_param.is_some_and(|t| t == crate::prelude::TYPE_NEVER);
        if key_empty || value_empty {
            return None;
        }
    }

    Some(i.intern_array(KeyedArrayInfo {
        key_param,
        value_param,
        known_items: None,
        intersections: None,
        flags: KeyedArrayFlags::default().with_non_empty(non_empty),
    }))
}
