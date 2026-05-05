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
use suffete::lattice::refines;
use suffete::meet;
use suffete::prelude;
use suffete::subtract;

fn create_sealed_world() -> MockWorld {
    let mut w = MockWorld::new();
    w.add_edge("Error", "Throwable")
        .add_edge("Exception", "Throwable")
        .add_edge("RuntimeException", "Exception")
        .add_edge("LogicException", "Exception")
        .with_sealed("Throwable", &["Error", "Exception"]);
    w
}

fn u(e: ElementId) -> TypeId {
    interner().intern_type(&[e], FlowFlags::EMPTY)
}

fn refines_of<W: suffete::world::World>(a: TypeId, b: TypeId, w: &W) -> bool {
    let mut report = LatticeReport::new();
    refines(a, b, w, LatticeOptions::default(), &mut report)
}

fn meet_of<W: suffete::world::World>(a: TypeId, b: TypeId, w: &W) -> TypeId {
    let mut report = LatticeReport::new();
    meet::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn subtract_of<W: suffete::world::World>(a: TypeId, b: TypeId, w: &W) -> TypeId {
    let mut report = LatticeReport::new();
    subtract::compute(a, b, w, LatticeOptions::default(), &mut report)
}

// ---------------------------------------------------------------------------
// Identity 1: (Throwable & !Exception) ≡ Error
// ---------------------------------------------------------------------------

#[test]
fn throwable_minus_exception_refines_error() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let error = ElementId::object_named("Error");

    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);

    assert!(refines_of(u(t_minus_e), u(error), &w));
}

#[test]
fn error_refines_throwable_minus_exception() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let error = ElementId::object_named("Error");

    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);

    assert!(refines_of(u(error), u(t_minus_e), &w));
}

// ---------------------------------------------------------------------------
// Identity 2: Throwable ≡ Error | Exception
// ---------------------------------------------------------------------------

#[test]
fn throwable_refines_error_or_exception_union() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let union = TypeId::union(&[error, exception]);
    assert!(refines_of(u(throwable), union, &w));
}

#[test]
fn error_or_exception_union_refines_throwable() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let union = TypeId::union(&[error, exception]);
    assert!(refines_of(union, u(throwable), &w));
}

// ---------------------------------------------------------------------------
// Identity 3: Throwable & !Error & !Exception ≡ never
// ---------------------------------------------------------------------------

#[test]
fn throwable_with_full_negations_is_uninhabited() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let t_no_err_no_exc =
        ElementId::intersected(throwable, &[ElementId::negated(u(error)), ElementId::negated(u(exception))]);

    assert!(is_uninhabited(t_no_err_no_exc, &w));
}

#[test]
fn throwable_with_full_negations_meet_anything_is_never() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let t_no_err_no_exc =
        ElementId::intersected(throwable, &[ElementId::negated(u(error)), ElementId::negated(u(exception))]);

    assert_eq!(meet_of(u(t_no_err_no_exc), prelude::TYPE_MIXED, &w), prelude::TYPE_NEVER);
}

#[test]
fn subtract_throwable_by_error_or_exception_is_never() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let error = ElementId::object_named("Error");
    let exception = ElementId::object_named("Exception");

    let union = TypeId::union(&[error, exception]);
    assert_eq!(subtract_of(u(throwable), union, &w), prelude::TYPE_NEVER);
}

// ---------------------------------------------------------------------------
// Traversable / Iterator / IteratorAggregate
// ---------------------------------------------------------------------------

fn create_traversable_world() -> MockWorld {
    let mut w = MockWorld::new();
    w.add_edge("Iterator", "Traversable")
        .add_edge("IteratorAggregate", "Traversable")
        .with_sealed("Traversable", &["Iterator", "IteratorAggregate"]);
    w
}

#[test]
fn traversable_refines_iterator_or_iterator_aggregate() {
    let w = create_traversable_world();
    let traversable = ElementId::object_named("Traversable");
    let it = ElementId::object_named("Iterator");
    let ia = ElementId::object_named("IteratorAggregate");

    let union = TypeId::union(&[it, ia]);
    assert!(refines_of(u(traversable), union, &w));
}

