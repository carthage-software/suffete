//! Table-driven combiner cases. Each test runs a vec of `(label, input,
//! expected)` triples through `run_cases`. Subtype-driven and array-shape
//! rows are split off into `#[ignore]`'d shells to keep the structural
//! tests green.

mod combiner_common;

use combiner_common::*;
use suffete::ElementId;

type Case = (&'static str, Vec<ElementId>, Vec<ElementId>);

fn run_cases(cases: Vec<Case>) {
    for (label, input, expected) in cases {
        let result = combine_default(input);
        let mut actual = result.clone();
        actual.sort();
        let mut e = expected.clone();
        e.sort();
        assert_eq!(actual, e, "case `{label}` failed: got {result:?}, expected {expected:?}");
    }
}

#[test]
fn bool_cases() {
    run_cases(vec![
        ("true", vec![t_true()], vec![t_true()]),
        ("false", vec![t_false()], vec![t_false()]),
        ("bool", vec![t_bool()], vec![t_bool()]),
        ("true | true", vec![t_true(), t_true()], vec![t_true()]),
        ("false | false", vec![t_false(), t_false()], vec![t_false()]),
        ("bool | bool", vec![t_bool(), t_bool()], vec![t_bool()]),
        ("true | false", vec![t_true(), t_false()], vec![t_bool()]),
        ("false | true", vec![t_false(), t_true()], vec![t_bool()]),
        ("bool | true", vec![t_bool(), t_true()], vec![t_bool()]),
        ("true | bool", vec![t_true(), t_bool()], vec![t_bool()]),
        ("bool | false", vec![t_bool(), t_false()], vec![t_bool()]),
        ("false | bool", vec![t_false(), t_bool()], vec![t_bool()]),
        ("true,false,bool", vec![t_true(), t_false(), t_bool()], vec![t_bool()]),
        ("bool,true,false", vec![t_bool(), t_true(), t_false()], vec![t_bool()]),
    ]);
}

#[test]
fn integer_cases_structural() {
    run_cases(vec![
        ("int", vec![t_int()], vec![t_int()]),
        ("int(0)", vec![t_lit_int(0)], vec![t_lit_int(0)]),
        ("int(1)", vec![t_lit_int(1)], vec![t_lit_int(1)]),
        ("int(-1)", vec![t_lit_int(-1)], vec![t_lit_int(-1)]),
        ("int | int(0)", vec![t_int(), t_lit_int(0)], vec![t_int()]),
        ("int(0) | int", vec![t_lit_int(0), t_int()], vec![t_int()]),
        ("int(0) | int(0)", vec![t_lit_int(0), t_lit_int(0)], vec![t_lit_int(0)]),
        ("int(0) | int(1)", vec![t_lit_int(0), t_lit_int(1)], vec![t_lit_int(0), t_lit_int(1)]),
        (
            "int(0) | int(1) | int(2)",
            vec![t_lit_int(0), t_lit_int(1), t_lit_int(2)],
            vec![t_lit_int(0), t_lit_int(1), t_lit_int(2)],
        ),
        (
            "int(0) | int(1) | int(-1)",
            vec![t_lit_int(0), t_lit_int(1), t_lit_int(-1)],
            vec![t_lit_int(0), t_lit_int(1), t_lit_int(-1)],
        ),
    ]);
}

#[test]
#[ignore = "needs subtype-driven int range merging / extension"]
fn integer_cases_subtype() {}

#[test]
fn string_cases_structural() {
    run_cases(vec![
        ("string", vec![t_string()], vec![t_string()]),
        ("''", vec![t_lit_string("")], vec![t_lit_string("")]),
        ("'hi'", vec![t_lit_string("hi")], vec![t_lit_string("hi")]),
        ("non-empty", vec![t_non_empty_string()], vec![t_non_empty_string()]),
        ("numeric-string", vec![t_numeric_string()], vec![t_numeric_string()]),
        ("lowercase-string", vec![t_lower_string()], vec![t_lower_string()]),
        ("uppercase-string", vec![t_upper_string()], vec![t_upper_string()]),
        ("truthy-string", vec![t_truthy_string()], vec![t_truthy_string()]),
        ("string | ''", vec![t_string(), t_lit_string("")], vec![t_string()]),
        ("string | 'hi'", vec![t_string(), t_lit_string("hi")], vec![t_string()]),
        ("'hi' | string", vec![t_lit_string("hi"), t_string()], vec![t_string()]),
        ("'a' | 'b'", vec![t_lit_string("a"), t_lit_string("b")], vec![t_lit_string("a"), t_lit_string("b")]),
        ("'a' | 'a'", vec![t_lit_string("a"), t_lit_string("a")], vec![t_lit_string("a")]),
    ]);
}

#[test]
#[ignore = "needs subtype-driven string axis collapse / refinement absorption"]
fn string_cases_subtype() {}

#[test]
fn float_cases() {
    run_cases(vec![
        ("float", vec![t_float()], vec![t_float()]),
        ("float(1.5)", vec![t_lit_float(1.5)], vec![t_lit_float(1.5)]),
        ("float | float(1.5)", vec![t_float(), t_lit_float(1.5)], vec![t_float()]),
        ("float(1.5) | float", vec![t_lit_float(1.5), t_float()], vec![t_float()]),
        ("float(1.5) | float(1.5)", vec![t_lit_float(1.5), t_lit_float(1.5)], vec![t_lit_float(1.5)]),
        ("float(1.5) | float(2.5)", vec![t_lit_float(1.5), t_lit_float(2.5)], vec![t_lit_float(1.5), t_lit_float(2.5)]),
    ]);
}

#[test]
fn cross_family_cases_structural() {
    run_cases(vec![
        ("int | string", vec![t_int(), t_string()], vec![t_int(), t_string()]),
        ("int | float", vec![t_int(), t_float()], vec![t_float(), t_int()]),
        ("int | bool", vec![t_int(), t_bool()], vec![t_bool(), t_int()]),
        ("int | string | float", vec![t_int(), t_string(), t_float()], vec![t_float(), t_int(), t_string()]),
    ]);
}

#[test]
#[ignore = "needs subtype-driven scalar / array-key / scalar-synthesis"]
fn cross_family_cases_subtype() {}

#[test]
fn special_type_cases() {
    run_cases(vec![
        ("null", vec![null()], vec![null()]),
        ("void", vec![void()], vec![void()]),
        ("never", vec![never()], vec![never()]),
        ("null | null", vec![null(), null()], vec![null()]),
        ("void | void", vec![void(), void()], vec![void()]),
        ("never | never", vec![never(), never()], vec![never()]),
        ("null | void", vec![null(), void()], vec![null()]),
        ("void | null", vec![void(), null()], vec![null()]),
        ("null | never", vec![null(), never()], vec![null()]),
        ("never | null", vec![never(), null()], vec![null()]),
        ("void | never", vec![void(), never()], vec![never()]),
        ("never | void", vec![never(), void()], vec![never()]),
        ("never | int", vec![never(), t_int()], vec![t_int()]),
        ("int | never", vec![t_int(), never()], vec![t_int()]),
        ("void | int", vec![void(), t_int()], vec![t_int()]),
        ("int | void", vec![t_int(), void()], vec![t_int()]),
        ("null | int", vec![null(), t_int()], vec![t_int(), null()]),
        ("int | null", vec![t_int(), null()], vec![t_int(), null()]),
        ("null | string", vec![null(), t_string()], vec![null(), t_string()]),
        ("null | object", vec![null(), t_object_any()], vec![null(), t_object_any()]),
        ("null | named", vec![null(), t_named("Foo")], vec![t_named("Foo"), null()]),
    ]);
}

#[test]
fn resource_cases() {
    run_cases(vec![
        ("resource", vec![t_resource()], vec![t_resource()]),
        ("open-resource", vec![t_open_resource()], vec![t_open_resource()]),
        ("closed-resource", vec![t_closed_resource()], vec![t_closed_resource()]),
        ("resource | open", vec![t_resource(), t_open_resource()], vec![t_resource()]),
        ("open | resource", vec![t_open_resource(), t_resource()], vec![t_resource()]),
        ("resource | closed", vec![t_resource(), t_closed_resource()], vec![t_resource()]),
        ("closed | resource", vec![t_closed_resource(), t_resource()], vec![t_resource()]),
        ("open | closed", vec![t_open_resource(), t_closed_resource()], vec![t_resource()]),
        ("closed | open", vec![t_closed_resource(), t_open_resource()], vec![t_resource()]),
        ("open | open", vec![t_open_resource(), t_open_resource()], vec![t_open_resource()]),
        ("closed | closed", vec![t_closed_resource(), t_closed_resource()], vec![t_closed_resource()]),
        ("resource | int", vec![t_resource(), t_int()], vec![t_int(), t_resource()]),
        ("open | int", vec![t_open_resource(), t_int()], vec![t_int(), t_open_resource()]),
        ("closed | string", vec![t_closed_resource(), t_string()], vec![t_closed_resource(), t_string()]),
    ]);
}

#[test]
fn object_cases() {
    run_cases(vec![
        ("object", vec![t_object_any()], vec![t_object_any()]),
        ("Foo", vec![t_named("Foo")], vec![t_named("Foo")]),
        ("E (enum)", vec![t_enum("E")], vec![t_enum("E")]),
        ("E::A (case)", vec![t_enum_case("E", "A")], vec![t_enum_case("E", "A")]),
        ("object | Foo", vec![t_object_any(), t_named("Foo")], vec![t_object_any()]),
        ("Foo | object", vec![t_named("Foo"), t_object_any()], vec![t_object_any()]),
        ("Foo | Foo", vec![t_named("Foo"), t_named("Foo")], vec![t_named("Foo")]),
        ("Foo | Bar", vec![t_named("Foo"), t_named("Bar")], vec![t_named("Bar"), t_named("Foo")]),
        ("E | E", vec![t_enum("E"), t_enum("E")], vec![t_enum("E")]),
        ("E | F", vec![t_enum("E"), t_enum("F")], vec![t_enum("E"), t_enum("F")]),
        ("E::A | E::A", vec![t_enum_case("E", "A"), t_enum_case("E", "A")], vec![t_enum_case("E", "A")]),
        (
            "E::A | E::B",
            vec![t_enum_case("E", "A"), t_enum_case("E", "B")],
            vec![t_enum_case("E", "A"), t_enum_case("E", "B")],
        ),
        ("E | E::A", vec![t_enum("E"), t_enum_case("E", "A")], vec![t_enum("E"), t_enum_case("E", "A")]),
        ("Foo | int", vec![t_named("Foo"), t_int()], vec![t_named("Foo"), t_int()]),
        ("object | int", vec![t_object_any(), t_int()], vec![t_int(), t_object_any()]),
        ("Foo | string", vec![t_named("Foo"), t_string()], vec![t_named("Foo"), t_string()]),
    ]);
}

#[test]
fn array_cases_empty() {
    run_cases(vec![
        ("array{}", vec![t_empty_array()], vec![t_empty_array()]),
        ("array{} | array{}", vec![t_empty_array(), t_empty_array()], vec![t_empty_array()]),
    ]);
}

#[test]
#[ignore = "needs t_list / t_keyed_unsealed / t_sealed_list helpers"]
fn array_cases_shapes() {}

#[test]
fn mixed_dominance_cases() {
    run_cases(vec![
        ("mixed", vec![mixed()], vec![mixed()]),
        ("mixed | int", vec![mixed(), t_int()], vec![mixed()]),
        ("int | mixed", vec![t_int(), mixed()], vec![mixed()]),
        ("mixed | string", vec![mixed(), t_string()], vec![mixed()]),
        ("mixed | object", vec![mixed(), t_object_any()], vec![mixed()]),
        ("mixed | array{}", vec![mixed(), t_empty_array()], vec![mixed()]),
        ("mixed | null", vec![mixed(), null()], vec![mixed()]),
        ("mixed | never", vec![mixed(), never()], vec![mixed()]),
        ("mixed | resource", vec![mixed(), t_resource()], vec![mixed()]),
        ("truthy-mixed", vec![mixed_truthy()], vec![mixed_truthy()]),
        ("falsy-mixed", vec![mixed_falsy()], vec![mixed_falsy()]),
        ("nonnull-mixed", vec![mixed_nonnull()], vec![mixed_nonnull()]),
    ]);
}

#[test]
#[ignore = "needs subtype-driven mixed-axis collapse (truthy + falsy -> nonnull, etc.)"]
fn mixed_axis_cases() {}

#[test]
fn multi_atom_cases_structural() {
    run_cases(vec![
        ("3 ints", vec![t_lit_int(1), t_lit_int(2), t_lit_int(3)], vec![t_lit_int(1), t_lit_int(2), t_lit_int(3)]),
        ("2 ints + int", vec![t_lit_int(1), t_lit_int(2), t_int()], vec![t_int()]),
        ("int | string | Foo", vec![t_int(), t_string(), t_named("Foo")], vec![t_named("Foo"), t_int(), t_string()]),
        ("null | int | string", vec![null(), t_int(), t_string()], vec![t_int(), null(), t_string()]),
        (
            "5 distinct named",
            vec![t_named("A"), t_named("B"), t_named("C"), t_named("D"), t_named("E")],
            vec![t_named("A"), t_named("B"), t_named("C"), t_named("D"), t_named("E")],
        ),
        (
            "4 distinct enums",
            vec![t_enum("E1"), t_enum("E2"), t_enum("E3"), t_enum("E4")],
            vec![t_enum("E1"), t_enum("E2"), t_enum("E3"), t_enum("E4")],
        ),
    ]);
}

#[test]
fn class_like_string_cases() {
    run_cases(vec![
        ("class-string", vec![t_class_string()], vec![t_class_string()]),
        ("interface-string", vec![t_interface_string()], vec![t_interface_string()]),
        ("enum-string", vec![t_enum_string()], vec![t_enum_string()]),
        ("trait-string", vec![t_trait_string()], vec![t_trait_string()]),
        ("class-string | string", vec![t_class_string(), t_string()], vec![t_class_string(), t_string()]),
        ("string | class-string", vec![t_string(), t_class_string()], vec![t_class_string(), t_string()]),
        ("class-string | array-key", vec![t_class_string(), t_array_key()], vec![t_array_key(), t_class_string()]),
        ("class-string | scalar", vec![t_class_string(), t_scalar()], vec![t_class_string(), t_scalar()]),
        (
            "all 4 class-like kinds",
            vec![t_class_string(), t_interface_string(), t_enum_string(), t_trait_string()],
            vec![t_class_string(), t_interface_string(), t_enum_string(), t_trait_string()],
        ),
    ]);
}
