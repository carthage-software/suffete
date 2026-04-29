//! Directed precision tests for `meet`. Each case is a concrete pair
//! `(a, b, expected_meet)` where the expected result is what
//! type-theoretic intersection demands. Cases that currently fail
//! mark imprecision spots in suffete's family-meet rules.

mod comparator_common;

use comparator_common::*;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
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

// --- iterable ---------------------------------------------------------

#[test]
fn iterable_int_int_meet_iterable_int_string_is_iterable_int_never() {
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_NEVER;
    use suffete::prelude::TYPE_STRING;
    let lhs = u(t_iterable(TYPE_INT, TYPE_INT));
    let rhs = u(t_iterable(TYPE_INT, TYPE_STRING));
    let expected = u(t_iterable(TYPE_INT, TYPE_NEVER));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn iterable_int_a_meet_iterable_int_b_keys_intersect() {
    use suffete::prelude::TYPE_ARRAY_KEY;
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_MIXED;
    let lhs = u(t_iterable(TYPE_ARRAY_KEY, TYPE_MIXED));
    let rhs = u(t_iterable(TYPE_INT, TYPE_MIXED));
    let expected = u(t_iterable(TYPE_INT, TYPE_MIXED));
    meet_eq(lhs, rhs, expected);
}

// --- callable ---------------------------------------------------------

#[test]
fn callable_meet_with_compatible_signatures_intersects_return_unions_params() {
    // Return type is covariant, so meet on return.
    // Params are contravariant, so meet means "accept the union" (join on params).
    use suffete::prelude::TYPE_INT;
    use suffete::prelude::TYPE_STRING;
    let lhs = u(t_callable(&[TYPE_INT], TYPE_INT));
    let rhs = u(t_callable(&[TYPE_INT], TYPE_STRING));
    let expected = u(t_callable(&[TYPE_INT], suffete::prelude::TYPE_NEVER));
    meet_eq(lhs, rhs, expected);
}

// --- class-like-string structural -------------------------------------

#[test]
fn class_string_unrelated_constraints_meet_is_never() {
    let w = MockWorld::new();
    let lhs = u(t_class_string_of(u(t_named("Foo"))));
    let rhs = u(t_class_string_of(u(t_named("Bar"))));
    meet_eq_with(lhs, rhs, suffete::prelude::TYPE_NEVER, &w);
}

#[test]
fn class_string_kinds_disjoint_meet_is_never() {
    let w = MockWorld::new();
    let class = u(t_class_string_of(u(t_named("Foo"))));
    let interface = u(t_interface_string_of(u(t_named("Foo"))));
    meet_eq_with(class, interface, suffete::prelude::TYPE_NEVER, &w);
}

// --- enum + enum-case ------------------------------------------------

#[test]
fn enum_meet_enum_case_is_case() {
    let w = MockWorld::new();
    let any = u(t_enum("E"));
    let case = u(t_enum_case("E", "A"));
    meet_eq_with(any, case, case, &w);
}

#[test]
fn distinct_enum_cases_meet_is_never() {
    let w = MockWorld::new();
    let a = u(t_enum_case("E", "A"));
    let b = u(t_enum_case("E", "B"));
    meet_eq_with(a, b, suffete::prelude::TYPE_NEVER, &w);
}

#[test]
fn distinct_enums_meet_is_never() {
    let w = MockWorld::new();
    let e = u(t_enum("E"));
    let f = u(t_enum("F"));
    meet_eq_with(e, f, suffete::prelude::TYPE_NEVER, &w);
}

// --- has-method / has-property composition ----------------------------

#[test]
fn has_method_meet_has_method_composes() {
    // `(object with foo) ∧ (object with bar) → object with foo & bar`.
    let lhs = u(t_has_method("foo"));
    let rhs = u(t_has_method("bar"));
    let result = {
        let w = empty_world();
        let mut report = LatticeReport::new();
        meet::compute(lhs, rhs, &w, LatticeOptions::default(), &mut report)
    };
    assert_ne!(result, suffete::prelude::TYPE_NEVER, "has-method ∧ has-method should compose, got NEVER");
}

#[test]
fn named_object_with_method_meet_has_method_passes_when_world_confirms() {
    let mut w = MockWorld::new();
    w.with_method("Foo", "doFoo");
    let named = u(t_named("Foo"));
    let constraint = u(t_has_method("doFoo"));
    let mut report = LatticeReport::new();
    let result = meet::compute(named, constraint, &w, LatticeOptions::default(), &mut report);
    assert_eq!(result, named, "Named(Foo) ∧ has_method(doFoo) should reduce to Named(Foo)");
}

// --- list / keyed-array crossing -------------------------------------

#[test]
fn empty_array_meet_list_int_is_empty_array() {
    use suffete::prelude::TYPE_INT;
    let lhs = u(t_empty_array());
    let rhs = u(t_list(TYPE_INT, false));
    meet_eq(lhs, rhs, lhs);
}

#[test]
fn list_int_meet_keyed_int_int_is_list_int() {
    use suffete::prelude::TYPE_INT;
    let lhs = u(t_list(TYPE_INT, false));
    let rhs = u(t_keyed_unsealed(TYPE_INT, TYPE_INT, false));
    meet_eq(lhs, rhs, lhs);
}

// --- mixed variants --------------------------------------------------

#[test]
fn truthy_mixed_meet_falsy_mixed_is_never() {
    let lhs = u(mixed_truthy());
    let rhs = u(mixed_falsy());
    meet_eq(lhs, rhs, suffete::prelude::TYPE_NEVER);
}

#[test]
fn nonnull_mixed_meet_null_is_never() {
    let lhs = u(mixed_nonnull());
    let rhs = u(null());
    meet_eq(lhs, rhs, suffete::prelude::TYPE_NEVER);
}

#[test]
#[ignore = "needs a `non-zero int` representation (positive | negative int union)"]
fn truthy_mixed_meet_int_is_truthy_int_set() {
    // truthy_mixed ∧ int admits all non-zero ints. We accept any
    // result that's a subtype of both inputs and non-empty.
    let w = empty_world();
    let lhs = u(mixed_truthy());
    let rhs = u(t_int());
    let mut report = LatticeReport::new();
    let result = meet::compute(lhs, rhs, &w, LatticeOptions::default(), &mut report);
    assert_ne!(result, suffete::prelude::TYPE_NEVER, "truthy_mixed ∧ int should be non-empty");
}

#[test]
fn template_with_int_or_string_meet_int_narrows_constraint_to_int() {
    let int_or_string = interner().intern_type(&[t_int(), t_string()], FlowFlags::EMPTY);
    let lhs = u(t_template_of("C", "T", int_or_string));
    let rhs = u(t_int());
    let expected = u(t_template_of("C", "T", u(t_int())));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn template_with_int_or_string_meet_string_narrows_constraint_to_string() {
    let int_or_string = interner().intern_type(&[t_int(), t_string()], FlowFlags::EMPTY);
    let lhs = u(t_template_of("C", "T", int_or_string));
    let rhs = u(t_string());
    let expected = u(t_template_of("C", "T", u(t_string())));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn template_with_int_meet_string_is_impossible() {
    let lhs = u(t_template_of("C", "T", u(t_int())));
    let rhs = u(t_string());
    meet_eq(lhs, rhs, suffete::prelude::TYPE_NEVER);
}

#[test]
fn template_with_int_meet_int_is_redundant_keeps_template() {
    let lhs = u(t_template_of("C", "T", u(t_int())));
    let rhs = u(t_int());
    meet_eq(lhs, rhs, lhs);
}

#[test]
fn same_template_meet_with_overlapping_constraints_intersects_them() {
    let int_or_string = interner().intern_type(&[t_int(), t_string()], FlowFlags::EMPTY);
    let int_or_float = interner().intern_type(&[t_int(), t_float()], FlowFlags::EMPTY);
    let lhs = u(t_template_of("C", "T", int_or_string));
    let rhs = u(t_template_of("C", "T", int_or_float));
    let expected = u(t_template_of("C", "T", u(t_int())));
    meet_eq(lhs, rhs, expected);
}

#[test]
fn distinct_templates_have_no_meet_rule_and_collapse_to_never() {
    let lhs = u(t_template_of("C", "T", u(t_int())));
    let rhs = u(t_template_of("C", "U", u(t_int())));
    meet_eq(lhs, rhs, suffete::prelude::TYPE_NEVER);
}

#[test]
fn contravariant_a_object_meet_a_int_under_contravariant_t_subsumes_to_more_specific() {
    use suffete::lattice::refines;
    use suffete::world::Variance;
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Contravariant)]);
    let a_object = u(t_generic_named("A", vec![u(t_named("Object"))]));
    let a_int = u(t_generic_named("A", vec![u(t_int())]));

    let mut report = LatticeReport::new();
    let m = meet::compute(a_object, a_int, &w, LatticeOptions::default(), &mut report);
    let r1 = refines(m, a_object, &w, LatticeOptions::default(), &mut report);
    let r2 = refines(m, a_int, &w, LatticeOptions::default(), &mut report);
    assert!(r1, "meet={m} should refine {a_object}");
    assert!(r2, "meet={m} should refine {a_int}");
}

