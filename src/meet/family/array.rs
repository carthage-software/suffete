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
/// is non-empty the result is non-empty too. Sealed lists (with
/// `known_elements`) are deferred and yield `None` for now.
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
        return None;
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
        flags: ListFlags::default().with_non_empty(non_empty),
    };
    Some(i.intern_list(merged))
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

    let mut merged: std::collections::BTreeMap<ArrayKey, KnownItemEntry> = Default::default();
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
        flags: KeyedArrayFlags::default().with_non_empty(non_empty),
    };
    Some(i.intern_array(merged_info))
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

    if let Some(array_key_param) = array_info.key_param {
        // For the result (a list, with `int` keys) to refine the
        // array side, every concrete `int` key must satisfy the
        // array's key constraint: `int <: array_key_param`. When that
        // fails (e.g. `array<string, …>` or `array<T.A, …>`), the
        // intersection has no representable list, so we collapse
        // to `None`.
        if !crate::lattice::refines(crate::prelude::TYPE_INT, array_key_param, world, options, report) {
            return None;
        }
    }

    let array_value_param = array_info.value_param.unwrap_or(crate::prelude::TYPE_MIXED);
    let element_type = crate::meet::compute(list_info.element_type, array_value_param, world, options, report);

    let non_empty = list_info.flags.non_empty() || array_info.flags.non_empty();
    if non_empty && element_type == crate::prelude::TYPE_NEVER {
        return None;
    }

    Some(i.intern_list(ListInfo {
        element_type,
        known_elements: None,
        known_count: None,
        flags: ListFlags::default().with_non_empty(non_empty),
    }))
}

fn unsealed_keyed_array_meet<W: World>(
    a_info: KeyedArrayInfo,
    b_info: KeyedArrayInfo,
    world: &W,
    options: LatticeOptions,
    report: &mut LatticeReport,
) -> Option<ElementId> {
    let i = interner();
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();

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
        let key_empty = key_param.map(|t| t == crate::prelude::TYPE_NEVER).unwrap_or(false);
        let value_empty = value_param.map(|t| t == crate::prelude::TYPE_NEVER).unwrap_or(false);
        if key_empty || value_empty {
            return None;
        }
    }

    Some(i.intern_array(KeyedArrayInfo {
        key_param,
        value_param,
        known_items: None,
        flags: KeyedArrayFlags::default().with_non_empty(non_empty),
    }))
}
