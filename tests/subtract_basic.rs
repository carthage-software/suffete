//! Subtract (`A \ B`) tests: universal axioms, subsumption-to-bottom,
//! disjoint-as-identity, integer-range splitting, bool / mixed
//! narrowings, and the upper-bound soundness invariant
//! `subtract(A, B) <: A`.

mod comparator_common;

use comparator_common::*;

use suffete::TypeId;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::overlaps;
use suffete::lattice::refines;
use suffete::prelude;
use suffete::subtract;
use suffete::world::World;

fn subtract_of<W: World>(a: TypeId, b: TypeId, world: &W) -> TypeId {
    let mut report = LatticeReport::new();
    subtract::compute(a, b, world, LatticeOptions::default(), &mut report)
}

fn refines_of<W: World>(a: TypeId, b: TypeId, world: &W) -> bool {
    let mut report = LatticeReport::new();
    refines(a, b, world, LatticeOptions::default(), &mut report)
}

fn overlaps_of<W: World>(a: TypeId, b: TypeId, world: &W) -> bool {
    let mut report = LatticeReport::new();
    overlaps(a, b, world, LatticeOptions::default(), &mut report)
}

#[track_caller]
fn assert_upper_bound<W: World>(a: TypeId, b: TypeId, world: &W) {
    let r = subtract_of(a, b, world);
    assert!(refines_of(r, a, world), "subtract({a:?}, {b:?}) = {r:?} does not refine {a:?}");
}

#[test]
fn reflexive_subtract_yields_never() {
    let cb = empty_world();
    assert_eq!(subtract_of(prelude::TYPE_INT, prelude::TYPE_INT, &cb), prelude::TYPE_NEVER);
}

#[test]
fn subtract_never_is_identity() {
    let cb = empty_world();
    assert_eq!(subtract_of(prelude::TYPE_INT, prelude::TYPE_NEVER, &cb), prelude::TYPE_INT);
}

#[test]
fn subtract_from_never_yields_never() {
    let cb = empty_world();
    assert_eq!(subtract_of(prelude::TYPE_NEVER, prelude::TYPE_INT, &cb), prelude::TYPE_NEVER);
}

#[test]
fn subtract_mixed_yields_never() {
    let cb = empty_world();
    assert_eq!(subtract_of(prelude::TYPE_INT, prelude::TYPE_MIXED, &cb), prelude::TYPE_NEVER);
    assert_eq!(subtract_of(prelude::TYPE_STRING, prelude::TYPE_MIXED, &cb), prelude::TYPE_NEVER);
}

#[test]
fn subsumption_collapses_to_never() {
    let cb = empty_world();
    let lit = u(t_lit_int(42));
    assert_eq!(subtract_of(lit, prelude::TYPE_INT, &cb), prelude::TYPE_NEVER);
}

#[test]
fn disjoint_kinds_subtract_is_identity() {
    let cb = empty_world();
    assert_eq!(subtract_of(prelude::TYPE_INT, prelude::TYPE_STRING, &cb), prelude::TYPE_INT);
    assert_eq!(subtract_of(prelude::TYPE_STRING, prelude::TYPE_NULL, &cb), prelude::TYPE_STRING);
}

#[test]
fn nullable_int_minus_null_is_int() {
    let cb = empty_world();
    let nullable_int = u_many(vec![null(), t_int()]);
    assert_eq!(subtract_of(nullable_int, prelude::TYPE_NULL, &cb), prelude::TYPE_INT);
}

#[test]
fn union_minus_union_distributes() {
    let cb = empty_world();
    let int_or_string_or_null = u_many(vec![t_int(), t_string(), null()]);
    let null_or_int = u_many(vec![null(), t_int()]);
    assert_eq!(subtract_of(int_or_string_or_null, null_or_int, &cb), prelude::TYPE_STRING);
}

