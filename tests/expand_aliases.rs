//! Stage 1 expander tests: alias resolution and structural descent.
//! Aliases declared on a class via `MockWorld::with_alias` are looked
//! up and replaced by their recorded body, recursively expanded.

mod comparator_common;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::expand;
use suffete::element::payload::AliasInfo;
use suffete::interner::interner;
use suffete::prelude;

fn t_alias(class: &str, alias: &str) -> ElementId {
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}

#[test]
fn alias_to_int_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let ty = u(t_alias("Foo", "Id"));
    let result = expand::expand(ty, &w);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn unknown_alias_passes_through_unchanged() {
    let cb = empty_world();
    let ty = u(t_alias("Foo", "Id"));
    assert_eq!(expand::expand(ty, &cb), ty);
}

#[test]
fn alias_to_union_flat_merges() {
    let mut w = MockWorld::new();
    let int_or_string = u_many(vec![t_int(), t_string()]);
    w.with_alias("Foo", "Key", int_or_string);
    let ty = u(t_alias("Foo", "Key"));
    let result = expand::expand(ty, &w);
    assert_eq!(result, prelude::TYPE_INT_OR_STRING);
}

#[test]
fn nested_alias_expands_recursively() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "A", prelude::TYPE_INT);
    w.with_alias("Foo", "B", u(t_alias("Foo", "A")));
    let ty = u(t_alias("Foo", "B"));
    let result = expand::expand(ty, &w);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn alias_inside_list_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_t = u(t_alias("Foo", "Id"));
    let list = u(t_list(alias_t, false));
    let result = expand::expand(list, &w);
    let expected = u(t_list(prelude::TYPE_INT, false));
    assert_eq!(result, expected);
}

#[test]
fn alias_inside_object_type_args_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_t = u(t_alias("Foo", "Id"));
    let generic = u(t_generic_named("Box", vec![alias_t]));
    let result = expand::expand(generic, &w);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn alias_inside_iterable_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "K", prelude::TYPE_STRING);
    w.with_alias("Foo", "V", prelude::TYPE_INT);
    let key = u(t_alias("Foo", "K"));
    let value = u(t_alias("Foo", "V"));
    let iter = u(t_iterable(key, value));
    let result = expand::expand(iter, &w);
    let expected = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    assert_eq!(result, expected);
}

#[test]
fn alias_inside_keyed_array_value_expands() {
    use std::collections::BTreeMap;
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_t = u(t_alias("Foo", "Id"));
    let shape = u(t_keyed_sealed(BTreeMap::from([(ak_str("id"), (false, alias_t))]), false));
    let result = expand::expand(shape, &w);
    let expected = u(t_keyed_sealed(BTreeMap::from([(ak_str("id"), (false, prelude::TYPE_INT))]), false));
    assert_eq!(result, expected);
}

#[test]
fn alias_to_alias_chain_expands_to_terminal() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "A", prelude::TYPE_INT);
    w.with_alias("Foo", "B", u(t_alias("Foo", "A")));
    w.with_alias("Foo", "C", u(t_alias("Foo", "B")));
    let ty = u(t_alias("Foo", "C"));
    let result = expand::expand(ty, &w);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn distinct_aliases_in_union_each_expand() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "I", prelude::TYPE_INT);
    w.with_alias("Foo", "S", prelude::TYPE_STRING);
    let ty = u_many(vec![t_alias("Foo", "I"), t_alias("Foo", "S")]);
    let result = expand::expand(ty, &w);
    assert_eq!(result, prelude::TYPE_INT_OR_STRING);
}

#[test]
fn no_alias_no_change() {
    let cb = empty_world();
    assert_eq!(expand::expand(prelude::TYPE_INT, &cb), prelude::TYPE_INT);
    assert_eq!(expand::expand(prelude::TYPE_INT_OR_STRING, &cb), prelude::TYPE_INT_OR_STRING);
}

#[test]
fn expanded_handle_is_stable() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let ty = u(t_alias("Foo", "Id"));
    let r1 = expand::expand(ty, &w);
    let r2 = expand::expand(ty, &w);
    assert_eq!(r1, r2);
}

#[test]
fn fully_structural_input_returns_same_handle() {
    let cb = empty_world();
    let ty = u(t_list(prelude::TYPE_INT, false));
    let result = expand::expand(ty, &cb);
    assert_eq!(result, ty);
}

#[test]
fn alias_preserves_flow_flags() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_elem = t_alias("Foo", "Id");
    let ty = interner().intern_type(&[alias_elem], FlowFlags::EMPTY);
    let result = expand::expand(ty, &w);
    assert_eq!(result.as_ref().flags, FlowFlags::EMPTY);
}

#[test]
fn deeply_nested_alias_in_box_of_list_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_t = u(t_alias("Foo", "Id"));
    let list_of_alias = u(t_list(alias_t, false));
    let box_of_list = u(t_generic_named("Box", vec![list_of_alias]));
    let result = expand::expand(box_of_list, &w);
    let expected_inner = u(t_list(prelude::TYPE_INT, false));
    let expected = u(t_generic_named("Box", vec![expected_inner]));
    assert_eq!(result, expected);
}
