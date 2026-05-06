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
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::is_uninhabited;
use suffete::meet;
use suffete::prelude;
use suffete::subtract;

fn u(e: ElementId) -> TypeId {
    interner().intern_type(&[e], FlowFlags::EMPTY)
}

fn meet_of<W: suffete::world::World>(a: TypeId, b: TypeId, w: &W) -> TypeId {
    let mut report = LatticeReport::new();
    meet::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn subtract_of<W: suffete::world::World>(a: TypeId, b: TypeId, w: &W) -> TypeId {
    let mut report = LatticeReport::new();
    subtract::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn sealed_throwable_world() -> MockWorld {
    let mut w = MockWorld::new();
    w.add_edge("Error", "Throwable")
        .add_edge("Exception", "Throwable")
        .add_edge("RuntimeException", "Exception")
        .with_sealed("Throwable", &["Error", "Exception"]);
    w
}

#[test]
fn subtract_throwable_by_exception_canonicalises_to_error() {
    let w = sealed_throwable_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let error = ElementId::object_named("Error");

    let result = subtract_of(u(throwable), u(exception), &w);
    assert_eq!(result, u(error));
}

#[test]
fn subtract_throwable_by_error_canonicalises_to_exception() {
    let w = sealed_throwable_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let result = subtract_of(u(throwable), u(error), &w);
    assert_eq!(result, u(exception));
}

#[test]
fn meet_throwable_with_negated_exception_canonicalises_to_error() {
    let w = sealed_throwable_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let error = ElementId::object_named("Error");

    let t_no_exc = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);
    let result = meet_of(u(t_no_exc), u(error), &w);
    assert_eq!(result, u(error));
}

#[test]
fn subtract_throwable_by_error_or_exception_is_never() {
    let w = sealed_throwable_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let union = TypeId::union(&[error, exception]);
    assert_eq!(subtract_of(u(throwable), union, &w), prelude::TYPE_NEVER);
}

#[test]
fn traversable_minus_iterator_minus_iterator_aggregate_is_never() {
    let mut w = MockWorld::new();
    w.add_edge("Iterator", "Traversable")
        .add_edge("IteratorAggregate", "Traversable")
        .with_sealed("Traversable", &["Iterator", "IteratorAggregate"]);

    let traversable = ElementId::object_named("Traversable");
    let iterator = ElementId::object_named("Iterator");
    let ia = ElementId::object_named("IteratorAggregate");

    let after_it = subtract_of(u(traversable), u(iterator), &w);
    let final_result = subtract_of(after_it, u(ia), &w);
    assert_eq!(final_result, prelude::TYPE_NEVER);
}

#[test]
fn partial_cover_does_not_collapse_when_residual_has_multiple() {
    let mut w = MockWorld::new();
    w.add_edge("A", "Foo").add_edge("B", "Foo").add_edge("C", "Foo").with_sealed("Foo", &["A", "B", "C"]);

    let foo = ElementId::object_named("Foo");
    let a = ElementId::object_named("A");
    let foo_no_a = ElementId::intersected(foo, &[ElementId::negated(u(a))]);

    assert!(!is_uninhabited(foo_no_a, &w));
}

#[test]
fn cycle_with_direct_coverage_collapses() {
    let mut w = MockWorld::new();
    w.add_edge("B", "A").add_edge("A", "B").with_sealed("A", &["B"]).with_sealed("B", &["A"]);

    let a = ElementId::object_named("A");
    let b = ElementId::object_named("B");
    let a_no_b = ElementId::intersected(a, &[ElementId::negated(u(b))]);

    assert!(is_uninhabited(a_no_b, &w));
}

#[test]
fn cycle_without_direct_coverage_terminates() {
    let mut w = MockWorld::new();
    w.add_edge("B", "A").add_edge("A", "B").with_sealed("A", &["B"]).with_sealed("B", &["A"]);

    let a = ElementId::object_named("A");
    let unrelated = ElementId::object_named("Unrelated");
    let a_no_unrelated = ElementId::intersected(a, &[ElementId::negated(u(unrelated))]);

    assert!(!is_uninhabited(a_no_unrelated, &w));
}

#[test]
fn depth_cap_does_not_overflow() {
    let mut w = MockWorld::new();
    for i in 0..20 {
        let parent = format!("S{}", i);
        let child = format!("S{}", i + 1);
        w.add_edge(&child, &parent);
    }
    w.with_sealed("S0", &["S1"]);

    let s0 = ElementId::object_named("S0");
    let s21 = ElementId::object_named("S21");
    let s0_no_s21 = ElementId::intersected(s0, &[ElementId::negated(u(s21))]);

    assert!(!is_uninhabited(s0_no_s21, &w));
}

#[test]
fn final_class_with_negated_self_is_never() {
    let final_cls = ElementId::object_named("Final");
    let x = ElementId::intersected(final_cls, &[ElementId::negated(u(final_cls))]);
    assert_eq!(x, prelude::NEVER);
}

#[test]
fn non_class_head_skips_sealed_logic() {
    let w = sealed_throwable_world();
    let x = ElementId::intersected(prelude::INT, &[ElementId::negated(u(ElementId::object_named("Exception")))]);
    assert!(!is_uninhabited(x, &w));
}

#[test]
fn null_world_returns_no_sealed_inheritors() {
    let w = empty_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);
    assert!(!is_uninhabited(t_minus_e, &w));
}