#[test]
fn int_range_minus_literal_in_middle_splits() {
    let cb = empty_world();
    let r = u(t_int_range(0, 10));
    let lit = u(t_lit_int(5));
    let result = subtract_of(r, lit, &cb);
    let expected = u_many(vec![t_int_range(0, 4), t_int_range(6, 10)]);
    assert_eq!(result, expected);
}

#[test]
fn int_range_minus_overlapping_range_keeps_outer_pieces() {
    let cb = empty_world();
    let outer = u(t_int_range(0, 20));
    let inner = u(t_int_range(5, 15));
    let result = subtract_of(outer, inner, &cb);
    let expected = u_many(vec![t_int_range(0, 4), t_int_range(16, 20)]);
    assert_eq!(result, expected);
}

#[test]
fn int_range_minus_overlapping_left_keeps_right() {
    let cb = empty_world();
    let r = u(t_int_range(0, 10));
    let cut = u(t_int_range(-5, 5));
    let result = subtract_of(r, cut, &cb);
    assert_eq!(result, u(t_int_range(6, 10)));
}

#[test]
fn int_range_minus_overlapping_right_keeps_left() {
    let cb = empty_world();
    let r = u(t_int_range(0, 10));
    let cut = u(t_int_range(5, 15));
    let result = subtract_of(r, cut, &cb);
    assert_eq!(result, u(t_int_range(0, 4)));
}

#[test]
fn int_range_minus_endpoint_collapses_to_literal_piece() {
    let cb = empty_world();
    let r = u(t_int_range(0, 1));
    let zero = u(t_lit_int(0));
    let result = subtract_of(r, zero, &cb);
    assert_eq!(result, u(t_lit_int(1)));
}

#[test]
fn int_range_minus_disjoint_range_is_identity() {
    let cb = empty_world();
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(20, 30));
    assert_eq!(subtract_of(a, b, &cb), a);
}

#[test]
fn open_int_minus_bounded_range_splits_unbounded_pieces() {
    let cb = empty_world();
    let int = prelude::TYPE_INT;
    let middle = u(t_int_range(0, 10));
    let result = subtract_of(int, middle, &cb);
    let expected = u_many(vec![t_int_to(-1), t_int_from(11)]);
    assert_eq!(result, expected);
}

#[test]
fn bool_minus_true_is_false() {
    let cb = empty_world();
    let bool_t = prelude::TYPE_BOOL;
    let true_t = prelude::TYPE_TRUE;
    assert_eq!(subtract_of(bool_t, true_t, &cb), prelude::TYPE_FALSE);
}

#[test]
fn bool_minus_false_is_true() {
    let cb = empty_world();
    let bool_t = prelude::TYPE_BOOL;
    let false_t = prelude::TYPE_FALSE;
    assert_eq!(subtract_of(bool_t, false_t, &cb), prelude::TYPE_TRUE);
}

#[test]
fn nullable_bool_minus_null_is_bool() {
    let cb = empty_world();
    let nullable_bool = u_many(vec![null(), t_bool()]);
    assert_eq!(subtract_of(nullable_bool, prelude::TYPE_NULL, &cb), prelude::TYPE_BOOL);
}

#[test]
fn mixed_minus_null_is_non_null_mixed() {
    let cb = empty_world();
    let result = subtract_of(prelude::TYPE_MIXED, prelude::TYPE_NULL, &cb);
    let expected = u(mixed_nonnull());
    assert_eq!(result, expected);
}

#[test]
fn multi_step_subtraction_chains() {
    let cb = empty_world();
    // (int|string|null) \ null \ string = int
    let three = u_many(vec![t_int(), t_string(), null()]);
    let null_string = u_many(vec![null(), t_string()]);
    assert_eq!(subtract_of(three, null_string, &cb), prelude::TYPE_INT);
}

#[test]
fn upper_bound_invariant_int_split() {
    let cb = empty_world();
    let r = u(t_int_range(0, 10));
    let mid = u(t_lit_int(5));
    assert_upper_bound(r, mid, &cb);
}

