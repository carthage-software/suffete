//! Triple-wise combiner cases. Mirrors mago's combiner_triples. Cases that
//! depend on subtype-driven absorption / range merging / list shape
//! combine are stubbed `#[ignore]`.

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
fn bool_triples() {
    check("true,false,bool", vec![t_true(), t_false(), t_bool()], &[t_bool()]);
    check("bool,true,false", vec![t_bool(), t_true(), t_false()], &[t_bool()]);
    check("false,bool,true", vec![t_false(), t_bool(), t_true()], &[t_bool()]);
    check("true,true,false", vec![t_true(), t_true(), t_false()], &[t_bool()]);
    check("false,false,true", vec![t_false(), t_false(), t_true()], &[t_bool()]);
    check("bool,bool,bool", vec![t_bool(), t_bool(), t_bool()], &[t_bool()]);
    check("true,true,true", vec![t_true(), t_true(), t_true()], &[t_true()]);
    check("false,false,false", vec![t_false(), t_false(), t_false()], &[t_false()]);
}

#[test]
#[ignore = "needs subtype-driven int-range merging"]
fn int_lit_range_triples() {}

#[test]
#[ignore = "needs subtype-driven string-axis absorption"]
fn string_triples() {}

#[test]
fn null_triples() {
    check("null,int,string", vec![null(), t_int(), t_string()], &[t_int(), null(), t_string()]);
    check("null,never,int", vec![null(), never(), t_int()], &[t_int(), null()]);
    check("null,void,int", vec![null(), void(), t_int()], &[t_int(), null()]);
    check("null,null,int", vec![null(), null(), t_int()], &[t_int(), null()]);
    check("null,object,Foo", vec![null(), t_object_any(), t_named("Foo")], &[null(), t_object_any()]);
    check("null,Foo,Bar", vec![null(), t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo"), null()]);
}

#[test]
fn never_triples_absorbed() {
    check("never,int,string", vec![never(), t_int(), t_string()], &[t_int(), t_string()]);
    check("int,never,string", vec![t_int(), never(), t_string()], &[t_int(), t_string()]);
    check("int,string,never", vec![t_int(), t_string(), never()], &[t_int(), t_string()]);
    check("never,never,int", vec![never(), never(), t_int()], &[t_int()]);
    check("never,never,never", vec![never(), never(), never()], &[never()]);
    check("never,Foo,Bar", vec![never(), t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo")]);
    // mago expects `[enum(E), object]` (no enum-vs-object collapse) but
    // suffete's structural object-family rule absorbs enums under object.
    check("never,object,enum (suffete absorbs enum)", vec![never(), t_object_any(), t_enum("E")], &[t_object_any()]);
}

#[test]
fn void_triples() {
    check("void,int,string", vec![void(), t_int(), t_string()], &[t_int(), t_string(), void()]);
    check("void,null,int", vec![void(), null(), t_int()], &[t_int(), null()]);
    check("void,never,int", vec![void(), never(), t_int()], &[t_int(), void()]);
    check("void,never,never", vec![void(), never(), never()], &[void()]);
    check("void,void,int", vec![void(), void(), t_int()], &[t_int(), void()]);
    check("void,void,void", vec![void(), void(), void()], &[void()]);
    check("void,Foo,Bar", vec![void(), t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo"), void()]);
}

#[test]
fn mixed_triples_dominate() {
    check("mixed,int,string", vec![mixed(), t_int(), t_string()], &[mixed()]);
    check("int,mixed,string", vec![t_int(), mixed(), t_string()], &[mixed()]);
    check("int,string,mixed", vec![t_int(), t_string(), mixed()], &[mixed()]);
    check("mixed,Foo,Bar", vec![mixed(), t_named("Foo"), t_named("Bar")], &[mixed()]);
    check("mixed,never,int", vec![mixed(), never(), t_int()], &[mixed()]);
}

#[test]
#[ignore = "needs t_list helper + list shape combine"]
fn array_triples() {}

#[test]
fn object_triples() {
    check("object,Foo,Bar", vec![t_object_any(), t_named("Foo"), t_named("Bar")], &[t_object_any()]);
    check("Foo,Bar,object", vec![t_named("Foo"), t_named("Bar"), t_object_any()], &[t_object_any()]);
    check(
        "Foo,Bar,Baz",
        vec![t_named("Foo"), t_named("Bar"), t_named("Baz")],
        &[t_named("Bar"), t_named("Baz"), t_named("Foo")],
    );
    check("Foo,Foo,Bar", vec![t_named("Foo"), t_named("Foo"), t_named("Bar")], &[t_named("Bar"), t_named("Foo")]);
    check(
        "E,E::A,E::B",
        vec![t_enum("E"), t_enum_case("E", "A"), t_enum_case("E", "B")],
        &[t_enum("E"), t_enum_case("E", "A"), t_enum_case("E", "B")],
    );
}

#[test]
#[ignore = "needs subtype-driven scalar/array-key absorption"]
fn scalar_subtype_triples() {}

#[test]
fn resource_triples() {
    check("open,closed,resource", vec![t_open_resource(), t_closed_resource(), t_resource()], &[t_resource()]);
    check("open,open,closed", vec![t_open_resource(), t_open_resource(), t_closed_resource()], &[t_resource()]);
    check(
        "closed,closed,closed",
        vec![t_closed_resource(), t_closed_resource(), t_closed_resource()],
        &[t_closed_resource()],
    );
    check("open,open,open", vec![t_open_resource(), t_open_resource(), t_open_resource()], &[t_open_resource()]);
    check("open,int,closed", vec![t_open_resource(), t_int(), t_closed_resource()], &[t_int(), t_resource()]);
}

#[test]
fn four_atoms() {
    // mago expects `[scalar]` due to scalar synthesis from int+string+float+bool.
    // suffete doesn't synthesise scalar from primitive sets yet, so the four
    // atoms remain.
    let r = combine_default(vec![t_int(), t_string(), t_float(), t_bool()]);
    assert_eq!(r.len(), 4);

    check(
        "int,string,bool,null",
        vec![t_int(), t_string(), t_bool(), null()],
        &[t_bool(), t_int(), null(), t_string()],
    );
    check(
        "Foo,Bar,Baz,Qux",
        vec![t_named("Foo"), t_named("Bar"), t_named("Baz"), t_named("Qux")],
        &[t_named("Bar"), t_named("Baz"), t_named("Foo"), t_named("Qux")],
    );
    check(
        "object,Foo,Bar,Baz",
        vec![t_object_any(), t_named("Foo"), t_named("Bar"), t_named("Baz")],
        &[t_object_any()],
    );
    check("true,false,bool,bool", vec![t_true(), t_false(), t_bool(), t_bool()], &[t_bool()]);
}
