//! Directed precision tests for `meet`. Each case is a concrete pair
//! `(a, b, expected_meet)` where the expected result is what
//! type-theoretic intersection demands. Cases that currently fail
//! mark imprecision spots in suffete's family-meet rules.

mod comparator_common;

use comparator_common::*;
use suffete::TypeId;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::meet;

fn meet_eq(a: TypeId, b: TypeId, expected: TypeId) {
    let w = empty_world();
    let mut report = LatticeReport::new();
    let result = meet::compute(a, b, &w, LatticeOptions::default(), &mut report);
    assert_eq!(result, expected, "meet({a}, {b}) = {result}, expected {expected}",);
}

fn meet_eq_with<W: suffete::world::World>(a: TypeId, b: TypeId, expected: TypeId, world: &W) {
    let mut report = LatticeReport::new();
    let result = meet::compute(a, b, world, LatticeOptions::default(), &mut report);
    assert_eq!(result, expected, "meet({a}, {b}) = {result}, expected {expected}",);
}

#[test]
fn numeric_meet_string_is_numeric_string() {
    let lhs = u(t_numeric());
    let rhs = u(t_string());
    let expected = u(t_numeric_string());
    meet_eq(lhs, rhs, expected);
}

#[test]
fn lower_meet_upper_keeps_only_empty() {
    // `lowercase` requires no uppercase chars; `uppercase` requires no
    // lowercase chars. The only string satisfying both is "".
    let lhs = u(t_lower_string());
    let rhs = u(t_upper_string());
    let expected = u(t_lit_string(""));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn lower_meet_non_empty_is_lower_non_empty() {
    let lhs = u(t_lower_string());
    let rhs = u(t_non_empty_string());
    let expected = suffete::TypeId::singleton(suffete::prelude::NON_EMPTY_LOWERCASE_STRING);
    meet_eq(lhs, rhs, expected);
}

#[test]
fn upper_meet_non_empty_is_upper_non_empty() {
    let lhs = u(t_upper_string());
    let rhs = u(t_non_empty_string());
    let expected = suffete::TypeId::singleton(suffete::prelude::NON_EMPTY_UPPERCASE_STRING);
    meet_eq(lhs, rhs, expected);
}

#[test]
fn truthy_meet_numeric_is_truthy_numeric() {
    let lhs = u(t_truthy_string());
    let rhs = u(t_numeric_string());
    let expected = suffete::TypeId::singleton(suffete::prelude::TRUTHY_NUMERIC_STRING);
    meet_eq(lhs, rhs, expected);
}

#[test]
fn array_key_meet_int_is_int() {
    let lhs = u(t_array_key());
    let rhs = u(t_int());
    let expected = u(t_int());
    meet_eq(lhs, rhs, expected);
}

#[test]
fn array_key_meet_string_is_string() {
    let lhs = u(t_array_key());
    let rhs = u(t_string());
    let expected = u(t_string());
    meet_eq(lhs, rhs, expected);
}

#[test]
fn scalar_meet_bool_is_bool() {
    let lhs = u(t_scalar());
    let rhs = u(t_bool());
    let expected = u(t_bool());
    meet_eq(lhs, rhs, expected);
}

#[test]
fn open_resource_meet_closed_resource_is_never() {
    let lhs = u(t_open_resource());
    let rhs = u(t_closed_resource());
    let expected = suffete::prelude::TYPE_NEVER;
    meet_eq(lhs, rhs, expected);
}

#[test]
fn class_string_unrelated_meet_is_never() {
    let lhs = u(t_lit_class_string("Foo"));
    let rhs = u(t_lit_class_string("Bar"));
    let expected = suffete::prelude::TYPE_NEVER;
    meet_eq(lhs, rhs, expected);
}

#[test]
fn class_string_descendant_meet_is_descendant() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo");
    let parent = u(t_class_string_of(u(t_named("Foo"))));
    let child = u(t_class_string_of(u(t_named("Bar"))));
    meet_eq_with(parent, child, child, &w);
}

#[test]
fn list_int_meet_list_string_is_list_never() {
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_STRING;
    let lhs = u(t_list(TYPE_INT, false));
    let rhs = u(t_list(TYPE_STRING, false));
    let expected = u(t_list(suffete::prelude::TYPE_NEVER, false));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn keyed_array_disjoint_keys_meet_is_combined_shape() {
    use std::collections::BTreeMap;
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_STRING;
    let lhs = u(t_keyed_sealed(BTreeMap::from([(ak_str("a"), (false, TYPE_INT))]), false));
    let rhs = u(t_keyed_sealed(BTreeMap::from([(ak_str("b"), (false, TYPE_STRING))]), false));
    let expected = u(t_keyed_sealed(
        BTreeMap::from([(ak_str("a"), (false, TYPE_INT)), (ak_str("b"), (false, TYPE_STRING))]),
        false,
    ));
    meet_eq(lhs, rhs, expected);
}
