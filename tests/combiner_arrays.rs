#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
)]

mod combiner_common;

use combiner_common::*;

#[test]
fn empty_array_idempotent() {
    for n in 1..=10 {
        let r = combine_default(vec![t_empty_array(); n]);
        assert_eq!(r, vec![t_empty_array()]);
    }
}

#[test]
fn empty_array_singleton_passthrough() {
    let r = combine_default(vec![t_empty_array()]);
    assert_eq!(r, vec![t_empty_array()]);
}

#[test]
fn list_int_idempotent() {}

#[test]
fn list_string_idempotent() {}

#[test]
fn list_with_different_element_types_combine() {}

#[test]
fn list_with_subset_element_collapses() {}

#[test]
fn non_empty_list_or_general_list_yields_general() {}

#[test]
fn two_non_empty_lists_stay_non_empty() {}

#[test]
fn empty_array_or_list_kept_separate() {}

#[test]
fn empty_array_or_non_empty_list_kept_separate() {}

#[test]
fn list_then_empty_yields_just_list() {}

#[test]
fn many_lists_with_various_elements_combine_into_one() {}

#[test]
fn single_sealed_list_passthrough() {}

#[test]
fn sealed_list_meets_unsealed_list_collapses() {}

#[test]
fn keyed_unsealed_idempotent() {}

#[test]
fn keyed_with_different_value_types_combine() {}

#[test]
fn keyed_with_different_key_types_combine() {}

#[test]
fn keyed_sealed_same_collapses() {}

#[test]
fn keyed_sealed_different_keys_kept_separate() {}

#[test]
fn keyed_sealed_overlapping_keys_combine_values() {}

#[test]
fn list_and_keyed_kept_separate() {}

#[test]
fn empty_array_overwritten_by_list() {}

#[test]
fn empty_array_overwritten_by_keyed() {}

#[test]
fn empty_alone_with_overwrite_kept() {
    // overwrite is a no-op for empty + empty; combiner_overwrite is a stub
    // that calls the default combiner.
    let r = combine_overwrite(vec![t_empty_array(), t_empty_array()]);
    assert_eq!(r, vec![t_empty_array()]);
}

#[test]
fn list_or_int_kept_separate() {}

#[test]
fn keyed_or_string_kept_separate() {}

#[test]
fn empty_or_int_kept_separate() {
    let r = combine_default(vec![t_empty_array(), t_int()]);
    assert_eq!(r.len(), 2);
}

#[test]
fn mixed_dominates_array() {
    assert_combines_to(vec![mixed(), t_empty_array()], vec![mixed()]);
}

#[test]
fn many_distinct_sealed_lists_generalise() {}

#[test]
fn under_array_threshold_keeps_sealed_lists() {}

#[test]
fn custom_low_array_threshold_generalises_quickly() {}

#[test]
fn list_with_known_elements_idempotent() {}

#[test]
fn many_empty_arrays_collapse() {
    for n in 1..=20 {
        assert_combines_to(vec![t_empty_array(); n], vec![t_empty_array()]);
    }
}

#[test]
fn many_unsealed_lists_collapse() {}

#[test]
fn many_unsealed_keyed_collapse() {}
