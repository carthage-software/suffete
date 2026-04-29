//! `meet::narrow` and `subtract::narrow` outcome classification tests.
//! Each variant (`Impossible` / `Redundant` / `Narrowed`) is exercised
//! against representative inputs so a downstream analyser can rely on
//! the diagnostics contract: redundant means the assertion adds no
//! information; impossible means the assertion can never hold.

mod comparator_common;

use comparator_common::*;

use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::meet;
use suffete::meet::MeetOutcome;
use suffete::prelude;
use suffete::subtract;
use suffete::subtract::SubtractOutcome;

fn meet_narrow(input: suffete::TypeId, narrowing: suffete::TypeId) -> MeetOutcome {
    let cb = empty_world();
    let mut report = LatticeReport::new();
    meet::narrow(input, narrowing, &cb, LatticeOptions::default(), &mut report)
}

fn subtract_narrow(input: suffete::TypeId, narrowing: suffete::TypeId) -> SubtractOutcome {
    let cb = empty_world();
    let mut report = LatticeReport::new();
    subtract::narrow(input, narrowing, &cb, LatticeOptions::default(), &mut report)
}

#[test]
fn meet_narrow_redundant_when_input_equals_narrowing() {
    let r = meet_narrow(prelude::TYPE_INT, prelude::TYPE_INT);
    assert_eq!(r, MeetOutcome::Redundant(prelude::TYPE_INT));
}

#[test]
fn meet_narrow_redundant_when_input_refines_narrowing() {
    let lit = u(t_lit_int(42));
    let r = meet_narrow(lit, prelude::TYPE_INT);
    assert_eq!(r, MeetOutcome::Redundant(lit));
}

#[test]
fn meet_narrow_narrowed_when_strictly_smaller() {
    // (int|null) ∧ int = int (strictly narrower than int|null).
    let nullable_int = u_many(vec![null(), t_int()]);
    let r = meet_narrow(nullable_int, prelude::TYPE_INT);
    assert_eq!(r, MeetOutcome::Narrowed(prelude::TYPE_INT));
}

#[test]
fn meet_narrow_narrowed_via_subsumption_other_direction() {
    // int ∧ literal-42 = literal-42 (strictly narrower).
    let lit = u(t_lit_int(42));
    let r = meet_narrow(prelude::TYPE_INT, lit);
    assert_eq!(r, MeetOutcome::Narrowed(lit));
}

#[test]
fn meet_narrow_impossible_when_disjoint() {
    let r = meet_narrow(prelude::TYPE_INT, prelude::TYPE_STRING);
    assert_eq!(r, MeetOutcome::Impossible);
}

#[test]
fn meet_narrow_impossible_for_unrelated_named_objects_with_no_descend() {
    // Two declared classes with no relationship and no overlap rule
    // → meet conservatively returns the compositional intersection
    //   if both are objects, but for our specific test we use kinds
    //   that family rules don't cover (e.g. a list and a string).
    let cb = empty_world();
    let mut report = LatticeReport::new();
    let r = meet::narrow(
        u(t_list(prelude::TYPE_INT, false)),
        prelude::TYPE_STRING,
        &cb,
        LatticeOptions::default(),
        &mut report,
    );
    assert_eq!(r, MeetOutcome::Impossible);
}

#[test]
fn meet_narrow_int_range_overlap_is_narrowed() {
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(5, 15));
    let r = meet_narrow(a, b);
    let expected = u(t_int_range(5, 10));
    assert_eq!(r, MeetOutcome::Narrowed(expected));
}

#[test]
fn meet_narrow_int_range_disjoint_is_impossible() {
    let a = u(t_int_range(0, 10));
    let b = u(t_int_range(20, 30));
    assert_eq!(meet_narrow(a, b), MeetOutcome::Impossible);
}

#[test]
fn meet_compute_returns_never_when_impossible() {
    let r = meet::compute(
        prelude::TYPE_INT,
        prelude::TYPE_STRING,
        &empty_world(),
        LatticeOptions::default(),
        &mut LatticeReport::new(),
    );
    assert_eq!(r, prelude::TYPE_NEVER);
}

#[test]
fn meet_compute_unwraps_redundant() {
    let r = meet::compute(
        prelude::TYPE_INT,
        prelude::TYPE_INT,
        &empty_world(),
        LatticeOptions::default(),
        &mut LatticeReport::new(),
    );
    assert_eq!(r, prelude::TYPE_INT);
}

