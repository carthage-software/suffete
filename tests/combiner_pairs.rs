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
use suffete::ElementId;

fn check(label: &str, input: Vec<ElementId>, expected: &[ElementId]) {
    let result = combine_default(input);
    let mut actual = result.clone();
    actual.sort();
    let mut expected_sorted = expected.to_vec();
    expected_sorted.sort();
    assert_eq!(actual, expected_sorted, "case `{label}` failed: got {result:?}, expected {expected:?}");
}

#[test]
fn primitive_pairs_int() {
    check("int | int", vec![t_int(), t_int()], &[t_int()]);
    check("int | string", vec![t_int(), t_string()], &[t_int(), t_string()]);
    check("int | float", vec![t_int(), t_float()], &[t_float(), t_int()]);
    check("int | bool", vec![t_int(), t_bool()], &[t_bool(), t_int()]);
    check("int | true", vec![t_int(), t_true()], &[t_int(), t_true()]);
    check("int | false", vec![t_int(), t_false()], &[t_false(), t_int()]);
    check("int | null", vec![t_int(), null()], &[t_int(), null()]);
    check("int | object", vec![t_int(), t_object_any()], &[t_int(), t_object_any()]);
    check("int | Foo", vec![t_int(), t_named("Foo")], &[t_named("Foo"), t_int()]);
    check("int | resource", vec![t_int(), t_resource()], &[t_int(), t_resource()]);
    check("int | array{}", vec![t_int(), t_empty_array()], &[t_empty_array(), t_int()]);
    check("int | class-string", vec![t_int(), t_class_string()], &[t_class_string(), t_int()]);
    check("int | never", vec![t_int(), never()], &[t_int()]);
    check("int | void", vec![t_int(), void()], &[t_int(), void()]);
    check("int | mixed", vec![t_int(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_string() {
    check("string | string", vec![t_string(), t_string()], &[t_string()]);
    check("string | int", vec![t_string(), t_int()], &[t_int(), t_string()]);
    check("string | float", vec![t_string(), t_float()], &[t_float(), t_string()]);
    check("string | bool", vec![t_string(), t_bool()], &[t_bool(), t_string()]);
    check("string | null", vec![t_string(), null()], &[null(), t_string()]);
    check("string | object", vec![t_string(), t_object_any()], &[t_object_any(), t_string()]);
    check("string | Foo", vec![t_string(), t_named("Foo")], &[t_named("Foo"), t_string()]);
    check("string | resource", vec![t_string(), t_resource()], &[t_resource(), t_string()]);
    check("string | array{}", vec![t_string(), t_empty_array()], &[t_empty_array(), t_string()]);
    check("string | class-string", vec![t_string(), t_class_string()], &[t_string()]);
    check("string | never", vec![t_string(), never()], &[t_string()]);
    check("string | void", vec![t_string(), void()], &[t_string(), void()]);
    check("string | mixed", vec![t_string(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_float() {
    check("float | float", vec![t_float(), t_float()], &[t_float()]);
    check("float | int", vec![t_float(), t_int()], &[t_float(), t_int()]);
    check("float | string", vec![t_float(), t_string()], &[t_float(), t_string()]);
    check("float | bool", vec![t_float(), t_bool()], &[t_bool(), t_float()]);
    check("float | null", vec![t_float(), null()], &[t_float(), null()]);
    check("float | object", vec![t_float(), t_object_any()], &[t_float(), t_object_any()]);
    check("float | resource", vec![t_float(), t_resource()], &[t_float(), t_resource()]);
    check("float | never", vec![t_float(), never()], &[t_float()]);
    check("float | void", vec![t_float(), void()], &[t_float(), void()]);
    check("float | mixed", vec![t_float(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_bool() {
    check("bool | bool", vec![t_bool(), t_bool()], &[t_bool()]);
    check("bool | int", vec![t_bool(), t_int()], &[t_bool(), t_int()]);
    check("bool | string", vec![t_bool(), t_string()], &[t_bool(), t_string()]);
    check("bool | float", vec![t_bool(), t_float()], &[t_bool(), t_float()]);
    check("bool | null", vec![t_bool(), null()], &[t_bool(), null()]);
    check("bool | object", vec![t_bool(), t_object_any()], &[t_bool(), t_object_any()]);
    check("bool | resource", vec![t_bool(), t_resource()], &[t_bool(), t_resource()]);
    check("bool | true", vec![t_bool(), t_true()], &[t_bool()]);
    check("bool | false", vec![t_bool(), t_false()], &[t_bool()]);
    check("bool | array{}", vec![t_bool(), t_empty_array()], &[t_empty_array(), t_bool()]);
    check("bool | never", vec![t_bool(), never()], &[t_bool()]);
    check("bool | void", vec![t_bool(), void()], &[t_bool(), void()]);
    check("bool | mixed", vec![t_bool(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_null() {
    check("null | null", vec![null(), null()], &[null()]);
    check("null | int", vec![null(), t_int()], &[t_int(), null()]);
    check("null | string", vec![null(), t_string()], &[null(), t_string()]);
    check("null | float", vec![null(), t_float()], &[t_float(), null()]);
    check("null | bool", vec![null(), t_bool()], &[t_bool(), null()]);
    check("null | object", vec![null(), t_object_any()], &[null(), t_object_any()]);
    check("null | Foo", vec![null(), t_named("Foo")], &[t_named("Foo"), null()]);
    check("null | resource", vec![null(), t_resource()], &[null(), t_resource()]);
    check("null | array{}", vec![null(), t_empty_array()], &[t_empty_array(), null()]);
    check("null | class-string", vec![null(), t_class_string()], &[t_class_string(), null()]);
    check("null | never", vec![null(), never()], &[null()]);
    check("null | void", vec![null(), void()], &[null()]);
    check("null | mixed", vec![null(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_void() {
    check("void | void", vec![void(), void()], &[void()]);
    check("void | int", vec![void(), t_int()], &[t_int(), void()]);
    check("void | string", vec![void(), t_string()], &[t_string(), void()]);
    check("void | float", vec![void(), t_float()], &[t_float(), void()]);
    check("void | bool", vec![void(), t_bool()], &[t_bool(), void()]);
    check("void | null", vec![void(), null()], &[null()]);
    check("void | object", vec![void(), t_object_any()], &[t_object_any(), void()]);
    check("void | Foo", vec![void(), t_named("Foo")], &[t_named("Foo"), void()]);
    check("void | resource", vec![void(), t_resource()], &[t_resource(), void()]);
    check("void | array{}", vec![void(), t_empty_array()], &[t_empty_array(), void()]);
    check("void | never", vec![void(), never()], &[void()]);
    check("void | mixed", vec![void(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_never() {
    check("never | never", vec![never(), never()], &[never()]);
    check("never | int", vec![never(), t_int()], &[t_int()]);
    check("never | string", vec![never(), t_string()], &[t_string()]);
    check("never | float", vec![never(), t_float()], &[t_float()]);
    check("never | bool", vec![never(), t_bool()], &[t_bool()]);
    check("never | null", vec![never(), null()], &[null()]);
    check("never | void", vec![never(), void()], &[void()]);
    check("never | object", vec![never(), t_object_any()], &[t_object_any()]);
    check("never | Foo", vec![never(), t_named("Foo")], &[t_named("Foo")]);
    check("never | resource", vec![never(), t_resource()], &[t_resource()]);
    check("never | array{}", vec![never(), t_empty_array()], &[t_empty_array()]);
    check("never | mixed", vec![never(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_mixed() {
    check("mixed | mixed", vec![mixed(), mixed()], &[mixed()]);
    check("mixed | int", vec![mixed(), t_int()], &[mixed()]);
    check("mixed | string", vec![mixed(), t_string()], &[mixed()]);
    check("mixed | float", vec![mixed(), t_float()], &[mixed()]);
    check("mixed | bool", vec![mixed(), t_bool()], &[mixed()]);
    check("mixed | null", vec![mixed(), null()], &[mixed()]);
    check("mixed | void", vec![mixed(), void()], &[mixed()]);
    check("mixed | object", vec![mixed(), t_object_any()], &[mixed()]);
    check("mixed | Foo", vec![mixed(), t_named("Foo")], &[mixed()]);
    check("mixed | resource", vec![mixed(), t_resource()], &[mixed()]);
    check("mixed | array{}", vec![mixed(), t_empty_array()], &[mixed()]);
    check("mixed | never", vec![mixed(), never()], &[mixed()]);
}

#[test]
fn primitive_pairs_array_key() {}

#[test]
fn primitive_pairs_scalar() {}

#[test]
fn primitive_pairs_object() {
    check("object | object", vec![t_object_any(), t_object_any()], &[t_object_any()]);
    check("object | Foo", vec![t_object_any(), t_named("Foo")], &[t_object_any()]);
    check("Foo | object", vec![t_named("Foo"), t_object_any()], &[t_object_any()]);
    check("object | Bar", vec![t_object_any(), t_named("Bar")], &[t_object_any()]);
    // `object` absorbs the entire object family including enums.
    check("object | enum (suffete absorbs)", vec![t_object_any(), t_enum("E")], &[t_object_any()]);
    check("object | int", vec![t_object_any(), t_int()], &[t_int(), t_object_any()]);
    check("object | resource", vec![t_object_any(), t_resource()], &[t_object_any(), t_resource()]);
    check("object | array{}", vec![t_object_any(), t_empty_array()], &[t_empty_array(), t_object_any()]);
    check("object | never", vec![t_object_any(), never()], &[t_object_any()]);
    check("object | mixed", vec![t_object_any(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_resource() {
    check("resource | resource", vec![t_resource(), t_resource()], &[t_resource()]);
    check("resource | open", vec![t_resource(), t_open_resource()], &[t_resource()]);
    check("resource | closed", vec![t_resource(), t_closed_resource()], &[t_resource()]);
    check("open | closed", vec![t_open_resource(), t_closed_resource()], &[t_resource()]);
    check("resource | int", vec![t_resource(), t_int()], &[t_int(), t_resource()]);
    check("open | int", vec![t_open_resource(), t_int()], &[t_int(), t_open_resource()]);
    check("closed | string", vec![t_closed_resource(), t_string()], &[t_closed_resource(), t_string()]);
    check("resource | never", vec![t_resource(), never()], &[t_resource()]);
    check("resource | mixed", vec![t_resource(), mixed()], &[mixed()]);
}

#[test]
fn primitive_pairs_array() {}

#[test]
fn primitive_pairs_class_like() {
    check("class-string | class-string", vec![t_class_string(), t_class_string()], &[t_class_string()]);
    check(
        "interface-string | interface-string",
        vec![t_interface_string(), t_interface_string()],
        &[t_interface_string()],
    );
    check(
        "class-string | interface-string",
        vec![t_class_string(), t_interface_string()],
        &[t_class_string(), t_interface_string()],
    );
    check("class-string | enum-string", vec![t_class_string(), t_enum_string()], &[t_class_string(), t_enum_string()]);
    check(
        "class-string | trait-string",
        vec![t_class_string(), t_trait_string()],
        &[t_class_string(), t_trait_string()],
    );
    check("class-string | string", vec![t_class_string(), t_string()], &[t_string()]);
    check("class-string | int", vec![t_class_string(), t_int()], &[t_class_string(), t_int()]);
    check("class-string | null", vec![t_class_string(), null()], &[t_class_string(), null()]);
    check("class-string | never", vec![t_class_string(), never()], &[t_class_string()]);
    check("class-string | mixed", vec![t_class_string(), mixed()], &[mixed()]);
}

#[test]
fn numeric_pairs() {}

#[test]
fn string_refinement_pairs() {}
