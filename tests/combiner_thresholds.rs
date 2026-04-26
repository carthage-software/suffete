//! Threshold-based literal collapse. Suffete does not implement this yet:
//! the `combine_with_*_threshold` helpers in `combiner_common` are stubs
//! that ignore the threshold and just call the default combiner. Every
//! test here is `#[ignore]`'d until the feature lands.

mod combiner_common;

use combiner_common::*;

#[test]
fn at_default_int_threshold_keeps_literals() {
    // This case happens to be threshold-independent: 128 distinct literals
    // are kept whether or not the threshold is enforced.
    let n = 128_usize;
    let inputs: Vec<_> = (0..n).map(|i| t_lit_int(i as i64)).collect();
    let result = combine_default(inputs);
    assert_eq!(result.len(), n);
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn just_over_default_int_threshold_generalises() {
    let n = 129_usize;
    let inputs: Vec<_> = (0..n).map(|i| t_lit_int(i as i64)).collect();
    let result = combine_default(inputs);
    assert_eq!(result, vec![t_int()]);
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn many_int_thresholds_walk() {
    for threshold in [1_u16, 2, 5, 10, 32, 64, 100, 128] {
        let inputs: Vec<_> = (0..200_i64).map(t_lit_int).collect();
        let result = combine_with_int_threshold(inputs, threshold);
        assert_eq!(result, vec![t_int()]);
    }
}

#[test]
fn int_threshold_above_input_count_keeps_literals() {
    let inputs: Vec<_> = (0..50_i64).map(t_lit_int).collect();
    let result = combine_with_int_threshold(inputs, 100);
    assert_eq!(result.len(), 50);
}

#[test]
fn at_default_string_threshold_keeps_literals() {
    let n = 128_usize;
    let inputs: Vec<_> = (0..n).map(|i| t_lit_string(&format!("s{i}"))).collect();
    let result = combine_default(inputs);
    assert_eq!(result.len(), n);
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn just_over_default_string_threshold_generalises() {
    let n = 129_usize;
    let inputs: Vec<_> = (0..n).map(|i| t_lit_string(&format!("s{i}"))).collect();
    let result = combine_default(inputs);
    assert_eq!(result, vec![t_string()]);
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn many_string_thresholds_walk() {
    for threshold in [1_u16, 2, 5, 10, 32, 64, 100, 128] {
        let inputs: Vec<_> = (0..200_usize).map(|i| t_lit_string(&format!("s{i}"))).collect();
        let result = combine_with_string_threshold(inputs, threshold);
        assert_eq!(result, vec![t_string()]);
    }
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn just_over_default_float_threshold_generalises() {
    let n = 129_usize;
    let inputs: Vec<_> = (0..n).map(|i| t_lit_float(i as f64)).collect();
    let result = combine_default(inputs);
    assert_eq!(result, vec![t_float()]);
}

#[test]
#[ignore = "needs t_sealed_list helper + threshold-based array collapse"]
fn many_distinct_sealed_lists_above_threshold_collapse() {}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn integer_threshold_zero_with_two_inputs_generalises() {
    let inputs = vec![t_lit_int(1), t_lit_int(2)];
    let result = combine_with_int_threshold(inputs, 0);
    assert_eq!(result, vec![t_int()]);
}

#[test]
#[ignore = "needs threshold-based literal collapse"]
fn string_threshold_zero_with_two_inputs_generalises() {
    let inputs = vec![t_lit_string("a"), t_lit_string("b")];
    let result = combine_with_string_threshold(inputs, 0);
    assert_eq!(result, vec![t_string()]);
}

#[test]
#[ignore = "needs t_sealed_list helper"]
fn array_threshold_zero_with_two_inputs_generalises() {}
