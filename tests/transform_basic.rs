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

use suffete::ElementId;
use suffete::prelude;
use suffete::transform;

#[test]
fn map_no_op_returns_same_handle() {
    let ty = prelude::TYPE_INT_OR_STRING;
    let result = transform::map(ty, |e| e);
    assert_eq!(result, ty);
}

#[test]
fn map_replaces_each_top_level_element() {
    let ty = u_many(vec![t_int(), t_string()]);
    let result = transform::map(ty, |e| if e == t_int() { t_float() } else { e });
    let expected = u_many(vec![t_float(), t_string()]);
    assert_eq!(result, expected);
}

#[test]
fn map_descends_into_list_element_type() {
    let inner = u(t_int());
    let list = u(t_list(inner, false));
    let result = transform::map(list, |e| if e == t_int() { t_string() } else { e });
    let expected = u(t_list(u(t_string()), false));
    assert_eq!(result, expected);
}

#[test]
fn map_descends_into_object_type_args() {
    let inner = prelude::TYPE_INT;
    let generic = u(t_generic_named("Box", vec![inner]));
    let result = transform::map(generic, |e| if e == t_int() { t_string() } else { e });
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_STRING]));
    assert_eq!(result, expected);
}

#[test]
fn map_descends_into_iterable_key_and_value() {
    let iter = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    let result = transform::map(iter, |e| if e == t_int() { t_float() } else { e });
    let expected = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_FLOAT));
    assert_eq!(result, expected);
}

#[test]
fn map_descends_into_keyed_array_value_param() {
    let arr = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let result = transform::map(arr, |e| if e == t_int() { t_float() } else { e });
    let expected = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_FLOAT, false));
    assert_eq!(result, expected);
}

#[test]
fn map_post_order_sees_rebuilt_children() {
    let inner = u(t_int());
    let list = u(t_list(inner, false));
    let mut seen: Vec<ElementId> = Vec::new();
    transform::map(list, |e| {
        seen.push(e);
        e
    });
    let Some(int_idx) = seen.iter().position(|e| *e == t_int()) else { panic!("int leaf seen") };
    let Some(list_idx) = seen.iter().position(|e| e.kind() == suffete::ElementKind::List) else { panic!("list seen") };
    assert!(int_idx < list_idx, "post-order: leaf must be visited before its container");
}

#[test]
fn flat_map_one_to_many_explodes_top_level() {
    let ty = u(t_lit_int(5));
    let result =
        transform::flat_map(
            ty,
            |e| {
                if e == t_lit_int(5) { vec![t_int_range(0, 4), t_int_range(6, 10)] } else { vec![e] }
            },
        );
    let expected = u_many(vec![t_int_range(0, 4), t_int_range(6, 10)]);
    assert_eq!(result, expected);
}

#[test]
fn flat_map_one_to_zero_drops_element() {
    let ty = u_many(vec![t_int(), t_string()]);
    let result = transform::flat_map(ty, |e| if e == t_string() { Vec::new() } else { vec![e] });
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn flat_map_inside_list_explodes_nested_element_type() {
    let list = u(t_list(u(t_lit_int(5)), false));
    let result = transform::flat_map(list, |e| {
        if e == t_lit_int(5) { vec![t_int_range(0, 4), t_int_range(6, 10)] } else { vec![e] }
    });
    let inner = u_many(vec![t_int_range(0, 4), t_int_range(6, 10)]);
    let expected = u(t_list(inner, false));
    assert_eq!(result, expected);
}

#[test]
fn filter_map_drops_when_returning_none() {
    let ty = u_many(vec![t_int(), t_string(), null()]);
    let result = transform::filter_map(ty, |e| if e == null() { None } else { Some(e) });
    let expected = u_many(vec![t_int(), t_string()]);
    assert_eq!(result, expected);
}

#[test]
fn filter_drops_predicate_false() {
    let ty = u_many(vec![t_int(), t_string(), null()]);
    let result = transform::filter(ty, |e| *e != null());
    let expected = u_many(vec![t_int(), t_string()]);
    assert_eq!(result, expected);
}

#[test]
fn filter_emptying_a_level_collapses_to_never() {
    let ty = u_many(vec![t_int(), t_string()]);
    let result = transform::filter(ty, |_| false);
    assert_eq!(result, prelude::TYPE_NEVER);
}

#[test]
fn filter_emptying_a_nested_level_yields_never_at_that_level() {
    let list = u(t_list(prelude::TYPE_INT, false));
    let result = transform::filter(list, |e| *e != t_int());
    let expected = u(t_list(prelude::TYPE_NEVER, false));
    assert_eq!(result, expected);
}

#[test]
fn map_descends_into_class_like_string_constraint() {
    use suffete::element::payload::ClassLikeKind;
    use suffete::element::payload::ClassLikeStringInfo;
    use suffete::element::payload::ClassLikeStringSpecifier;
    use suffete::interner::interner;
    let constrained = interner().intern_class_like_string(ClassLikeStringInfo {
        kind: ClassLikeKind::Class,
        specifier: ClassLikeStringSpecifier::OfType { constraint: u(t_named("Foo")) },
    });
    let ty = u(constrained);
    let result = transform::map(ty, |e| if e == t_named("Foo") { t_named("Bar") } else { e });
    let expected_inner = interner().intern_class_like_string(ClassLikeStringInfo {
        kind: ClassLikeKind::Class,
        specifier: ClassLikeStringSpecifier::OfType { constraint: u(t_named("Bar")) },
    });
    assert_eq!(result, u(expected_inner));
}

#[test]
fn map_descends_into_generic_parameter_constraint() {
    use mago_atom::atom;
    use suffete::element::payload::DefiningEntity;
    let param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), prelude::TYPE_INT);
    let ty = u(param);
    let result = transform::map(ty, |e| if e == t_int() { t_string() } else { e });
    let expected_param =
        ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), prelude::TYPE_STRING);
    let expected = u(expected_param);
    assert_eq!(result, expected);
}
