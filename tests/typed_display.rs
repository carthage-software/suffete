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

mod comparator_common;

use comparator_common::*;

use suffete::TypeId;
use suffete::prelude;
use suffete::typed::Handle;
use suffete::typed::Typed;
use suffete::typed::View;

fn render(ty: TypeId) -> String {
    format!("{ty}")
}

#[test]
fn primitives_render_as_keywords() {
    assert_eq!(render(prelude::TYPE_INT), "int");
    assert_eq!(render(prelude::TYPE_FLOAT), "float");
    assert_eq!(render(prelude::TYPE_STRING), "string");
    assert_eq!(render(prelude::TYPE_BOOL), "bool");
    assert_eq!(render(prelude::TYPE_NULL), "null");
    assert_eq!(render(prelude::TYPE_VOID), "void");
    assert_eq!(render(prelude::TYPE_NEVER), "never");
    assert_eq!(render(prelude::TYPE_MIXED), "mixed");
    assert_eq!(render(prelude::TYPE_OBJECT), "object");
    assert_eq!(render(prelude::TYPE_ARRAY_KEY), "array-key");
    assert_eq!(render(prelude::TYPE_NUMERIC), "numeric");
    assert_eq!(render(prelude::TYPE_SCALAR), "scalar");
}

#[test]
fn int_literal_renders_as_value() {
    assert_eq!(render(u(t_lit_int(42))), "int(42)");
    assert_eq!(render(u(t_lit_int(-1))), "int(-1)");
}

#[test]
fn int_range_named_forms() {
    assert_eq!(render(u(t_positive_int())), "positive-int");
    assert_eq!(render(u(t_negative_int())), "negative-int");
    assert_eq!(render(u(t_non_negative_int())), "non-negative-int");
    assert_eq!(render(u(t_non_positive_int())), "non-positive-int");
}

#[test]
fn int_range_with_bounds() {
    assert_eq!(render(u(t_int_range(0, 10))), "int<0, 10>");
    assert_eq!(render(u(t_int_from(5))), "int<5, max>");
    assert_eq!(render(u(t_int_to(100))), "int<min, 100>");
}

#[test]
fn string_literal_renders_quoted() {
    assert_eq!(render(u(t_lit_string("hello"))), "string('hello')");
}

#[test]
fn refined_strings_render() {
    assert_eq!(render(u(t_non_empty_string())), "non-empty-string");
    assert_eq!(render(u(t_numeric_string())), "numeric-string");
    assert_eq!(render(u(t_lower_string())), "lowercase-string");
    assert_eq!(render(u(t_upper_string())), "uppercase-string");
    assert_eq!(render(u(t_truthy_string())), "truthy-string");
}

#[test]
fn class_like_string_renders() {
    assert_eq!(render(u(t_class_string())), "class-string");
    assert_eq!(render(u(t_interface_string())), "interface-string");
    assert_eq!(render(u(t_enum_string())), "enum-string");
}

#[test]
fn named_object_renders() {
    assert_eq!(render(u(t_named("Foo"))), "Foo");
}

#[test]
fn generic_object_renders_with_args() {
    let box_int = t_generic_named("Box", vec![u(t_int())]);
    assert_eq!(render(u(box_int)), "Box<int>");
}

#[test]
fn intersected_object_renders_with_amp() {
    let foo_and_bar = t_named_intersected("Foo", &[t_named("Bar")]);
    assert_eq!(render(u(foo_and_bar)), "Foo&Bar");
}

#[test]
fn union_renders_with_pipe() {
    let int_or_string = u_many(vec![t_int(), t_string()]);
    assert_eq!(render(int_or_string), "int|string");
}

#[test]
fn nullable_renders() {
    let nullable_int = u_many(vec![null(), t_int()]);
    let s = render(nullable_int);
    assert!(s.contains("null") && s.contains("int"));
}

#[test]
fn list_renders() {
    let list_int = u(t_list(u(t_int()), false));
    assert_eq!(render(list_int), "list<int>");
}

#[test]
fn non_empty_list_renders() {
    let list_int = u(t_list(u(t_int()), true));
    assert_eq!(render(list_int), "non-empty-list<int>");
}

#[test]
fn keyed_array_unsealed_renders() {
    let arr = u(t_keyed_unsealed(u(t_string()), u(t_int()), false));
    assert_eq!(render(arr), "array<string, int>");
}

#[test]
fn iterable_renders() {
    let iter = u(t_iterable(u(t_string()), u(t_int())));
    assert_eq!(render(iter), "iterable<string, int>");
}

