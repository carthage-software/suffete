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
use crate::world::NullWorld;

/// `list<A> ∧ list<B>` is `list<A ∧ B>` (covariant). When either side
/// is non-empty the result is non-empty too. Sealed lists (with
/// `known_elements`) are deferred and yield `None` for now.
pub(in crate::meet) fn list_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_list(a);
    let b_info = *i.get_list(b);

    if a_info.known_elements.is_some() || b_info.known_elements.is_some() {
        return None;
    }

    let mut report = LatticeReport::new();
    let element_type = crate::meet::compute(
        a_info.element_type,
        b_info.element_type,
        &NullWorld,
        LatticeOptions::default(),
        &mut report,
    );
    let non_empty = a_info.flags.non_empty() || b_info.flags.non_empty();
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
/// (a key is required iff it's required on both sides).
pub(in crate::meet) fn keyed_array_meet(a: ElementId, b: ElementId) -> Option<ElementId> {
    let i = interner();
    let a_info = *i.get_array(a);
    let b_info = *i.get_array(b);

    let (Some(a_known_id), Some(b_known_id)) = (a_info.known_items, b_info.known_items) else {
        return None;
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
                let mut report = LatticeReport::new();
                let value = crate::meet::compute(
                    existing.value,
                    b_entry.value,
                    &NullWorld,
                    LatticeOptions::default(),
                    &mut report,
                );
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
