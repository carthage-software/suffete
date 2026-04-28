//! Mega-table combiner cases. Each `expect(label, input, expected)` is one
//! row. Subtype-driven, range-merging, and array-shape rows are skipped
//! (covered by the dedicated `#[ignore]`'d shells in other files).

mod combiner_common;

use combiner_common::*;
use suffete::ElementId;

fn expect(label: &str, input: Vec<ElementId>, expected: &[ElementId]) {
    let result = combine_default(input);
    let mut actual = result.clone();
    actual.sort();
    let mut e = expected.to_vec();
    e.sort();
    assert_eq!(actual, e, "{label}: got {result:?}, expected {expected:?}");
}

#[test]
fn boolean_megatable() {
    expect("true", vec![t_true()], &[t_true()]);
    expect("false", vec![t_false()], &[t_false()]);
    expect("bool", vec![t_bool()], &[t_bool()]);
    expect("true|true", vec![t_true(), t_true()], &[t_true()]);
    expect("true|false", vec![t_true(), t_false()], &[t_bool()]);
    expect("true|bool", vec![t_true(), t_bool()], &[t_bool()]);
    expect("false|true", vec![t_false(), t_true()], &[t_bool()]);
    expect("false|false", vec![t_false(), t_false()], &[t_false()]);
    expect("false|bool", vec![t_false(), t_bool()], &[t_bool()]);
    expect("bool|true", vec![t_bool(), t_true()], &[t_bool()]);
    expect("bool|false", vec![t_bool(), t_false()], &[t_bool()]);
    expect("bool|bool", vec![t_bool(), t_bool()], &[t_bool()]);
    expect("3true", vec![t_true(); 3], &[t_true()]);
    expect("3false", vec![t_false(); 3], &[t_false()]);
    expect("3bool", vec![t_bool(); 3], &[t_bool()]);
    expect("true,true,false", vec![t_true(), t_true(), t_false()], &[t_bool()]);
    expect("false,false,true", vec![t_false(), t_false(), t_true()], &[t_bool()]);
    expect("true,bool,false", vec![t_true(), t_bool(), t_false()], &[t_bool()]);
    expect("bool,bool,true", vec![t_bool(), t_bool(), t_true()], &[t_bool()]);
    expect("bool,bool,false", vec![t_bool(), t_bool(), t_false()], &[t_bool()]);
    expect("4true", vec![t_true(); 4], &[t_true()]);
    expect("4false", vec![t_false(); 4], &[t_false()]);
    expect("bool*4", vec![t_bool(); 4], &[t_bool()]);
    expect("alt true,false", vec![t_true(), t_false(), t_true(), t_false()], &[t_bool()]);
}

#[test]
fn integer_megatable_singletons() {
    expect("int", vec![t_int()], &[t_int()]);
    expect("int(0)", vec![t_lit_int(0)], &[t_lit_int(0)]);
    expect("int(-100)", vec![t_lit_int(-100)], &[t_lit_int(-100)]);
    expect("int(1000)", vec![t_lit_int(1000)], &[t_lit_int(1000)]);
    expect("positive-int", vec![t_positive_int()], &[t_positive_int()]);
    expect("non-negative-int", vec![t_non_negative_int()], &[t_non_negative_int()]);
    expect("negative-int", vec![t_negative_int()], &[t_negative_int()]);
    expect("non-positive-int", vec![t_non_positive_int()], &[t_non_positive_int()]);
    expect("Range(0,10)", vec![t_int_range(0, 10)], &[t_int_range(0, 10)]);
    expect("From(5)", vec![t_int_from(5)], &[t_int_from(5)]);
    expect("To(5)", vec![t_int_to(5)], &[t_int_to(5)]);
    expect("UnspecLit", vec![t_int_unspec_lit()], &[t_int_unspec_lit()]);
}

#[test]
fn integer_megatable_dominator() {
    expect("int|int(0)", vec![t_int(), t_lit_int(0)], &[t_int()]);
    expect("int|positive", vec![t_int(), t_positive_int()], &[t_int()]);
    expect("int|negative", vec![t_int(), t_negative_int()], &[t_int()]);
    expect("int|Range", vec![t_int(), t_int_range(0, 10)], &[t_int()]);
    expect("int|From", vec![t_int(), t_int_from(5)], &[t_int()]);
    expect("int|To", vec![t_int(), t_int_to(5)], &[t_int()]);
    expect("int|UnspecLit", vec![t_int(), t_int_unspec_lit()], &[t_int()]);
}

#[test]
fn integer_megatable_subtype() {}

#[test]
fn string_megatable_singletons() {
    expect("string", vec![t_string()], &[t_string()]);
    expect("non-empty-string", vec![t_non_empty_string()], &[t_non_empty_string()]);
    expect("numeric-string", vec![t_numeric_string()], &[t_numeric_string()]);
    expect("lowercase-string", vec![t_lower_string()], &[t_lower_string()]);
    expect("uppercase-string", vec![t_upper_string()], &[t_upper_string()]);
    expect("truthy-string", vec![t_truthy_string()], &[t_truthy_string()]);
    expect("''", vec![t_lit_string("")], &[t_lit_string("")]);
    expect("'hi'", vec![t_lit_string("hi")], &[t_lit_string("hi")]);
    expect("'0'", vec![t_lit_string("0")], &[t_lit_string("0")]);
    expect("'123'", vec![t_lit_string("123")], &[t_lit_string("123")]);
}

