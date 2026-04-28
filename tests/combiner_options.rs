//! `JoinOptions` extended-rule tests (report §19): each rule must fire
//! only when its toggle is on.

mod combiner_common;

use combiner_common::*;

use std::collections::BTreeMap;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::element::payload::ArrayKey;
use suffete::element::payload::KeyedArrayInfo;
use suffete::element::payload::KnownItemEntry;
use suffete::interner::interner;
use suffete::join;
use suffete::prelude;

fn t_array_with_items(items: &[(ArrayKey, TypeId)]) -> ElementId {
    let entries: Vec<KnownItemEntry> =
        items.iter().map(|(key, value)| KnownItemEntry { key: *key, value: *value, optional: false }).collect();
    let i = interner();
    i.intern_array(KeyedArrayInfo {
        key_param: None,
        value_param: None,
        known_items: Some(i.intern_known_items(&entries)),
        flags: Default::default(),
    })
}

fn t_array_with_params(key: TypeId, value: TypeId) -> ElementId {
    let i = interner();
    i.intern_array(KeyedArrayInfo {
        key_param: Some(key),
        value_param: Some(value),
        known_items: None,
        flags: Default::default(),
    })
}

fn ut(elem: ElementId) -> TypeId {
    interner().intern_type(&[elem], FlowFlags::EMPTY)
}

#[test]
fn overwrite_empty_array_drops_when_other_array_present() {
    let other = t_array_with_params(ut(t_string()), ut(t_int()));
    let opts = join::JoinOptions::default().with_overwrite_empty_array(true);
    let out = join::compute_with(&[t_empty_array(), other], &opts);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], other);
}

#[test]
fn overwrite_empty_array_keeps_when_alone() {
    let opts = join::JoinOptions::default().with_overwrite_empty_array(true);
    let out = join::compute_with(&[t_empty_array()], &opts);
    assert_eq!(out, vec![t_empty_array()]);
}

