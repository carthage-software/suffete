//! Overlap (`overlaps`) tests covering the universal axioms,
//! subsumption shortcut, integer-range overlap, and family-wedge cases
//! from comparison.md §2.

mod comparator_common;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::element::payload::DefiningEntity;
use suffete::prelude;

fn t_param_with_constraint(class: &str, name: &str, constraint: suffete::TypeId) -> ElementId {
    ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), constraint)
}

#[test]
fn reflexive_overlap() {
    let cb = empty_world();
    assert!(atomic_overlaps(t_int(), t_int(), &cb));
    assert!(atomic_overlaps(t_string(), t_string(), &cb));
    assert!(atomic_overlaps(t_lit_int(0), t_lit_int(0), &cb));
}

#[test]
fn never_disjoint_with_anything() {
    let cb = empty_world();
    assert!(!atomic_overlaps(never(), t_int(), &cb));
    assert!(!atomic_overlaps(never(), mixed(), &cb));
    assert!(!atomic_overlaps(never(), never(), &cb));
}

#[test]
fn mixed_overlaps_anything() {
    let cb = empty_world();
    assert!(atomic_overlaps(mixed(), t_int(), &cb));
    assert!(atomic_overlaps(mixed(), null(), &cb));
    assert!(atomic_overlaps(mixed(), t_named("Foo"), &cb));
}

#[test]
fn placeholder_overlaps_anything() {
    let cb = empty_world();
    assert!(atomic_overlaps(placeholder(), t_int(), &cb));
    assert!(atomic_overlaps(placeholder(), t_string(), &cb));
}

#[test]
fn subtype_implies_overlap() {
    let cb = empty_world();
    assert!(atomic_overlaps(t_lit_int(5), t_int(), &cb));
    assert!(atomic_overlaps(t_int(), t_lit_int(5), &cb));
    assert!(atomic_overlaps(t_true(), t_bool(), &cb));
    assert!(atomic_overlaps(t_int(), t_array_key(), &cb));
}

#[test]
fn distinct_kinds_disjoint() {
    let cb = empty_world();
    assert!(!atomic_overlaps(t_int(), t_string(), &cb));
    assert!(!atomic_overlaps(t_int(), null(), &cb));
    assert!(!atomic_overlaps(null(), t_string(), &cb));
    assert!(!atomic_overlaps(t_int(), t_resource(), &cb));
    assert!(!atomic_overlaps(t_string(), t_named("Foo"), &cb));
}

#[test]
fn int_and_float_are_disjoint() {
    // Strict value-set semantics: an `int` runtime value is not a
    // member of `float`. Flow-position coercion is modeled by a
    // separate predicate, not by `overlaps`.
    let cb = empty_world();
    assert!(!atomic_overlaps(t_int(), t_float(), &cb));
}

#[test]
fn int_ranges_overlap_when_intervals_intersect() {
    let cb = empty_world();
    let a = t_int_range(0, 10);
    let b = t_int_range(5, 15);
    assert!(atomic_overlaps(a, b, &cb));
    assert!(atomic_overlaps(b, a, &cb));
}

#[test]
fn int_ranges_disjoint_when_intervals_separate() {
    let cb = empty_world();
    let a = t_int_range(0, 10);
    let b = t_int_range(20, 30);
    assert!(!atomic_overlaps(a, b, &cb));
    assert!(!atomic_overlaps(b, a, &cb));
}

#[test]
fn touching_int_ranges_overlap_at_endpoint() {
    let cb = empty_world();
    let a = t_int_range(0, 10);
    let b = t_int_range(10, 20);
    assert!(atomic_overlaps(a, b, &cb));
}

#[test]
fn open_lower_int_overlaps_open_upper_int() {
    let cb = empty_world();
    let a = t_int_to(5);
    let b = t_int_from(0);
    assert!(atomic_overlaps(a, b, &cb));
}

#[test]
fn positive_int_disjoint_with_negative_int() {
    let cb = empty_world();
    assert!(!atomic_overlaps(t_positive_int(), t_negative_int(), &cb));
}

#[test]
fn lit_int_overlaps_range_when_in_bounds() {
    let cb = empty_world();
    let r = t_int_range(0, 10);
    assert!(atomic_overlaps(t_lit_int(5), r, &cb));
    assert!(atomic_overlaps(r, t_lit_int(5), &cb));
}

#[test]
fn lit_int_disjoint_with_range_when_out_of_bounds() {
    let cb = empty_world();
    let r = t_int_range(0, 10);
    assert!(!atomic_overlaps(t_lit_int(20), r, &cb));
}

#[test]
fn distinct_int_literals_disjoint() {
    let cb = empty_world();
    assert!(!atomic_overlaps(t_lit_int(1), t_lit_int(2), &cb));
}

#[test]
fn class_like_string_overlaps_string() {
    let cb = empty_world();
    assert!(atomic_overlaps(t_class_string(), t_string(), &cb));
    assert!(atomic_overlaps(t_string(), t_interface_string(), &cb));
}

