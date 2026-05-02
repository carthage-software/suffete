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

#[test]
fn mixed_to_int_sets_coerced_and_from_nested() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(mixed(), t_int(), &cb);
    assert!(!v);
    assert!(r.coerced());
    assert!(r.causes.true_union_narrow());
    assert!(r.causes.nested_mixed());
}

#[test]
fn mixed_to_string_sets_coerced_and_from_nested() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(mixed(), t_string(), &cb);
    assert!(!v);
    assert!(r.coerced());
    assert!(r.causes.true_union_narrow());
    assert!(r.causes.nested_mixed());
}

#[test]
fn array_key_to_int_sets_coerced() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_array_key(), t_int(), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
    assert!(!r.causes.nested_mixed());
}

#[test]
fn array_key_to_string_sets_coerced() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_array_key(), t_string(), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
}

#[test]
fn object_to_named_sets_coerced_and_object_any_down() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_object_any(), t_named("Foo"), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
    assert!(r.causes.object_any_down());
}

#[test]
fn bool_to_true_sets_coerced() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_bool(), t_true(), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
}

#[test]
fn bool_to_false_sets_coerced() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_bool(), t_false(), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
}

#[test]
fn lit_int_in_int_no_flags_set() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_lit_int(5), t_int(), &cb);
    assert!(v);
    assert!(!r.coerced());
    assert_eq!(r.replacement, None);
}

#[test]
fn int_to_lit_int_no_coerced_flag() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_int(), t_lit_int(5), &cb);
    assert!(!v);
    assert!(!r.coerced());
}

#[test]
fn int_to_positive_int_no_coerced_flag() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_int(), t_positive_int(), &cb);
    assert!(!v);
    assert!(!r.coerced());
}

#[test]
fn equal_atoms_no_flags() {
    let cb = empty_world();
    for atom in [t_int(), t_string(), t_bool(), t_float(), null(), t_object_any(), mixed()] {
        let (v, r) = atomic_is_contained_capturing(atom, atom, &cb);
        assert!(v, "{atom:?} should equal itself");
        assert!(!r.coerced(), "no causes for {atom:?} == itself");
        assert_eq!(r.replacement, None);
    }
}

#[test]
fn never_to_anything_no_flags() {
    let cb = empty_world();
    for atom in [t_int(), t_string(), t_object_any(), mixed()] {
        let (v, r) = atomic_is_contained_capturing(never(), atom, &cb);
        assert!(v, "never <: {atom:?}");
        assert!(!r.coerced());
    }
}

#[test]
fn mixed_to_never_sets_coerced() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(mixed(), never(), &cb);
    assert!(!v);
    assert!(r.causes.true_union_narrow());
    assert!(r.causes.nested_mixed());
}

#[test]
fn concrete_to_never_does_not_set_coerced() {
    let cb = empty_world();
    for atom in [t_int(), t_string()] {
        let (v, r) = atomic_is_contained_capturing(atom, never(), &cb);
        assert!(!v);
        assert!(!r.coerced());
    }
}

#[test]
fn literal_int_to_other_literal_no_coerced_flag() {
    let cb = empty_world();
    let (v, r) = atomic_is_contained_capturing(t_lit_int(1), t_lit_int(2), &cb);
    assert!(!v);
    assert!(!r.coerced());
}

#[test]
fn int_does_not_refine_float_under_strict_subtype() {
    let cb = empty_world();
    let (v, _r) = atomic_is_contained_capturing(t_int(), t_float(), &cb);
    assert!(!v);
}

#[test]
fn template_constrained_to_mixed_sets_from_as_mixed_on_rejection() {
    let cb = empty_world();
    let template = t_template("Foo", "T");
    let (v, r) = atomic_is_contained_capturing(template, t_int(), &cb);
    assert!(!v);
    assert!(r.causes.from_as_mixed());
    assert!(r.causes.true_union_narrow());
    assert!(!r.causes.nested_mixed());
}

#[test]
fn template_to_mixed_does_not_set_from_as_mixed() {
    let cb = empty_world();
    let template = t_template("Foo", "T");
    let (v, r) = atomic_is_contained_capturing(template, mixed(), &cb);
    assert!(v);
    assert!(!r.causes.from_as_mixed());
}

#[test]
fn fresh_report_has_empty_bounds_and_no_replacements() {
    let r = suffete::lattice::LatticeReport::new();
    assert!(r.replacement.is_none());
    assert!(r.replacement_element.is_none());
    assert!(r.bounds.is_empty());
}

#[test]
fn nullable_int_to_int_with_ignore_null_passes() {
    let cb = empty_world();
    let nullable = u_many(vec![t_int(), null()]);
    let int_only = u(t_int());
    assert!(is_contained_with(nullable, int_only, &cb, true, false, false));
}

#[test]
fn int_or_false_to_int_with_ignore_false_passes() {
    let cb = empty_world();
    let int_or_false = u_many(vec![t_int(), t_false()]);
    let int_only = u(t_int());
    assert!(is_contained_with(int_or_false, int_only, &cb, false, true, false));
}

#[test]
fn nullable_int_to_int_without_ignore_null_fails() {
    let cb = empty_world();
    let nullable = u_many(vec![t_int(), null()]);
    let int_only = u(t_int());
    assert!(!is_contained_with(nullable, int_only, &cb, false, false, false));
}
