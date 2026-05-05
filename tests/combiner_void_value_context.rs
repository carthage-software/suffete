//! `void` is observationally equal to `null` at runtime: a function
//! declared `: void` returns the value `null` to its callers. The
//! distinction is purely declarative (a `void` return forbids
//! `return $expr` in the body). In every value-flow context (unions
//! produced by match arms, ternary branches, conditional expressions,
//! etc.) `void` must be canonicalised to `null`.

#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]

mod combiner_common;

use combiner_common::*;

#[test]
fn void_alone_is_preserved_as_return_type_annotation() {
    assert_combines_to(vec![void()], vec![void()]);
}

#[test]
fn void_or_never_keeps_void() {
    assert_combines_to(vec![void(), never()], vec![void()]);
    assert_combines_to(vec![never(), void()], vec![void()]);
}

#[test]
fn void_with_only_nevers_keeps_void() {
    assert_combines_to(vec![void(), never(), never()], vec![void()]);
}

#[test]
fn void_or_null_collapses_to_null() {
    assert_combines_to(vec![void(), null()], vec![null()]);
    assert_combines_to(vec![null(), void()], vec![null()]);
}

#[test]
fn true_or_void_should_be_true_or_null() {
    assert_combines_to(vec![t_true(), void()], vec![t_true(), null()]);
    assert_combines_to(vec![void(), t_true()], vec![t_true(), null()]);
}

#[test]
fn false_or_void_should_be_false_or_null() {
    assert_combines_to(vec![t_false(), void()], vec![t_false(), null()]);
}

#[test]
fn bool_or_void_should_be_bool_or_null() {
    assert_combines_to(vec![t_bool(), void()], vec![t_bool(), null()]);
}

#[test]
fn int_or_void_should_be_int_or_null() {
    assert_combines_to(vec![t_int(), void()], vec![t_int(), null()]);
    assert_combines_to(vec![void(), t_int()], vec![t_int(), null()]);
}

#[test]
fn string_or_void_should_be_string_or_null() {
    assert_combines_to(vec![t_string(), void()], vec![t_string(), null()]);
}

#[test]
fn object_or_void_should_be_object_or_null() {
    assert_combines_to(vec![t_object_any(), void()], vec![t_object_any(), null()]);
    assert_combines_to(vec![t_named("Foo"), void()], vec![t_named("Foo"), null()]);
}

#[test]
fn resource_or_void_should_be_resource_or_null() {
    assert_combines_to(vec![t_resource(), void()], vec![t_resource(), null()]);
}

#[test]
fn literal_or_void_should_be_literal_or_null() {
    assert_combines_to(vec![t_lit_int(42), void()], vec![t_lit_int(42), null()]);
    assert_combines_to(vec![t_lit_string("hello"), void()], vec![t_lit_string("hello"), null()]);
}

#[test]
fn three_way_int_string_void_should_have_null_not_void() {
    let r = combine_default(vec![void(), t_int(), t_string()]);
    let mut sorted = r;
    sorted.sort();
    let mut expected = vec![t_int(), t_string(), null()];
    expected.sort();
    assert_eq!(sorted, expected);
}

#[test]
fn void_with_multiple_others_is_replaced_by_single_null() {
    let r = combine_default(vec![void(), t_int(), t_string(), t_bool()]);
    let null_count = r.iter().filter(|e| **e == null()).count();
    let void_count = r.iter().filter(|e| **e == void()).count();
    assert_eq!(null_count, 1, "should be exactly one null in the result");
    assert_eq!(void_count, 0, "should be no void in the result when value-types are present");
}
