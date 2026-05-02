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

use mago_atom::atom;

use suffete::ElementId;
use suffete::element::payload::DefiningEntity;
use suffete::prelude;

fn t_param_with_constraint(class_name: &str, template_name: &str, constraint: suffete::TypeId) -> ElementId {
    ElementId::generic_parameter(template_name, DefiningEntity::ClassLike(atom(class_name)), constraint)
}

#[test]
fn same_t_reflexive_under_same_defining_entity() {
    let t1 = t_template("Box", "T");
    let t2 = t_template("Box", "T");
    assert!(atomic_is_contained(t1, t2, &empty_world()));
}

#[test]
fn different_defining_entities_not_same_t() {
    let box_t = t_template("Box", "T");
    let bag_t = t_template("Bag", "T");
    assert!(!atomic_is_contained(box_t, bag_t, &empty_world()));
}

#[test]
fn different_parameter_names_same_class_not_same_t() {
    let box_t = t_template("Box", "T");
    let box_u = t_template("Box", "U");
    assert!(!atomic_is_contained(box_t, box_u, &empty_world()));
}

#[test]
fn template_with_mixed_constraint_refines_mixed() {
    let t = t_template("Box", "T");
    assert!(atomic_is_contained(t, mixed(), &empty_world()));
}

#[test]
fn template_with_int_constraint_refines_int() {
    let t = t_param_with_constraint("Box", "T", u(t_int()));
    assert!(atomic_is_contained(t, t_int(), &empty_world()));
}

#[test]
fn template_with_int_constraint_refines_array_key() {
    let t = t_param_with_constraint("Box", "T", u(t_int()));
    assert!(atomic_is_contained(t, t_array_key(), &empty_world()));
}

#[test]
fn template_with_int_constraint_does_not_refine_string() {
    let t = t_param_with_constraint("Box", "T", u(t_int()));
    assert!(!atomic_is_contained(t, t_string(), &empty_world()));
}

#[test]
fn concrete_value_does_not_refine_template_parameter() {
    let t = t_template("Box", "T");
    assert!(!atomic_is_contained(t_int(), t, &empty_world()));
    assert!(!atomic_is_contained(t_string(), t, &empty_world()));
}

#[test]
fn template_self_refines_via_mixed_top() {
    let t = t_template("Box", "T");
    assert!(atomic_is_contained(t, t, &empty_world()));
    assert!(atomic_is_contained(t, mixed(), &empty_world()));
}

#[test]
fn template_with_named_constraint_refines_ancestor() {
    let cb = MockWorld::from_edges(&[("Dog", "Animal")]);
    let t = t_param_with_constraint("Owner", "T", u(t_named("Dog")));
    assert!(atomic_is_contained(t, t_named("Animal"), &cb));
    assert!(!atomic_is_contained(t, t_named("Cat"), &cb));
}

#[test]
fn never_refines_any_template() {
    let t = t_template("Box", "T");
    let never_t = prelude::TYPE_NEVER;
    assert!(is_contained(never_t, u(t), &empty_world()));
}