#[test]
fn upper_bound_invariant_nullable_int_minus_null() {
    let cb = empty_world();
    let nullable_int = u_many(vec![null(), t_int()]);
    assert_upper_bound(nullable_int, prelude::TYPE_NULL, &cb);
}

#[test]
fn unrelated_named_objects_subtract_is_identity_open_world() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    w.declare("Bar");
    let foo = u(t_named("Foo"));
    let bar = u(t_named("Bar"));
    // Open world: a subclass of Foo could also extend Bar; subtract is conservative.
    assert_eq!(subtract_of(foo, bar, &w), foo);
}

#[test]
fn descendant_minus_ancestor_is_never() {
    let cb = MockWorld::from_edges(&[("Dog", "Animal")]);
    let dog = u(t_named("Dog"));
    let animal = u(t_named("Animal"));
    assert_eq!(subtract_of(dog, animal, &cb), prelude::TYPE_NEVER);
}

#[test]
fn ancestor_minus_descendant_is_identity() {
    let cb = MockWorld::from_edges(&[("Dog", "Animal")]);
    let dog = u(t_named("Dog"));
    let animal = u(t_named("Animal"));
    // Without a closed-world assumption, an `Animal` value might not be a `Dog`,
    // so `Animal \ Dog` returns `Animal` unchanged.
    assert_eq!(subtract_of(animal, dog, &cb), animal);
}

#[test]
fn subtract_then_meet_is_disjoint_for_int_range() {
    let cb = empty_world();
    let r = u(t_int_range(0, 10));
    let mid = u(t_int_range(5, 7));
    let diff = subtract_of(r, mid, &cb);
    // (r \ mid) and mid should be disjoint.
    assert!(!overlaps_of(diff, mid, &cb));
}

#[test]
fn template_with_int_or_string_minus_int_narrows_constraint_to_string() {
    use suffete::FlowFlags;
    use suffete::interner::interner;
    let cb = empty_world();
    let int_or_string = interner().intern_type(&[t_int(), t_string()], FlowFlags::EMPTY);
    let lhs = u(t_template_of("C", "T", int_or_string));
    let expected = u(t_template_of("C", "T", u(t_string())));
    assert_eq!(subtract_of(lhs, prelude::TYPE_INT, &cb), expected);
}

#[test]
fn template_with_int_or_string_minus_string_narrows_constraint_to_int() {
    use suffete::FlowFlags;
    use suffete::interner::interner;
    let cb = empty_world();
    let int_or_string = interner().intern_type(&[t_int(), t_string()], FlowFlags::EMPTY);
    let lhs = u(t_template_of("C", "T", int_or_string));
    let expected = u(t_template_of("C", "T", u(t_int())));
    assert_eq!(subtract_of(lhs, prelude::TYPE_STRING, &cb), expected);
}

#[test]
fn template_with_int_minus_int_is_impossible() {
    let cb = empty_world();
    let lhs = u(t_template_of("C", "T", u(t_int())));
    assert_eq!(subtract_of(lhs, prelude::TYPE_INT, &cb), prelude::TYPE_NEVER);
}

#[test]
fn template_with_int_minus_string_is_redundant_keeps_template() {
    let cb = empty_world();
    let lhs = u(t_template_of("C", "T", u(t_int())));
    assert_eq!(subtract_of(lhs, prelude::TYPE_STRING, &cb), lhs);
}

#[test]
fn same_template_minus_same_template_with_disjoint_constraint_is_identity() {
    let cb = empty_world();
    let lhs = u(t_template_of("C", "T", u(t_int())));
    let rhs = u(t_template_of("C", "T", u(t_string())));
    assert_eq!(subtract_of(lhs, rhs, &cb), lhs);
}

#[test]
fn same_template_minus_same_template_with_subset_constraint_is_impossible() {
    let cb = empty_world();
    let lhs = u(t_template_of("C", "T", u(t_int())));
    let rhs = u(t_template_of("C", "T", prelude::TYPE_MIXED));
    assert_eq!(subtract_of(lhs, rhs, &cb), prelude::TYPE_NEVER);
}
