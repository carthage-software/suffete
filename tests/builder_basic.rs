//! TypeBuilder tests: constructors, mutations, flow-flag toggles,
//! origin short-circuit, and the canonical "build collapses to TYPE_NEVER
//! on empty buffer" contract.

mod comparator_common;

use comparator_common::*;

use suffete::FlowFlags;
use suffete::TypeBuilder;
use suffete::prelude;

#[test]
fn new_is_empty_with_empty_flags() {
    let b = TypeBuilder::new();
    assert!(b.is_empty());
    assert_eq!(b.flags(), FlowFlags::EMPTY);
    assert_eq!(b.build(), prelude::TYPE_NEVER);
}

#[test]
fn default_matches_new() {
    let a = TypeBuilder::new().build();
    let b = TypeBuilder::default().build();
    assert_eq!(a, b);
}

#[test]
fn from_type_round_trips_unchanged_via_short_circuit() {
    let ty = u_many(vec![t_int(), t_string()]);
    let b = TypeBuilder::from_type(ty);
    assert_eq!(b.elements().len(), 2);
    assert_eq!(b.build(), ty);
}

#[test]
fn from_via_into_round_trips() {
    let ty = prelude::TYPE_INT;
    let b: TypeBuilder = ty.into();
    assert_eq!(b.build(), ty);
}

#[test]
fn push_then_build_canonicalises_to_union() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).push(t_string());
    let result = b.build();
    assert_eq!(result, prelude::TYPE_INT_OR_STRING);
}

#[test]
fn push_order_does_not_affect_build_result() {
    let mut a = TypeBuilder::new();
    a.push(t_int()).push(t_string());
    let mut b = TypeBuilder::new();
    b.push(t_string()).push(t_int());
    assert_eq!(a.build(), b.build());
}

#[test]
fn extend_appends_many_elements() {
    let mut b = TypeBuilder::new();
    b.extend(vec![t_int(), t_string(), t_float()]);
    let result = b.build();
    let expected = u_many(vec![t_int(), t_string(), t_float()]);
    assert_eq!(result, expected);
}

#[test]
fn remove_drops_first_match() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).push(t_string()).push(t_int());
    b.remove(t_int());
    assert_eq!(b.build(), prelude::TYPE_INT_OR_STRING);
}

#[test]
fn remove_absent_is_noop_and_preserves_short_circuit() {
    let ty = prelude::TYPE_INT;
    let mut b = TypeBuilder::from_type(ty);
    b.remove(t_string());
    assert_eq!(b.build(), ty);
}

#[test]
fn remove_all_drops_every_occurrence() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).push(t_string()).push(t_int());
    b.remove_all(t_int());
    assert_eq!(b.build(), prelude::TYPE_STRING);
}

#[test]
fn retain_keeps_matching_predicate() {
    let mut b = TypeBuilder::from_type(u_many(vec![t_int(), t_string(), t_float()]));
    b.retain(|e| *e != t_string());
    let result = b.build();
    let expected = u_many(vec![t_int(), t_float()]);
    assert_eq!(result, expected);
}

#[test]
fn replace_swaps_first_match() {
    let mut b = TypeBuilder::from_type(u_many(vec![t_int(), null()]));
    b.replace(null(), t_string());
    assert_eq!(b.build(), prelude::TYPE_INT_OR_STRING);
}

#[test]
fn replace_absent_is_noop_and_preserves_short_circuit() {
    let ty = prelude::TYPE_INT;
    let mut b = TypeBuilder::from_type(ty);
    b.replace(null(), t_string());
    assert_eq!(b.build(), ty);
}

#[test]
fn map_replaces_in_place() {
    let mut b = TypeBuilder::from_type(u(t_lit_int(1)));
    b.map(|e| if e == t_lit_int(1) { t_int() } else { e });
    assert_eq!(b.build(), suffete::TypeId::singleton(t_int()));
}

#[test]
fn flat_map_one_to_many_explodes() {
    let mut b = TypeBuilder::from_type(u(t_lit_int(5)));
    b.flat_map(|e| if e == t_lit_int(5) { vec![t_int_range(0, 4), t_int_range(6, 10)] } else { vec![e] });
    let result = b.build();
    let expected = u_many(vec![t_int_range(0, 4), t_int_range(6, 10)]);
    assert_eq!(result, expected);
}

