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

use suffete::ElementKind;
use suffete::inspect;
use suffete::prelude;

#[test]
fn any_top_level_match() {
    let ty = u_many(vec![t_int(), t_string()]);
    assert!(inspect::any(ty, |e| e.kind() == ElementKind::Int));
}

#[test]
fn any_top_level_no_match() {
    let ty = u_many(vec![t_int(), t_string()]);
    assert!(!inspect::any(ty, |e| e.kind() == ElementKind::Float));
}

#[test]
fn any_descends_into_list_element() {
    let list = u(t_list(prelude::TYPE_INT, false));
    assert!(inspect::any(list, |e| e.kind() == ElementKind::Int));
}

#[test]
fn any_descends_into_object_type_args() {
    let generic = u(t_generic_named("Box", vec![prelude::TYPE_STRING]));
    assert!(inspect::any(generic, |e| e.kind() == ElementKind::String));
}

#[test]
fn any_descends_into_iterable_key_and_value() {
    let iter = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    assert!(inspect::any(iter, |e| e.kind() == ElementKind::String));
    assert!(inspect::any(iter, |e| e.kind() == ElementKind::Int));
}

#[test]
fn any_short_circuits_after_first_match() {
    let many = u_many(vec![t_int(), t_string(), t_float()]);
    let mut seen = 0;
    inspect::any(many, |e| {
        seen += 1;
        e.kind() == ElementKind::Int
    });
    assert_eq!(seen, 1);
}

#[test]
fn all_top_level_match() {
    let ty = u_many(vec![t_int(), t_lit_int(42)]);
    assert!(inspect::all(ty, |e| e.kind() == ElementKind::Int));
}

#[test]
fn all_top_level_no_match() {
    let ty = u_many(vec![t_int(), t_string()]);
    assert!(!inspect::all(ty, |e| e.kind() == ElementKind::Int));
}

#[test]
fn all_descends_into_list_element_failure() {
    let list = u(t_list(prelude::TYPE_INT, false));
    // The List element itself isn't Int.
    assert!(!inspect::all(list, |e| e.kind() == ElementKind::Int));
}

#[test]
fn any_descends_into_generic_parameter_constraint() {
    use mago_atom::atom;
    use suffete::ElementId;
    use suffete::element::payload::DefiningEntity;
    let param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("Foo")), prelude::TYPE_STRING);
    let ty = u(param);
    assert!(inspect::any(ty, |e| e.kind() == ElementKind::String));
}