#[test]
fn generic_parameter_overlaps_via_constraint() {
    let cb = empty_world();
    let t = t_param_with_constraint("Box", "T", u(t_int()));
    assert!(atomic_overlaps(t, t_int(), &cb));
    assert!(atomic_overlaps(t_int(), t, &cb));
    assert!(atomic_overlaps(t, t_lit_int(5), &cb));
    assert!(!atomic_overlaps(t, t_string(), &cb));
}

#[test]
fn unbounded_generic_parameter_overlaps_anything() {
    let cb = empty_world();
    let t = t_template("Box", "T");
    assert!(atomic_overlaps(t, t_int(), &cb));
    assert!(atomic_overlaps(t, t_string(), &cb));
    assert!(atomic_overlaps(t, null(), &cb));
}

#[test]
fn union_overlap_distributes() {
    let cb = empty_world();
    let int_or_string = u_many(vec![t_int(), t_string()]);
    let string_or_float = u_many(vec![t_string(), t_float()]);
    assert!(overlaps(int_or_string, string_or_float, &cb));
}

#[test]
fn union_disjoint_when_no_pair_overlaps() {
    let cb = empty_world();
    let int_or_null = u_many(vec![t_int(), null()]);
    let string_or_resource = u_many(vec![t_string(), t_resource()]);
    assert!(!overlaps(int_or_null, string_or_resource, &cb));
}

#[test]
fn nullable_overlap_via_null_branch() {
    let cb = empty_world();
    let nullable_int = u_many(vec![null(), t_int()]);
    let nullable_string = u_many(vec![null(), t_string()]);
    assert!(overlaps(nullable_int, nullable_string, &cb));
}

#[test]
fn truthy_mixed_overlaps_int() {
    let cb = empty_world();
    assert!(atomic_overlaps(mixed_truthy(), t_int(), &cb));
}

#[test]
fn nonnull_mixed_disjoint_with_null() {
    let cb = empty_world();
    assert!(!atomic_overlaps(mixed_nonnull(), null(), &cb));
    assert!(!atomic_overlaps(null(), mixed_nonnull(), &cb));
}

#[test]
fn truthy_mixed_disjoint_with_null() {
    let cb = empty_world();
    assert!(!atomic_overlaps(mixed_truthy(), null(), &cb));
    assert!(!atomic_overlaps(mixed_truthy(), t_false(), &cb));
    assert!(!atomic_overlaps(mixed_truthy(), t_lit_int(0), &cb));
    assert!(!atomic_overlaps(mixed_truthy(), t_lit_string(""), &cb));
}

#[test]
fn truthy_mixed_overlaps_truthy_inputs() {
    let cb = empty_world();
    assert!(atomic_overlaps(mixed_truthy(), t_true(), &cb));
    assert!(atomic_overlaps(mixed_truthy(), t_lit_int(42), &cb));
    assert!(atomic_overlaps(mixed_truthy(), t_named("Foo"), &cb));
    assert!(atomic_overlaps(mixed_truthy(), t_resource(), &cb));
}

#[test]
fn falsy_mixed_disjoint_with_truthy_inputs() {
    let cb = empty_world();
    assert!(!atomic_overlaps(mixed_falsy(), t_true(), &cb));
    assert!(!atomic_overlaps(mixed_falsy(), t_named("Foo"), &cb));
    assert!(!atomic_overlaps(mixed_falsy(), t_resource(), &cb));
    assert!(!atomic_overlaps(mixed_falsy(), t_lit_int(42), &cb));
}

#[test]
fn falsy_mixed_overlaps_falsy_inputs() {
    let cb = empty_world();
    assert!(atomic_overlaps(mixed_falsy(), null(), &cb));
    assert!(atomic_overlaps(mixed_falsy(), t_false(), &cb));
    assert!(atomic_overlaps(mixed_falsy(), t_lit_int(0), &cb));
    assert!(atomic_overlaps(mixed_falsy(), t_lit_string(""), &cb));
}

#[test]
fn truthy_mixed_disjoint_with_falsy_mixed() {
    let cb = empty_world();
    assert!(!atomic_overlaps(mixed_truthy(), mixed_falsy(), &cb));
}

#[test]
fn truthy_mixed_overlaps_undetermined_inputs() {
    let cb = empty_world();
    // `int` (general) admits both truthy and falsy values; overlap holds.
    assert!(atomic_overlaps(mixed_truthy(), t_int(), &cb));
    assert!(atomic_overlaps(mixed_truthy(), t_string(), &cb));
    assert!(atomic_overlaps(mixed_truthy(), t_bool(), &cb));
}

#[test]
fn class_string_literal_overlaps_lit_class_string() {
    let cb = empty_world();
    assert!(atomic_overlaps(t_lit_class_string("Foo"), t_class_string(), &cb));
    assert!(atomic_overlaps(t_class_string(), t_lit_class_string("Foo"), &cb));
}

#[test]
fn never_type_disjoint_at_type_level() {
    let cb = empty_world();
    assert!(!overlaps(prelude::TYPE_NEVER, prelude::TYPE_INT, &cb));
}
