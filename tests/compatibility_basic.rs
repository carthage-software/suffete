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
use suffete::compatibility::runtime_compatible;
use suffete::compatibility::statically_compatible;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::world::Variance;

fn statically<W: suffete::world::World>(a: suffete::TypeId, b: suffete::TypeId, world: &W) -> bool {
    let mut r = LatticeReport::new();
    statically_compatible(a, b, world, LatticeOptions::default(), &mut r)
}

fn at_runtime<W: suffete::world::World>(a: suffete::TypeId, b: suffete::TypeId, world: &W) -> bool {
    let mut r = LatticeReport::new();
    runtime_compatible(a, b, world, LatticeOptions::default(), &mut r)
}

#[test]
fn primitives_int_and_string_are_incompatible_under_both() {
    let cb = empty_world();
    let a = u(t_int());
    let b = u(t_string());
    assert!(!statically(a, b, &cb));
    assert!(!at_runtime(a, b, &cb));
}

#[test]
fn array_key_and_string_compatible_under_both() {
    let cb = empty_world();
    let a = u(t_array_key());
    let b = u(t_string());
    assert!(statically(a, b, &cb));
    assert!(at_runtime(a, b, &cb));
}

#[test]
fn numeric_and_string_compatible_under_both() {
    let cb = empty_world();
    let a = u(t_numeric());
    let b = u(t_string());
    assert!(statically(a, b, &cb));
    assert!(at_runtime(a, b, &cb));
}

#[test]
fn never_and_anything_incompatible_under_both() {
    let cb = empty_world();
    let n = u(never());
    let i = u(t_int());
    assert!(!statically(n, i, &cb));
    assert!(!at_runtime(n, i, &cb));
}

#[test]
fn cell_int_and_cell_string_diverge_static_vs_runtime() {
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);

    let cell_int = u(t_generic_named("Cell", vec![u(t_int())]));
    let cell_string = u(t_generic_named("Cell", vec![u(t_string())]));

    assert!(!statically(cell_int, cell_string, &w));
    assert!(at_runtime(cell_int, cell_string, &w));
}

#[test]
fn cell_int_and_box_string_unrelated_classes_incompatible() {
    // Mark both `Cell` and `Box` as `final` ; without that, PHP's
    // open class graph admits a hypothetical descendant that
    // satisfies both, and the lattice correctly reports that as a
    // possible runtime witness. Final closes the door, giving the
    // strict "no common subclass" answer this test wants.
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    w.with_final("Cell");
    w.with_final("Box");

    let cell_int = u(t_generic_named("Cell", vec![u(t_int())]));
    let box_string = u(t_generic_named("Box", vec![u(t_string())]));

    assert!(!statically(cell_int, box_string, &w));
    assert!(!at_runtime(cell_int, box_string, &w));
}

#[test]
fn intersection_runtime_compatible_with_each_conjunct() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    w.declare("Bar");

    let foo_and_bar = u(t_named_intersected("Foo", &[t_named("Bar")]));
    let foo = u(t_named("Foo"));
    let bar = u(t_named("Bar"));

    assert!(at_runtime(foo_and_bar, foo, &w));
    assert!(at_runtime(foo_and_bar, bar, &w));
}

#[test]
fn descendant_classes_compatible_under_both() {
    let mut w = MockWorld::new();
    w.add_edge("Dog", "Animal");

    let dog = u(t_named("Dog"));
    let animal = u(t_named("Animal"));

    assert!(statically(dog, animal, &w));
    assert!(at_runtime(dog, animal, &w));
}

#[test]
fn object_any_runtime_compatible_with_any_named_class() {
    let mut w = MockWorld::new();
    w.declare("Foo");

    let any = u(t_object_any());
    let foo = u(t_named("Foo"));

    assert!(at_runtime(any, foo, &w));
    assert!(at_runtime(foo, any, &w));
}

#[test]
fn has_method_runtime_compatible_with_any_object() {
    let mut w = MockWorld::new();
    w.declare("Foo");

    let h = u(t_has_method("doStuff"));
    let foo = u(t_named("Foo"));

    assert!(at_runtime(h, foo, &w));
}

#[test]
fn cross_family_object_vs_int_incompatible_at_runtime() {
    let mut w = MockWorld::new();
    w.declare("Foo");

    let foo = u(t_named("Foo"));
    let int = u(t_int());
    assert!(!at_runtime(foo, int, &w));
    assert!(!at_runtime(int, foo, &w));
}

#[test]
fn enum_and_named_class_unrelated_incompatible() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Status");
    w.declare("Foo");

    let status = u(t_enum("Status"));
    let foo = u(t_named("Foo"));

    assert!(!statically(status, foo, &w));
    assert!(!at_runtime(status, foo, &w));
}

#[test]
fn union_distribution_static_and_runtime() {
    let cb = empty_world();
    let int_or_string = u_many(vec![t_int(), t_string()]);
    let int_only = u(t_int());

    assert!(statically(int_or_string, int_only, &cb));
    assert!(at_runtime(int_or_string, int_only, &cb));
}