#[test]
fn overwrite_empty_array_off_keeps_both() {
    let other = t_array_with_params(ut(t_string()), ut(t_int()));
    let out = join::compute_with(&[t_empty_array(), other], &join::JoinOptions::default());
    let mut sorted = out.clone();
    sorted.sort();
    let mut expected = vec![t_empty_array(), other];
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn string_literal_collapse_fires_above_threshold() {
    let lits = (0..5).map(|n| t_lit_string(&format!("s{n}"))).collect::<Vec<_>>();
    let opts = join::JoinOptions::default().with_string_literal_collapse_threshold(3);
    let out = join::compute_with(&lits, &opts);
    assert_eq!(out, vec![prelude::STRING]);
}

#[test]
fn string_literal_collapse_at_or_below_threshold_keeps_literals() {
    let lits = (0..3).map(|n| t_lit_string(&format!("s{n}"))).collect::<Vec<_>>();
    let opts = join::JoinOptions::default().with_string_literal_collapse_threshold(3);
    let out = join::compute_with(&lits, &opts);
    let mut sorted = out.clone();
    sorted.sort();
    let mut expected = lits.clone();
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn merge_int_ranges_collapses_consecutive_literals() {
    let opts = join::JoinOptions::default().with_merge_int_ranges(true);
    let out = join::compute_with(
        &[t_lit_int(0), t_lit_int(1), t_lit_int(2), t_lit_int(3)],
        &opts,
    );
    assert_eq!(out, vec![ElementId::int_range(Some(0), Some(3))]);
}

#[test]
fn merge_int_ranges_with_gap_keeps_separate() {
    let opts = join::JoinOptions::default().with_merge_int_ranges(true);
    let out = join::compute_with(&[t_lit_int(0), t_lit_int(1), t_lit_int(5)], &opts);
    let mut sorted = out.clone();
    sorted.sort();
    let mut expected = vec![ElementId::int_range(Some(0), Some(1)), t_lit_int(5)];
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn merge_int_ranges_combines_overlapping_ranges() {
    let opts = join::JoinOptions::default().with_merge_int_ranges(true);
    let r1 = ElementId::int_range(Some(0), Some(10));
    let r2 = ElementId::int_range(Some(5), Some(15));
    let out = join::compute_with(&[r1, r2], &opts);
    assert_eq!(out, vec![ElementId::int_range(Some(0), Some(15))]);
}

#[test]
fn merge_int_ranges_off_keeps_separate() {
    let out = join::compute_with(&[t_lit_int(0), t_lit_int(1)], &join::JoinOptions::default());
    let mut sorted = out.clone();
    sorted.sort();
    let mut expected = vec![t_lit_int(0), t_lit_int(1)];
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn rewrite_int_keyed_to_list_converts_contiguous_indices() {
    let arr = t_array_with_items(&[(ArrayKey::Int(0), ut(t_int())), (ArrayKey::Int(1), ut(t_string()))]);
    let opts = join::JoinOptions::default().with_rewrite_int_keyed_to_list(true);
    let out = join::compute_with(&[arr], &opts);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].kind(), suffete::ElementKind::List);
}

#[test]
fn rewrite_int_keyed_to_list_skips_non_contiguous() {
    let arr = t_array_with_items(&[(ArrayKey::Int(0), ut(t_int())), (ArrayKey::Int(5), ut(t_string()))]);
    let opts = join::JoinOptions::default().with_rewrite_int_keyed_to_list(true);
    let out = join::compute_with(&[arr], &opts);
    assert_eq!(out, vec![arr]);
}

#[test]
fn rewrite_int_keyed_to_list_skips_string_keys() {
    let arr = t_array_with_items(&[(ArrayKey::String(name_atom("name")), ut(t_int()))]);
    let opts = join::JoinOptions::default().with_rewrite_int_keyed_to_list(true);
    let out = join::compute_with(&[arr], &opts);
    assert_eq!(out, vec![arr]);
}

#[test]
fn merge_array_shapes_combines_overlapping_keys() {
    let a = t_array_with_items(&[(ArrayKey::String(name_atom("k")), ut(t_int()))]);
    let b = t_array_with_items(&[(ArrayKey::String(name_atom("k")), ut(t_string()))]);
    let opts = join::JoinOptions::default().with_merge_array_shapes(true);
    let out = join::compute_with(&[a, b], &opts);
    assert_eq!(out.len(), 1);
    let merged = interner().get_array(out[0]);
    let entries = interner().get_known_items(merged.known_items.unwrap());
    assert_eq!(entries.len(), 1);
    let value_elements = entries[0].value.as_ref().elements;
    assert!(value_elements.contains(&t_int()));
    assert!(value_elements.contains(&t_string()));
}

#[test]
fn merge_array_shapes_skips_disjoint_keys() {
    let a = t_array_with_items(&[(ArrayKey::String(name_atom("k1")), ut(t_int()))]);
    let b = t_array_with_items(&[(ArrayKey::String(name_atom("k2")), ut(t_string()))]);
    let opts = join::JoinOptions::default().with_merge_array_shapes(true);
    let out = join::compute_with(&[a, b], &opts);
    assert_eq!(out.len(), 2);
}

#[test]
fn merge_array_shapes_off_keeps_separate() {
    let a = t_array_with_items(&[(ArrayKey::String(name_atom("k")), ut(t_int()))]);
    let b = t_array_with_items(&[(ArrayKey::String(name_atom("k")), ut(t_string()))]);
    let out = join::compute_with(&[a, b], &join::JoinOptions::default());
    assert_eq!(out.len(), 2);
}

#[test]
fn default_options_match_compute() {
    let elements = vec![t_int(), t_string(), t_lit_int(42)];
    let a = join::compute(&elements);
    let b = join::compute_with(&elements, &join::JoinOptions::default());
    assert_eq!(a, b);
}

// Suppress unused warnings on the BTreeMap import — kept for symmetry
// with other combiner tests that build sealed shapes via the keyed-
// shape helper.
#[allow(dead_code)]
fn _unused_btreemap() -> BTreeMap<ArrayKey, ()> {
    BTreeMap::new()
}