#[test]
fn flat_map_one_to_zero_drops_element() {
    let mut b = TypeBuilder::from_type(u_many(vec![t_int(), t_string()]));
    b.flat_map(|e| if e == t_string() { Vec::new() } else { vec![e] });
    assert_eq!(b.build(), prelude::TYPE_INT);
}

#[test]
fn set_flags_replaces_flag_set() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).set_flags(FlowFlags::EMPTY);
    let result = b.build();
    assert_eq!(result.flags(), FlowFlags::EMPTY);
}

#[test]
fn modify_flags_lets_caller_mutate_flag_set() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).modify_flags(|_| FlowFlags::EMPTY);
    let result = b.build();
    assert_eq!(result.flags(), FlowFlags::EMPTY);
}

#[test]
fn unmodified_from_type_short_circuits_to_origin_handle() {
    let ty = u_many(vec![t_int(), t_string()]);
    let b = TypeBuilder::from_type(ty);
    let built = b.build();
    assert_eq!(built, ty);
}

#[test]
fn mutated_then_reverted_buffer_still_rebuilds_via_no_diff_check() {
    // The dirty flag stays set after push+remove; the short-circuit
    // does not detect "shape returned to origin", so we go through join
    // and intern again. Documented as intentional — diff tracking
    // would defeat the build-then-finalise point.
    let ty = prelude::TYPE_INT;
    let mut b = TypeBuilder::from_type(ty);
    b.push(t_string()).remove(t_string());
    assert_eq!(b.build(), ty);
}

#[test]
fn chain_of_mutations_yields_canonical_union() {
    let mut b = TypeBuilder::from_type(prelude::TYPE_INT);
    b.push(t_string()).push(null()).remove(null()).set_flags(FlowFlags::EMPTY);
    assert_eq!(b.build(), prelude::TYPE_INT_OR_STRING);
}

#[test]
fn contains_reports_buffer_membership() {
    let b = TypeBuilder::from_type(u_many(vec![t_int(), t_string()]));
    assert!(b.contains(t_int()));
    assert!(b.contains(t_string()));
    assert!(!b.contains(null()));
}

#[test]
fn len_reports_pre_canonicalisation_count() {
    let mut b = TypeBuilder::new();
    b.push(t_int()).push(t_int()).push(t_string());
    assert_eq!(b.len(), 3);
    let built = b.build();
    assert_eq!(built.as_ref().elements.len(), 2);
}

#[test]
fn elements_returns_buffer_view_in_mutation_order() {
    let mut b = TypeBuilder::new();
    b.push(t_string()).push(t_int());
    assert_eq!(b.elements(), &[t_string(), t_int()]);
}

#[test]
fn empty_builder_after_remove_all_collapses_to_never() {
    let mut b = TypeBuilder::from_type(prelude::TYPE_INT);
    b.remove_all(t_int());
    assert_eq!(b.build(), prelude::TYPE_NEVER);
}

#[test]
fn flat_map_collapsing_all_to_empty_yields_never() {
    let mut b = TypeBuilder::from_type(u_many(vec![t_int(), t_string()]));
    b.flat_map(|_| Vec::new());
    assert_eq!(b.build(), prelude::TYPE_NEVER);
}

#[test]
fn build_canonicalises_full_int_decomposition_to_int() {
    let mut b = TypeBuilder::new();
    b.push(t_int_range(0, 1)).push(t_int_to(-1)).push(t_int_range(2, 500)).push(t_int_from(500));
    assert_eq!(b.build_canonical(), prelude::TYPE_INT);
}

#[test]
fn build_canonicalises_string_decomposition_to_string() {
    // `non_empty` + `lit("")` cover the full string space, so the
    // canonical form is plain `string`. `lit("hello")` is also subsumed.
    let mut b = TypeBuilder::new();
    b.push(t_lit_string("")).push(t_non_empty_string()).push(t_lit_string("hello"));
    assert_eq!(b.build_canonical(), prelude::TYPE_STRING);
}