#[test]
fn invariant_a_associativity_arb_failing_case() {
    use suffete::lattice::refines;
    use suffete::world::Variance;
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Invariant)]);
    w.with_templates("B", &[("T", Variance::Contravariant)]);
    w.with_templates("C", &[("T", Variance::Contravariant)]);
    w.with_templates("D", &[("T", Variance::Invariant)]);
    w.with_extended("B", "C", vec![suffete::prelude::TYPE_MIXED]);
    w.with_extended("A", "B", vec![suffete::prelude::TYPE_MIXED]);

    let object = u(suffete::prelude::OBJECT);
    let a_object = u(t_generic_named("A", vec![object]));
    let a_int = u(t_generic_named("A", vec![u(t_int())]));
    let a_bare = u(t_named("A"));

    let a_t = interner().intern_type(&[a_object.as_ref().elements[0], t_int()], FlowFlags::EMPTY);
    let b_t = interner().intern_type(&[a_bare.as_ref().elements[0], t_int()], FlowFlags::EMPTY);
    let c_t = a_int;

    let mut report = LatticeReport::new();
    let l = meet::compute(meet::compute(a_t, b_t, &w, LatticeOptions::default(), &mut report), c_t, &w, LatticeOptions::default(), &mut report);
    let r = meet::compute(a_t, meet::compute(b_t, c_t, &w, LatticeOptions::default(), &mut report), &w, LatticeOptions::default(), &mut report);

    eprintln!("a={a_t}, b={b_t}, c={c_t}");
    eprintln!("(a∩b)∩c = {l}");
    eprintln!("a∩(b∩c) = {r}");
    let l_refines_c = refines(l, c_t, &w, LatticeOptions::default(), &mut report);
    let r_refines_c = refines(r, c_t, &w, LatticeOptions::default(), &mut report);
    eprintln!("l refines c = {l_refines_c}");
    eprintln!("r refines c = {r_refines_c}");
    assert!(l_refines_c, "(a∩b)∩c should refine c");
    assert!(r_refines_c, "a∩(b∩c) should refine c");
}
