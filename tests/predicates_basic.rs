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

use suffete::predicates as p;
use suffete::prelude;

#[test]
fn is_never_true_only_for_type_never() {
    assert!(p::is_never(prelude::TYPE_NEVER));
    assert!(!p::is_never(prelude::TYPE_INT));
    assert!(!p::is_never(prelude::TYPE_MIXED));
}

#[test]
fn is_mixed_true_only_for_vanilla_mixed() {
    assert!(p::is_mixed(prelude::TYPE_MIXED));
    assert!(!p::is_mixed(prelude::TYPE_INT));
    assert!(!p::is_mixed(u(mixed_truthy())));
}

#[test]
fn is_singleton_and_is_union() {
    assert!(p::is_singleton(prelude::TYPE_INT));
    assert!(!p::is_union(prelude::TYPE_INT));
    let u2 = u_many(vec![t_int(), t_string()]);
    assert!(!p::is_singleton(u2));
    assert!(p::is_union(u2));
}

#[test]
fn is_int_true_for_int_family() {
    assert!(p::is_int(prelude::TYPE_INT));
    assert!(p::is_int(u(t_lit_int(42))));
    assert!(p::is_int(u(t_int_range(0, 10))));
    assert!(p::is_int(u(t_positive_int())));
}

#[test]
fn is_int_false_for_unions_with_other_kinds() {
    assert!(!p::is_int(u_many(vec![t_int(), t_string()])));
    assert!(!p::is_int(prelude::TYPE_STRING));
    assert!(!p::is_int(prelude::TYPE_NEVER));
}

#[test]
fn is_int_true_for_union_of_int_variants() {
    let u = u_many(vec![t_lit_int(0), t_int_range(1, 10)]);
    assert!(p::is_int(u));
}

#[test]
fn is_string_distinguishes_class_like_string() {
    assert!(p::is_string(prelude::TYPE_STRING));
    assert!(p::is_string(u(t_non_empty_string())));
    assert!(!p::is_string(u(t_class_string())));
}

#[test]
fn is_bool_includes_true_false() {
    assert!(p::is_bool(prelude::TYPE_BOOL));
    assert!(p::is_bool(prelude::TYPE_TRUE));
    assert!(p::is_bool(prelude::TYPE_FALSE));
    assert!(p::is_bool(u_many(vec![t_true(), t_false()])));
}

#[test]
fn is_object_covers_object_family() {
    assert!(p::is_object(prelude::TYPE_OBJECT));
    assert!(p::is_object(u(t_named("Foo"))));
    assert!(p::is_object(u(t_enum("E"))));
}

#[test]
fn is_array_covers_list_and_keyed() {
    assert!(p::is_array(u(t_empty_array())));
    assert!(p::is_array(u(t_list(prelude::TYPE_INT, false))));
    assert!(p::is_list(u(t_list(prelude::TYPE_INT, false))));
    assert!(!p::is_list(u(t_empty_array())));
}

#[test]
fn is_callable_strict() {
    assert!(p::is_callable(u(t_callable_mixed())));
    assert!(!p::is_callable(prelude::TYPE_STRING));
}

#[test]
fn is_scalar_dominator_and_members() {
    assert!(p::is_scalar(prelude::TYPE_SCALAR));
    assert!(p::is_scalar(prelude::TYPE_INT));
    assert!(p::is_scalar(prelude::TYPE_STRING));
    assert!(!p::is_scalar(u(t_named("Foo"))));
}

#[test]
fn is_numeric_includes_int_and_float() {
    assert!(p::is_numeric(prelude::TYPE_INT));
    assert!(p::is_numeric(prelude::TYPE_FLOAT));
    assert!(p::is_numeric(prelude::TYPE_NUMERIC));
    assert!(!p::is_numeric(prelude::TYPE_STRING));
}

#[test]
fn contains_string_in_union() {
    let u = u_many(vec![t_int(), t_string()]);
    assert!(p::contains_string(u));
    assert!(p::contains_int(u));
    assert!(!p::contains_float(u));
}

#[test]
fn contains_null_in_nullable() {
    let u = u_many(vec![null(), t_int()]);
    assert!(p::contains_null(u));
}

#[test]
fn contains_object_in_union() {
    let u = u_many(vec![null(), t_named("Foo")]);
    assert!(p::contains_object(u));
}

#[test]
fn contains_mixed_top_level_only() {
    assert!(p::contains_mixed(prelude::TYPE_MIXED));
    let nested = u(t_list(prelude::TYPE_MIXED, false));
    assert!(!p::contains_mixed(nested));
    assert!(p::contains_mixed_anywhere(nested));
}

#[test]
fn true_is_truthy_false_is_falsy() {
    assert!(p::is_truthy(prelude::TYPE_TRUE));
    assert!(!p::is_truthy(prelude::TYPE_FALSE));
    assert!(p::is_falsy(prelude::TYPE_FALSE));
    assert!(!p::is_falsy(prelude::TYPE_TRUE));
}

#[test]
fn null_void_falsy() {
    assert!(p::is_falsy(prelude::TYPE_NULL));
    assert!(p::is_falsy(prelude::TYPE_VOID));
}

#[test]
fn object_always_truthy() {
    assert!(p::is_truthy(u(t_named("Foo"))));
    assert!(!p::is_falsy(u(t_named("Foo"))));
}

#[test]
fn callable_always_truthy() {
    assert!(p::is_truthy(u(t_callable_mixed())));
}

