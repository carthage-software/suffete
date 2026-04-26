//! Most array tests are stubbed. Suffete has the [`KeyedArrayInfo`] payload
//! but no list/keyed-array constructor helpers, no shape-merging in the
//! combiner, and no `with_overwrite_empty_array` option. The blocked tests
//! are kept as `#[ignore]`'d shells so that the porting checklist matches
//! mago 1:1 and the missing features stay visible.

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
#[ignore = "needs t_list helper + list element-type combine"]
fn list_int_idempotent() {}

#[test]
#[ignore = "needs t_list helper"]
fn list_string_idempotent() {}

#[test]
#[ignore = "needs list shape merging"]
fn list_with_different_element_types_combine() {}

#[test]
#[ignore = "needs subtype-driven list element absorption"]
fn list_with_subset_element_collapses() {}

#[test]
#[ignore = "needs non-empty + general list collapse"]
fn non_empty_list_or_general_list_yields_general() {}

#[test]
#[ignore = "needs non-empty list shape preservation"]
fn two_non_empty_lists_stay_non_empty() {}

#[test]
#[ignore = "needs t_list helper"]
fn empty_array_or_list_kept_separate() {}

#[test]
#[ignore = "needs t_list helper"]
fn empty_array_or_non_empty_list_kept_separate() {}

#[test]
#[ignore = "needs subtype-driven empty_array <: list"]
fn list_then_empty_yields_just_list() {}

#[test]
#[ignore = "needs list element union combine"]
fn many_lists_with_various_elements_combine_into_one() {}

#[test]
#[ignore = "needs t_sealed_list helper"]
fn single_sealed_list_passthrough() {}

#[test]
#[ignore = "needs sealed-list <: unsealed-list collapse"]
fn sealed_list_meets_unsealed_list_collapses() {}

#[test]
#[ignore = "needs t_keyed_unsealed helper"]
fn keyed_unsealed_idempotent() {}

#[test]
#[ignore = "needs keyed-array value-type union combine"]
fn keyed_with_different_value_types_combine() {}

#[test]
#[ignore = "needs keyed-array key-type union combine"]
fn keyed_with_different_key_types_combine() {}

#[test]
#[ignore = "needs t_keyed_sealed helper"]
fn keyed_sealed_same_collapses() {}

#[test]
#[ignore = "needs t_keyed_sealed helper"]
fn keyed_sealed_different_keys_kept_separate() {}

#[test]
#[ignore = "needs sealed-keyed value union combine"]
fn keyed_sealed_overlapping_keys_combine_values() {}

#[test]
#[ignore = "needs list/keyed helpers"]
fn list_and_keyed_kept_separate() {}

#[test]
#[ignore = "needs CombinerOptions::with_overwrite_empty_array"]
fn empty_array_overwritten_by_list() {}

#[test]
#[ignore = "needs CombinerOptions::with_overwrite_empty_array"]
fn empty_array_overwritten_by_keyed() {}

#[test]
fn empty_alone_with_overwrite_kept() {
    // overwrite is a no-op for empty + empty; combiner_overwrite is a stub
    // that calls the default combiner.
    let r = combine_overwrite(vec![t_empty_array(), t_empty_array()]);
    assert_eq!(r, vec![t_empty_array()]);
}

#[test]
#[ignore = "needs t_list helper"]
fn list_or_int_kept_separate() {}

#[test]
#[ignore = "needs t_keyed_unsealed helper"]
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
#[ignore = "needs threshold-based sealed-list generalization"]
fn many_distinct_sealed_lists_generalise() {}

#[test]
#[ignore = "needs t_sealed_list helper"]
fn under_array_threshold_keeps_sealed_lists() {}

#[test]
#[ignore = "needs threshold-based array generalization"]
fn custom_low_array_threshold_generalises_quickly() {}

#[test]
#[ignore = "needs t_sealed_list helper"]
fn list_with_known_elements_idempotent() {}

#[test]
fn many_empty_arrays_collapse() {
    for n in 1..=20 {
        assert_combines_to(vec![t_empty_array(); n], vec![t_empty_array()]);
    }
}

#[test]
#[ignore = "needs t_list helper"]
fn many_unsealed_lists_collapse() {}

#[test]
#[ignore = "needs t_keyed_unsealed helper"]
fn many_unsealed_keyed_collapse() {}
