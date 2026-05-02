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

mod comparator_common;

use comparator_common::*;

#[test]
fn mixed_reflexive() {
    assert_atomic_subtype(mixed(), mixed());
}

#[test]
fn truthy_mixed_reflexive() {
    assert_atomic_subtype(mixed_truthy(), mixed_truthy());
}

#[test]
fn falsy_mixed_reflexive() {
    assert_atomic_subtype(mixed_falsy(), mixed_falsy());
}

#[test]
fn nonnull_mixed_reflexive() {
    assert_atomic_subtype(mixed_nonnull(), mixed_nonnull());
}

#[test]
fn truthy_mixed_in_mixed() {
    assert_atomic_subtype(mixed_truthy(), mixed());
}

#[test]
fn falsy_mixed_in_mixed() {
    assert_atomic_subtype(mixed_falsy(), mixed());
}

#[test]
fn nonnull_mixed_in_mixed() {
    assert_atomic_subtype(mixed_nonnull(), mixed());
}

#[test]
fn int_in_mixed() {
    assert_atomic_subtype(t_int(), mixed());
}

#[test]
fn string_in_mixed() {
    assert_atomic_subtype(t_string(), mixed());
}

#[test]
fn float_in_mixed() {
    assert_atomic_subtype(t_float(), mixed());
}

#[test]
fn bool_in_mixed() {
    assert_atomic_subtype(t_bool(), mixed());
}

#[test]
fn null_in_mixed() {
    assert_atomic_subtype(null(), mixed());
}

#[test]
fn void_in_mixed() {
    assert_atomic_subtype(void(), mixed());
}

#[test]
fn object_in_mixed() {
    assert_atomic_subtype(t_object_any(), mixed());
    assert_atomic_subtype(t_named("Foo"), mixed());
}

#[test]
fn array_in_mixed() {
    assert_atomic_subtype(t_empty_array(), mixed());
}

#[test]
fn resource_in_mixed() {
    assert_atomic_subtype(t_resource(), mixed());
}

#[test]
fn mixed_not_in_int() {
    assert_atomic_not_subtype(mixed(), t_int());
}

#[test]
fn mixed_not_in_string() {
    assert_atomic_not_subtype(mixed(), t_string());
}

#[test]
fn mixed_not_in_float() {
    assert_atomic_not_subtype(mixed(), t_float());
}

#[test]
fn mixed_not_in_bool() {
    assert_atomic_not_subtype(mixed(), t_bool());
}

#[test]
fn mixed_not_in_null() {
    assert_atomic_not_subtype(mixed(), null());
}

#[test]
fn mixed_not_in_object() {
    assert_atomic_not_subtype(mixed(), t_object_any());
}

#[test]
fn mixed_not_in_array() {
    assert_atomic_not_subtype(mixed(), t_empty_array());
}

#[test]
fn never_in_mixed_variants() {
    assert_atomic_subtype(never(), mixed());
    assert_atomic_subtype(never(), mixed_truthy());
    assert_atomic_subtype(never(), mixed_falsy());
    assert_atomic_subtype(never(), mixed_nonnull());
}

#[test]
fn null_not_in_nonnull_mixed() {
    assert_atomic_not_subtype(null(), mixed_nonnull());
}

#[test]
fn int_in_nonnull_mixed() {
    assert_atomic_subtype(t_int(), mixed_nonnull());
    assert_atomic_subtype(t_lit_int(0), mixed_nonnull());
    assert_atomic_subtype(t_lit_int(42), mixed_nonnull());
}

#[test]
fn string_in_nonnull_mixed() {
    assert_atomic_subtype(t_string(), mixed_nonnull());
    assert_atomic_subtype(t_lit_string(""), mixed_nonnull());
}

#[test]
fn object_in_nonnull_mixed() {
    assert_atomic_subtype(t_object_any(), mixed_nonnull());
    assert_atomic_subtype(t_named("Foo"), mixed_nonnull());
}

#[test]
fn vanilla_mixed_not_in_nonnull_mixed() {
    assert_atomic_not_subtype(mixed(), mixed_nonnull());
}

#[test]
fn nonnull_mixed_in_nonnull_mixed() {
    assert_atomic_subtype(mixed_nonnull(), mixed_nonnull());
}

#[test]
fn truthy_mixed_in_nonnull_mixed() {
    // Truthy excludes null, so a truthy mixed is also non-null.
    assert_atomic_subtype(mixed_truthy(), mixed_nonnull());
}

