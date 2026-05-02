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

#[test]
fn intersected_input_refines_each_conjunct() {
    let cb = MockWorld::from_edges(&[]);
    let foo_and_bar = t_named_intersected("Foo", &[t_named("Bar")]);
    assert!(atomic_is_contained(foo_and_bar, t_named("Foo"), &cb));
    assert!(atomic_is_contained(foo_and_bar, t_named("Bar"), &cb));
}

#[test]
fn intersected_input_does_not_refine_unrelated_class() {
    let cb = MockWorld::from_edges(&[]);
    let foo_and_bar = t_named_intersected("Foo", &[t_named("Bar")]);
    assert!(!atomic_is_contained(foo_and_bar, t_named("Quux"), &cb));
}

#[test]
fn intersected_input_refines_ancestor_of_any_conjunct() {
    let cb = MockWorld::from_edges(&[("Bar", "BarAncestor")]);
    let foo_and_bar = t_named_intersected("Foo", &[t_named("Bar")]);
    assert!(atomic_is_contained(foo_and_bar, t_named("BarAncestor"), &cb));
}

#[test]
fn input_must_refine_every_conjunct_of_intersected_container() {
    let cb = MockWorld::from_edges(&[("Foo", "Bar"), ("Foo", "Baz")]);
    let bar_and_baz = t_named_intersected("Bar", &[t_named("Baz")]);
    assert!(atomic_is_contained(t_named("Foo"), bar_and_baz, &cb));
}

#[test]
fn input_missing_one_conjunct_fails_intersected_container() {
    let cb = MockWorld::from_edges(&[("Foo", "Bar")]);
    let bar_and_baz = t_named_intersected("Bar", &[t_named("Baz")]);
    assert!(!atomic_is_contained(t_named("Foo"), bar_and_baz, &cb));
}

#[test]
fn intersected_input_into_intersected_container_both_directions() {
    let cb = MockWorld::from_edges(&[]);
    let lhs = t_named_intersected("Foo", &[t_named("Bar")]);
    let rhs = t_named_intersected("Foo", &[t_named("Bar")]);
    assert!(atomic_is_contained(lhs, rhs, &cb));
}

#[test]
fn static_container_rejects_plain_named_input() {
    let cb = MockWorld::from_edges(&[]);
    let plain_foo = t_named("Foo");
    let static_foo = t_named_static("Foo");
    assert!(!atomic_is_contained(plain_foo, static_foo, &cb));
}

#[test]
fn static_container_accepts_static_input() {
    let cb = MockWorld::from_edges(&[]);
    let static_foo = t_named_static("Foo");
    assert!(atomic_is_contained(static_foo, static_foo, &cb));
}

#[test]
fn this_container_accepts_only_this_input() {
    let cb = MockWorld::from_edges(&[]);
    let this_foo = t_named_this("Foo");
    let static_foo = t_named_static("Foo");
    let plain_foo = t_named("Foo");
    assert!(atomic_is_contained(this_foo, this_foo, &cb));
    assert!(!atomic_is_contained(static_foo, this_foo, &cb));
    assert!(!atomic_is_contained(plain_foo, this_foo, &cb));
}

#[test]
fn this_input_refines_static_container() {
    let cb = MockWorld::from_edges(&[]);
    let this_foo = t_named_this("Foo");
    let static_foo = t_named_static("Foo");
    assert!(atomic_is_contained(this_foo, static_foo, &cb));
}

#[test]
fn static_input_refines_plain_named_container() {
    let cb = MockWorld::from_edges(&[]);
    let static_foo = t_named_static("Foo");
    let plain_foo = t_named("Foo");
    assert!(atomic_is_contained(static_foo, plain_foo, &cb));
}