#[test]
fn subtract_narrow_impossible_when_input_equals_narrowing() {
    let r = subtract_narrow(prelude::TYPE_INT, prelude::TYPE_INT);
    assert_eq!(r, SubtractOutcome::Impossible);
}

#[test]
fn subtract_narrow_impossible_when_input_refines_narrowing() {
    // Removing `int` from `lit-42` leaves nothing: lit-42 is fully in int.
    let lit = u(t_lit_int(42));
    let r = subtract_narrow(lit, prelude::TYPE_INT);
    assert_eq!(r, SubtractOutcome::Impossible);
}

#[test]
fn subtract_narrow_redundant_when_disjoint() {
    // Removing `string` from `int` is identity: int and string are disjoint.
    let r = subtract_narrow(prelude::TYPE_INT, prelude::TYPE_STRING);
    assert_eq!(r, SubtractOutcome::Redundant(prelude::TYPE_INT));
}

#[test]
fn subtract_narrow_narrowed_when_strictly_smaller() {
    // (int|null) \ null = int.
    let nullable_int = u_many(vec![null(), t_int()]);
    let r = subtract_narrow(nullable_int, prelude::TYPE_NULL);
    assert_eq!(r, SubtractOutcome::Narrowed(prelude::TYPE_INT));
}

#[test]
fn subtract_narrow_narrowed_for_int_range_split() {
    let r_ty = u(t_int_range(0, 10));
    let mid = u(t_lit_int(5));
    let r = subtract_narrow(r_ty, mid);
    let expected = u_many(vec![t_int_range(0, 4), t_int_range(6, 10)]);
    assert_eq!(r, SubtractOutcome::Narrowed(expected));
}

#[test]
fn subtract_narrow_impossible_when_int_range_fully_covered() {
    // int<5,7> \ int<0,10> → impossible (input fully refines narrowing).
    let small = u(t_int_range(5, 7));
    let big = u(t_int_range(0, 10));
    assert_eq!(subtract_narrow(small, big), SubtractOutcome::Impossible);
}

#[test]
fn subtract_compute_returns_never_when_impossible() {
    let r = subtract::compute(
        prelude::TYPE_INT,
        prelude::TYPE_INT,
        &empty_world(),
        LatticeOptions::default(),
        &mut LatticeReport::new(),
    );
    assert_eq!(r, prelude::TYPE_NEVER);
}

#[test]
fn subtract_compute_unwraps_redundant() {
    let r = subtract::compute(
        prelude::TYPE_INT,
        prelude::TYPE_STRING,
        &empty_world(),
        LatticeOptions::default(),
        &mut LatticeReport::new(),
    );
    assert_eq!(r, prelude::TYPE_INT);
}

#[test]
fn meet_narrow_unrelated_objects_are_impossible() {
    // Foo ∧ Bar with no shared ancestry reports Impossible: PHP's
    // single-inheritance class graph cannot host a value that is both,
    // so the meet is `never`. Lifting this requires a world surface
    // for shared interfaces / traits.
    let mut w = MockWorld::new();
    w.declare("Foo");
    w.declare("Bar");
    let foo = u(t_named("Foo"));
    let bar = u(t_named("Bar"));
    let mut report = LatticeReport::new();
    let r = meet::narrow(foo, bar, &w, LatticeOptions::default(), &mut report);
    match r {
        MeetOutcome::Impossible => {}
        other => panic!("expected Impossible, got {other:?}"),
    }
}

#[test]
fn meet_outcome_into_type_round_trip() {
    let lit = u(t_lit_int(42));
    assert_eq!(MeetOutcome::Redundant(lit).into_type(), lit);
    assert_eq!(MeetOutcome::Narrowed(prelude::TYPE_INT).into_type(), prelude::TYPE_INT);
    assert_eq!(MeetOutcome::Impossible.into_type(), prelude::TYPE_NEVER);
}

#[test]
fn subtract_outcome_into_type_round_trip() {
    let lit = u(t_lit_int(42));
    assert_eq!(SubtractOutcome::Redundant(lit).into_type(), lit);
    assert_eq!(SubtractOutcome::Narrowed(prelude::TYPE_INT).into_type(), prelude::TYPE_INT);
    assert_eq!(SubtractOutcome::Impossible.into_type(), prelude::TYPE_NEVER);
}