#[test]
fn true_in_truthy_mixed() {
    assert_atomic_subtype(t_true(), mixed_truthy());
}

#[test]
fn false_not_in_truthy_mixed() {
    assert_atomic_not_subtype(t_false(), mixed_truthy());
}

#[test]
fn bool_not_in_truthy_or_falsy_mixed() {
    assert_atomic_not_subtype(t_bool(), mixed_truthy());
    assert_atomic_not_subtype(t_bool(), mixed_falsy());
}

#[test]
fn null_in_falsy_mixed() {
    assert_atomic_subtype(null(), mixed_falsy());
}

#[test]
fn null_not_in_truthy_mixed() {
    assert_atomic_not_subtype(null(), mixed_truthy());
}

#[test]
fn lit_int_truthiness() {
    assert_atomic_subtype(t_lit_int(0), mixed_falsy());
    assert_atomic_not_subtype(t_lit_int(0), mixed_truthy());
    assert_atomic_subtype(t_lit_int(42), mixed_truthy());
    assert_atomic_subtype(t_lit_int(-1), mixed_truthy());
    assert_atomic_not_subtype(t_lit_int(42), mixed_falsy());
}

#[test]
fn int_general_truthiness_undetermined() {
    assert_atomic_not_subtype(t_int(), mixed_truthy());
    assert_atomic_not_subtype(t_int(), mixed_falsy());
}

#[test]
fn positive_int_in_truthy_mixed() {
    assert_atomic_subtype(t_positive_int(), mixed_truthy());
    assert_atomic_subtype(t_negative_int(), mixed_truthy());
}

#[test]
fn lit_float_truthiness() {
    assert_atomic_subtype(t_lit_float(0.0), mixed_falsy());
    assert_atomic_subtype(t_lit_float(1.5), mixed_truthy());
    assert_atomic_subtype(t_lit_float(-2.5), mixed_truthy());
}

#[test]
fn float_general_truthiness_undetermined() {
    assert_atomic_not_subtype(t_float(), mixed_truthy());
    assert_atomic_not_subtype(t_float(), mixed_falsy());
}

#[test]
fn lit_string_truthiness() {
    assert_atomic_subtype(t_lit_string(""), mixed_falsy());
    assert_atomic_subtype(t_lit_string("0"), mixed_falsy());
    assert_atomic_subtype(t_lit_string("hi"), mixed_truthy());
    assert_atomic_subtype(t_lit_string("false"), mixed_truthy());
    assert_atomic_not_subtype(t_lit_string("hi"), mixed_falsy());
    assert_atomic_not_subtype(t_lit_string(""), mixed_truthy());
}

#[test]
fn truthy_string_in_truthy_mixed() {
    assert_atomic_subtype(t_truthy_string(), mixed_truthy());
}

#[test]
fn general_string_truthiness_undetermined() {
    assert_atomic_not_subtype(t_string(), mixed_truthy());
    assert_atomic_not_subtype(t_string(), mixed_falsy());
}

#[test]
fn objects_are_truthy() {
    assert_atomic_subtype(t_object_any(), mixed_truthy());
    assert_atomic_subtype(t_named("Foo"), mixed_truthy());
    assert_atomic_not_subtype(t_object_any(), mixed_falsy());
}

#[test]
fn resources_are_truthy() {
    assert_atomic_subtype(t_resource(), mixed_truthy());
    assert_atomic_subtype(t_open_resource(), mixed_truthy());
}

#[test]
fn class_like_strings_are_truthy() {
    assert_atomic_subtype(t_class_string(), mixed_truthy());
    assert_atomic_subtype(t_lit_class_string("Foo"), mixed_truthy());
}

#[test]
fn empty_array_is_falsy() {
    assert_atomic_subtype(t_empty_array(), mixed_falsy());
    assert_atomic_not_subtype(t_empty_array(), mixed_truthy());
}

#[test]
fn non_empty_list_is_truthy() {
    let l = t_list(u(t_int()), true);
    assert_atomic_subtype(l, mixed_truthy());
}

#[test]
fn general_list_truthiness_undetermined() {
    let l = t_list(u(t_int()), false);
    assert_atomic_not_subtype(l, mixed_truthy());
    assert_atomic_not_subtype(l, mixed_falsy());
}