#[test]
fn traversable_with_full_negations_is_uninhabited() {
    let w = create_traversable_world();
    let traversable = ElementId::object_named("Traversable");
    let it = ElementId::object_named("Iterator");
    let ia = ElementId::object_named("IteratorAggregate");

    let t_no_it_no_ia = ElementId::intersected(traversable, &[ElementId::negated(u(it)), ElementId::negated(u(ia))]);

    assert!(is_uninhabited(t_no_it_no_ia, &w));
}

// ---------------------------------------------------------------------------
// Partial cover: Throwable & !Exception ≠ Error (there are other Throwable)
// ---------------------------------------------------------------------------

#[test]
fn partial_cover_does_not_collapse_when_residual_has_multiple() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");

    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);

    // Throwable & !Exception is NOT uninhabited — Error survives
    assert!(!is_uninhabited(t_minus_e, &w));
}

// ---------------------------------------------------------------------------
// Transitive negation cover: Throwable & !Exception covers RuntimeException
// because RuntimeException refines Exception
// ---------------------------------------------------------------------------

#[test]
fn transitive_negation_via_descendant() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");
    let rte = ElementId::object_named("RuntimeException");

    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);

    // RuntimeException should be excluded by !Exception (it refines Exception)
    assert!(is_uninhabited(ElementId::intersected(t_minus_e, &[rte]), &w));
}

// ---------------------------------------------------------------------------
// Recursive sealing
// ---------------------------------------------------------------------------

#[test]
fn transitive_sealing_collapses_to_never() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo")
        .add_edge("Baz", "Foo")
        .add_edge("Bar1", "Bar")
        .add_edge("Bar2", "Bar")
        .with_sealed("Foo", &["Bar", "Baz"])
        .with_sealed("Bar", &["Bar1", "Bar2"]);

    let foo = ElementId::object_named("Foo");
    let bar1 = ElementId::object_named("Bar1");
    let bar2 = ElementId::object_named("Bar2");
    let baz = ElementId::object_named("Baz");

    let covered = ElementId::intersected(
        foo,
        &[ElementId::negated(u(bar1)), ElementId::negated(u(bar2)), ElementId::negated(u(baz))],
    );

    assert!(is_uninhabited(covered, &w));
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn unrelated_negation_does_not_affect_cover() {
    let w = create_sealed_world();
    let throwable = ElementId::object_named("Throwable");

    // Throwable & !int — int is not in the cover, so coverage is unaffected
    let t_no_int = ElementId::intersected(throwable, &[ElementId::negated(prelude::TYPE_INT)]);

    // Not uninhabited — int doesn't cover any inheritor
    assert!(!is_uninhabited(t_no_int, &w));
}

#[test]
fn non_class_head_skips_sealed_logic() {
    let w = create_sealed_world();
    // int & !Exception — int is not a sealed class
    let x = ElementId::intersected(prelude::INT, &[ElementId::negated(u(ElementId::object_named("Exception")))]);
    // Should not panic, and should not be fully covered
    assert!(!is_uninhabited(x, &w));
}

#[test]
fn null_world_returns_no_sealed_inheritors() {
    let w = empty_world();
    let throwable = ElementId::object_named("Throwable");
    let exception = ElementId::object_named("Exception");

    let t_minus_e = ElementId::intersected(throwable, &[ElementId::negated(u(exception))]);

    // With NullWorld, Throwable is not sealed — not uninhabited
    assert!(!is_uninhabited(t_minus_e, &w));
}

#[test]
fn final_class_with_negated_self_is_being_uninhabited_by_existing_rules() {
    let _w = MockWorld::new().add_edge("Final", "Final");
    let final_cls = ElementId::object_named("Final");
    // Check that !Final & Final is uninhabited (existing rules, not sealed)
    let x = ElementId::intersected(final_cls, &[ElementId::negated(u(final_cls))]);
    // self-negation always empty via intersected construction
    assert_eq!(x, prelude::NEVER);
}