#[test]
fn class_like_string_truthy() {
    assert!(p::is_truthy(u(t_class_string())));
}

#[test]
fn lit_int_zero_falsy_nonzero_truthy() {
    assert!(p::is_falsy(u(t_lit_int(0))));
    assert!(p::is_truthy(u(t_lit_int(42))));
    assert!(p::is_truthy(u(t_lit_int(-1))));
}

#[test]
fn int_range_truthy_when_excludes_zero() {
    assert!(p::is_truthy(u(t_positive_int())));
    assert!(p::is_truthy(u(t_negative_int())));
    assert!(!p::is_truthy(u(t_int_range(0, 10))));
    assert!(!p::is_truthy(prelude::TYPE_INT));
}

#[test]
fn empty_string_falsy_truthy_string_truthy() {
    assert!(p::is_falsy(u(t_lit_string(""))));
    assert!(p::is_truthy(u(t_truthy_string())));
    assert!(p::is_truthy(u(t_lit_string("foo"))));
    // Mago's pattern: only KNOWN empty strings are guaranteed falsy.
    // The literal `"0"` is non-empty so neither guarantee holds without
    // widening it through `widen::literals` first.
    assert!(!p::is_falsy(u(t_lit_string("0"))));
    assert!(!p::is_truthy(u(t_lit_string("0"))));
}

#[test]
fn empty_array_falsy() {
    assert!(p::is_falsy(u(t_empty_array())));
}

#[test]
fn non_empty_list_truthy() {
    assert!(p::is_truthy(u(t_list(prelude::TYPE_INT, true))));
}

#[test]
fn resources_truthy_unless_explicitly_closed() {
    // Mago's pattern: `closed.is_none_or(|c| !c)`. Unknown-state and
    // open resources are truthy; only explicitly-closed resources are
    // falsy.
    assert!(p::is_truthy(u(t_open_resource())));
    assert!(p::is_falsy(u(t_closed_resource())));
    assert!(p::is_truthy(u(t_resource())));
    assert!(!p::is_falsy(u(t_resource())));
}

#[test]
fn truthy_mixed_truthy() {
    assert!(p::is_truthy(u(mixed_truthy())));
    assert!(p::is_falsy(u(mixed_falsy())));
}

#[test]
fn could_be_truthy_for_general_int() {
    assert!(p::could_be_truthy(prelude::TYPE_INT));
    assert!(p::could_be_falsy(prelude::TYPE_INT));
}

#[test]
fn could_be_truthy_excludes_never_and_void() {
    assert!(!p::could_be_truthy(prelude::TYPE_NEVER));
    assert!(!p::could_be_truthy(prelude::TYPE_VOID));
}

#[test]
fn could_be_falsy_excludes_never_but_includes_void() {
    assert!(!p::could_be_falsy(prelude::TYPE_NEVER));
    assert!(p::could_be_falsy(prelude::TYPE_VOID));
}

#[test]
fn nullable_int_could_be_both() {
    let nullable = u_many(vec![null(), t_int()]);
    assert!(p::could_be_truthy(nullable));
    assert!(p::could_be_falsy(nullable));
}

#[test]
fn is_literal_true_for_known_values() {
    assert!(p::is_literal(u(t_lit_int(42))));
    assert!(p::is_literal(u(t_lit_string("foo"))));
    assert!(p::is_literal(u(t_lit_float(1.5))));
    assert!(p::is_literal(prelude::TYPE_TRUE));
    assert!(p::is_literal(prelude::TYPE_FALSE));
    assert!(p::is_literal(prelude::TYPE_NULL));
    assert!(p::is_literal(prelude::TYPE_VOID));
}

#[test]
fn is_literal_false_for_general_forms() {
    assert!(!p::is_literal(prelude::TYPE_INT));
    assert!(!p::is_literal(prelude::TYPE_STRING));
    assert!(!p::is_literal(u(t_int_range(0, 10))));
    assert!(!p::is_literal(u(t_non_empty_string())));
}

#[test]
fn is_constant_foldable_requires_singleton() {
    assert!(p::is_constant_foldable(u(t_lit_int(42))));
    let union_lits = u_many(vec![t_lit_int(1), t_lit_int(2)]);
    assert!(p::is_literal(union_lits));
    assert!(!p::is_constant_foldable(union_lits));
}

#[test]
fn contains_template_anywhere_finds_nested() {
    use mago_atom::atom;
    use suffete::ElementId;
    use suffete::element::payload::DefiningEntity;
    let param = ElementId::generic_parameter("T", DefiningEntity::ClassLike(atom("F")), prelude::TYPE_MIXED);
    let nested = u(t_list(u(param), false));
    assert!(p::contains_template_anywhere(nested));
    assert!(!p::contains_template_anywhere(prelude::TYPE_INT));
}

#[test]
fn contains_unresolved_anywhere_finds_alias_in_list() {
    use mago_atom::atom;
    use suffete::element::payload::AliasInfo;
    use suffete::interner::interner;
    let alias = interner().intern_alias(AliasInfo { class_name: atom("Foo"), alias_name: atom("Id") });
    let nested = u(t_list(u(alias), false));
    assert!(p::contains_unresolved_anywhere(nested));
    assert!(!p::is_fully_resolved(nested));
    assert!(p::is_fully_resolved(prelude::TYPE_INT));
}

#[test]
fn contains_mixed_anywhere_walks_into_object_args() {
    let nested = u(t_generic_named("Box", vec![prelude::TYPE_MIXED]));
    assert!(p::contains_mixed_anywhere(nested));
}
