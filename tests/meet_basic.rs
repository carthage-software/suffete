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
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::refines;
use suffete::meet;
use suffete::prelude;
use suffete::world::World;

fn meet_of<W: World>(a: TypeId, b: TypeId, world: &W) -> TypeId {
    let mut report = LatticeReport::new();
    meet::compute(a, b, world, LatticeOptions::default(), &mut report)
}

fn refines_of<W: World>(a: TypeId, b: TypeId, world: &W) -> bool {
    let mut report = LatticeReport::new();
    refines(a, b, world, LatticeOptions::default(), &mut report)
}

/// Lower-bound property: `meet(a, b)` refines both `a` and `b`.
#[track_caller]
fn assert_lower_bound<W: World>(a: TypeId, b: TypeId, world: &W) {
    let m = meet_of(a, b, world);
    assert!(refines_of(m, a, world), "meet({a:?}, {b:?}) = {m:?} does not refine {a:?}");
    assert!(refines_of(m, b, world), "meet({a:?}, {b:?}) = {m:?} does not refine {b:?}");
}

#[test]
fn reflexive_meet() {
    let cb = empty_world();
    assert_eq!(meet_of(prelude::TYPE_INT, prelude::TYPE_INT, &cb), prelude::TYPE_INT);
}

#[test]
fn meet_with_mixed_yields_other() {
    let cb = empty_world();
    assert_eq!(meet_of(prelude::TYPE_MIXED, prelude::TYPE_INT, &cb), prelude::TYPE_INT);
    assert_eq!(meet_of(prelude::TYPE_INT, prelude::TYPE_MIXED, &cb), prelude::TYPE_INT);
}

#[test]
fn meet_with_never_yields_never() {
    let cb = empty_world();
    assert_eq!(meet_of(prelude::TYPE_NEVER, prelude::TYPE_INT, &cb), prelude::TYPE_NEVER);
    assert_eq!(meet_of(prelude::TYPE_INT, prelude::TYPE_NEVER, &cb), prelude::TYPE_NEVER);
}

#[test]
fn subsumption_picks_more_specific_side() {
    let cb = empty_world();
    let lit = u(t_lit_int(42));
    let int = prelude::TYPE_INT;
    assert_eq!(meet_of(lit, int, &cb), lit);
    assert_eq!(meet_of(int, lit, &cb), lit);
}

#[test]
fn distinct_kinds_meet_to_never() {
    let cb = empty_world();
    assert_eq!(meet_of(prelude::TYPE_INT, prelude::TYPE_STRING, &cb), prelude::TYPE_NEVER);
    assert_eq!(meet_of(prelude::TYPE_INT, prelude::TYPE_NULL, &cb), prelude::TYPE_NEVER);
}

#[test]
fn overlapping_int_ranges_meet_to_intersection() {
    let cb = empty_world();
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(5, 15));
    let m = meet_of(a, b, &cb);
    assert_eq!(m, u(t_int_range(5, 10)));
    assert_lower_bound(a, b, &cb);
}

#[test]
fn touching_int_ranges_collapse_to_literal() {
    let cb = empty_world();
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(10, 20));
    let m = meet_of(a, b, &cb);
    assert_eq!(m, u(t_lit_int(10)));
}

#[test]
fn disjoint_int_ranges_meet_to_never() {
    let cb = empty_world();
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(20, 30));
    assert_eq!(meet_of(a, b, &cb), prelude::TYPE_NEVER);
}

#[test]
fn lit_int_in_range_meets_to_lit() {
    let cb = empty_world();
    let lit = u(t_lit_int(5));
    let r = u(t_int_range(0, 10));
    assert_eq!(meet_of(lit, r, &cb), lit);
}

#[test]
fn lit_int_outside_range_meets_to_never() {
    let cb = empty_world();
    let lit = u(t_lit_int(20));
    let r = u(t_int_range(0, 10));
    assert_eq!(meet_of(lit, r, &cb), prelude::TYPE_NEVER);
}

#[test]
fn distinct_int_literals_meet_to_never() {
    let cb = empty_world();
    let a = u(t_lit_int(1));
    let b = u(t_lit_int(2));
    assert_eq!(meet_of(a, b, &cb), prelude::TYPE_NEVER);
}

#[test]
fn open_lower_meets_open_upper_into_bounded_range() {
    let cb = empty_world();
    let from_zero = u(t_int_from(0));
    let to_ten = u(t_int_to(10));
    let m = meet_of(from_zero, to_ten, &cb);
    assert_eq!(m, u(t_int_range(0, 10)));
}

#[test]
fn nominal_subsumption_meet_picks_descendant() {
    let cb = MockWorld::from_edges(&[("Dog", "Animal")]);
    let dog = u(t_named("Dog"));
    let animal = u(t_named("Animal"));
    assert_eq!(meet_of(dog, animal, &cb), dog);
    assert_eq!(meet_of(animal, dog, &cb), dog);
}

#[test]
fn unrelated_named_objects_compose_intersection() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    w.declare("Bar");
    let foo = u(t_named("Foo"));
    let bar = u(t_named("Bar"));
    let m = meet_of(foo, bar, &w);
    // Result should refine both Foo and Bar via the Int-L rule.
    assert!(refines_of(m, foo, &w));
    assert!(refines_of(m, bar, &w));
    assert_lower_bound(foo, bar, &w);
}

#[test]
fn union_meet_distributes() {
    let cb = empty_world();
    let int_or_string = u_many(vec![t_int(), t_string()]);
    let int_or_null = u_many(vec![t_int(), null()]);
    // (int|string) ∧ (int|null) = int.
    assert_eq!(meet_of(int_or_string, int_or_null, &cb), prelude::TYPE_INT);
}

#[test]
fn union_meet_yields_union_when_multiple_pairs_survive() {
    let cb = empty_world();
    let int_or_string = u_many(vec![t_int(), t_string()]);
    let m = meet_of(int_or_string, int_or_string, &cb);
    // (int|string) ∧ (int|string) = int|string (each survives via reflexivity).
    assert_eq!(m, int_or_string);
}

#[test]
fn nullable_int_meet_int_drops_null() {
    let cb = empty_world();
    let nullable_int = u_many(vec![null(), t_int()]);
    let int = prelude::TYPE_INT;
    assert_eq!(meet_of(nullable_int, int, &cb), int);
}

#[test]
fn class_like_string_meet_string_picks_class_like_string() {
    let cb = empty_world();
    let cls = u(t_class_string());
    let s = prelude::TYPE_STRING;
    assert_eq!(meet_of(cls, s, &cb), cls);
}