#[test]
fn callable_signature_renders() {
    let sig = u(t_callable(&[u(t_int())], u(t_string())));
    assert_eq!(render(sig), "(callable(int): string)");
}

#[test]
fn resource_variants_render() {
    assert_eq!(render(u(t_resource())), "resource");
    assert_eq!(render(u(t_open_resource())), "open-resource");
    assert_eq!(render(u(t_closed_resource())), "closed-resource");
}

#[test]
fn empty_array_renders() {
    assert_eq!(render(u(t_empty_array())), "array{}");
}

#[test]
fn typed_intersection_methods_on_typeid_return_trivial() {
    let ty = u(t_int());
    assert!(!Typed::can_be_intersected(&ty));
    assert!(!Typed::has_intersection_types(&ty));
    assert!(Typed::intersection_types(&ty).is_empty());
}

#[test]
fn typed_intersection_methods_on_element_id_dispatch() {
    let foo = t_named("Foo");
    assert!(Typed::can_be_intersected(&foo));
    assert!(!Typed::has_intersection_types(&foo));

    let intersected = t_named_intersected("Foo", &[t_named("Bar")]);
    assert!(Typed::has_intersection_types(&intersected));
    assert_eq!(Typed::intersection_types(&intersected), &[t_named("Bar")]);
}

#[test]
fn view_dispatches_to_inner() {
    let int_id = prelude::TYPE_INT;
    let int_view = View::Type(int_id.as_ref());
    assert_eq!(format!("{int_view}"), "int");
    assert!(!Typed::can_be_intersected(&int_view));
}

#[test]
fn handle_dispatches_to_inner() {
    let int_handle = Handle::Type(prelude::TYPE_INT);
    assert_eq!(format!("{int_handle}"), "int");
    assert!(!Typed::can_be_intersected(&int_handle));

    let foo = t_named("Foo");
    let foo_handle = Handle::Element(foo);
    assert_eq!(format!("{foo_handle}"), "Foo");
    assert!(Typed::can_be_intersected(&foo_handle));
}

#[test]
fn pretty_falls_back_to_display_for_singletons() {
    assert_eq!(Typed::pretty(&prelude::TYPE_INT), "int");
}

#[test]
fn pretty_breaks_unions_with_more_than_three_into_lines() {
    let u = u_many(vec![t_int(), t_string(), null(), t_float()]);
    let pretty = Typed::pretty(&u);
    assert!(pretty.contains('\n'), "pretty should be multi-line for >3 elements, got: {pretty}");
    assert!(pretty.contains("int"));
    assert!(pretty.contains("string"));
    assert!(pretty.contains("null"));
    assert!(pretty.contains("float"));
}

#[test]
fn pretty_three_element_union_stays_inline_with_spaces() {
    let u = u_many(vec![t_int(), t_string(), null()]);
    assert_eq!(Typed::pretty(&u), "int | null | string");
}

#[test]
fn pretty_object_shape_breaks_into_lines() {
    let shape = u(t_object_shape(&[("name", u(t_string()), false), ("age", u(t_int()), false)], true));
    let pretty = Typed::pretty(&shape);
    assert!(pretty.contains('\n'), "shape pretty should be multi-line, got: {pretty}");
    assert!(pretty.starts_with("object{\n"));
    assert!(pretty.ends_with('}'));
}

#[test]
fn pretty_unsealed_shape_includes_dots_line() {
    let shape = u(t_object_shape(&[("a", u(t_int()), false)], false));
    let pretty = Typed::pretty(&shape);
    assert!(pretty.contains("...,\n"), "unsealed shape should have ... line, got: {pretty}");
}

#[test]
fn pretty_callable_with_many_params_breaks_into_lines() {
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_STRING;
    let sig = u(t_callable(&[TYPE_INT, TYPE_STRING, TYPE_INT, TYPE_STRING], TYPE_INT));
    let pretty = Typed::pretty(&sig);
    assert!(pretty.contains('\n'), "callable with >2 params should be multi-line, got: {pretty}");
    assert!(pretty.contains("callable("));
    assert!(pretty.contains("): int"));
}

#[test]
fn pretty_callable_with_two_params_stays_inline() {
    use suffete::prelude::TYPE_INT;
    let sig = u(t_callable(&[TYPE_INT, TYPE_INT], TYPE_INT));
    assert_eq!(Typed::pretty(&sig), "(callable(int, int): int)");
}