#[test]
fn string_megatable_dominator() {
    expect("string|''", vec![t_string(), t_lit_string("")], &[t_string()]);
    expect("string|'hi'", vec![t_string(), t_lit_string("hi")], &[t_string()]);
    expect("string|'0'", vec![t_string(), t_lit_string("0")], &[t_string()]);
    expect("'hi'|string", vec![t_lit_string("hi"), t_string()], &[t_string()]);
    expect("'a'|'b'", vec![t_lit_string("a"), t_lit_string("b")], &[t_lit_string("a"), t_lit_string("b")]);
    expect("'a'|'a'", vec![t_lit_string("a"), t_lit_string("a")], &[t_lit_string("a")]);
}

#[test]
fn string_megatable_subtype() {}

#[test]
fn float_megatable() {
    expect("float", vec![t_float()], &[t_float()]);
    expect("float(0)", vec![t_lit_float(0.0)], &[t_lit_float(0.0)]);
    expect("float(1.5)", vec![t_lit_float(1.5)], &[t_lit_float(1.5)]);
    expect("float(-1.0)", vec![t_lit_float(-1.0)], &[t_lit_float(-1.0)]);
    expect("float|float(1.5)", vec![t_float(), t_lit_float(1.5)], &[t_float()]);
    expect("float(1.5)|float", vec![t_lit_float(1.5), t_float()], &[t_float()]);
    expect("float(1.5)|float(2.5)", vec![t_lit_float(1.5), t_lit_float(2.5)], &[t_lit_float(1.5), t_lit_float(2.5)]);
    expect("float(1.5)|float(1.5)", vec![t_lit_float(1.5), t_lit_float(1.5)], &[t_lit_float(1.5)]);
}

#[test]
fn object_megatable() {
    expect("object", vec![t_object_any()], &[t_object_any()]);
    expect("Foo", vec![t_named("Foo")], &[t_named("Foo")]);
    expect("E", vec![t_enum("E")], &[t_enum("E")]);
    expect("E::A", vec![t_enum_case("E", "A")], &[t_enum_case("E", "A")]);
    expect("object|Foo", vec![t_object_any(), t_named("Foo")], &[t_object_any()]);
    expect("Foo|object", vec![t_named("Foo"), t_object_any()], &[t_object_any()]);
    expect("Foo|Foo", vec![t_named("Foo"), t_named("Foo")], &[t_named("Foo")]);
    expect("Foo|Bar", vec![t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo")]);
    expect(
        "Foo|Bar|Baz",
        vec![t_named("Foo"), t_named("Bar"), t_named("Baz")],
        &[t_named("Bar"), t_named("Baz"), t_named("Foo")],
    );
    expect("Foo|Bar|object", vec![t_named("Foo"), t_named("Bar"), t_object_any()], &[t_object_any()]);
    expect("E|E", vec![t_enum("E"), t_enum("E")], &[t_enum("E")]);
    expect("E|F", vec![t_enum("E"), t_enum("F")], &[t_enum("E"), t_enum("F")]);
    expect("E|E::A", vec![t_enum("E"), t_enum_case("E", "A")], &[t_enum("E")]);
    expect(
        "E::A|E::B",
        vec![t_enum_case("E", "A"), t_enum_case("E", "B")],
        &[t_enum_case("E", "A"), t_enum_case("E", "B")],
    );
}

#[test]
fn object_megatable_generic() {}

#[test]
fn array_megatable_empty() {
    expect("array{}", vec![t_empty_array()], &[t_empty_array()]);
    expect("array{}|array{}", vec![t_empty_array(); 2], &[t_empty_array()]);
    expect("3*array{}", vec![t_empty_array(); 3], &[t_empty_array()]);
}

#[test]
fn array_megatable_shapes() {}

#[test]
fn cross_family_megatable() {
    expect("int|object", vec![t_int(), t_object_any()], &[t_int(), t_object_any()]);
    expect("object|int", vec![t_object_any(), t_int()], &[t_int(), t_object_any()]);
    expect("int|resource", vec![t_int(), t_resource()], &[t_int(), t_resource()]);
    expect("int|open", vec![t_int(), t_open_resource()], &[t_int(), t_open_resource()]);
    expect("string|object", vec![t_string(), t_object_any()], &[t_object_any(), t_string()]);
    expect("string|resource", vec![t_string(), t_resource()], &[t_resource(), t_string()]);
    expect("string|array{}", vec![t_string(), t_empty_array()], &[t_empty_array(), t_string()]);
    expect("float|object", vec![t_float(), t_object_any()], &[t_float(), t_object_any()]);
    expect("bool|object", vec![t_bool(), t_object_any()], &[t_bool(), t_object_any()]);
    expect("null|object", vec![null(), t_object_any()], &[null(), t_object_any()]);
    expect("null|array{}", vec![null(), t_empty_array()], &[t_empty_array(), null()]);
    expect("null|resource", vec![null(), t_resource()], &[null(), t_resource()]);
    expect("Foo|resource", vec![t_named("Foo"), t_resource()], &[t_named("Foo"), t_resource()]);
    // mago expects `[Foo, enum(E)]` (no enum-vs-named collapse) — suffete's
    // structural object-family rule keeps both since OBJECT isn't present.
    expect("Foo|E", vec![t_named("Foo"), t_enum("E")], &[t_named("Foo"), t_enum("E")]);
    expect("E|resource", vec![t_enum("E"), t_resource()], &[t_enum("E"), t_resource()]);
    expect("array{}|object", vec![t_empty_array(), t_object_any()], &[t_empty_array(), t_object_any()]);
}

#[test]
fn scalar_synthesis_megatable() {}
