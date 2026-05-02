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

use mago_atom::atom;

use suffete::ElementId;
use suffete::element::payload::AliasInfo;
use suffete::element::payload::ConditionalInfo;
use suffete::element::payload::DerivedInfo;
use suffete::element::payload::GlobalReference;
use suffete::element::payload::MemberReference;
use suffete::element::payload::NameSelector;
use suffete::element::payload::SymbolReference;
use suffete::element::payload::VariableInfo;
use suffete::interner::interner;
use suffete::prelude;

fn t_variable(name: &str) -> ElementId {
    interner().intern_variable(VariableInfo { name: atom(name) })
}

fn t_reference(name: &str) -> ElementId {
    interner().intern_reference(SymbolReference { name: atom(name), type_args: None, intersections: None })
}

fn t_member_ref(class: &str, member: &str) -> ElementId {
    interner().intern_member_reference(MemberReference {
        class_like_name: atom(class),
        selector: NameSelector::Identifier(atom(member)),
    })
}

fn t_global_ref(name: &str) -> ElementId {
    interner().intern_global_reference(GlobalReference { selector: NameSelector::Identifier(atom(name)) })
}

fn t_alias(class: &str, alias: &str) -> ElementId {
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}

fn t_conditional(
    subject: suffete::TypeId,
    target: suffete::TypeId,
    then: suffete::TypeId,
    otherwise: suffete::TypeId,
) -> ElementId {
    interner().intern_conditional(ConditionalInfo { subject, target, then, otherwise, negated: false })
}

fn t_keyof(target: suffete::TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::KeyOf(target))
}

#[test]
fn variable_reflexive() {
    let cb = empty_world();
    let v = t_variable("T");
    assert!(atomic_is_contained(v, v, &cb));
}

#[test]
fn distinct_variables_dont_refine() {
    let cb = empty_world();
    let t = t_variable("T");
    let u = t_variable("U");
    assert!(!atomic_is_contained(t, u, &cb));
    assert!(!atomic_is_contained(u, t, &cb));
}

#[test]
fn reference_handle_equal_refines_via_interning() {
    let cb = empty_world();
    let r1 = t_reference("Foo");
    let r2 = t_reference("Foo");
    assert!(atomic_is_contained(r1, r2, &cb));
}

#[test]
fn distinct_references_dont_refine_without_resolution() {
    let cb = empty_world();
    let foo = t_reference("Foo");
    let bar = t_reference("Bar");
    assert!(!atomic_is_contained(foo, bar, &cb));
}

#[test]
fn concrete_input_does_not_refine_unresolved_reference() {
    let cb = empty_world();
    let foo_ref = t_reference("Foo");
    assert!(!atomic_is_contained(t_int(), foo_ref, &cb));
    assert!(!atomic_is_contained(t_named("Foo"), foo_ref, &cb));
}

#[test]
fn reference_input_does_not_refine_concrete_container() {
    let cb = empty_world();
    let foo_ref = t_reference("Foo");
    assert!(!atomic_is_contained(foo_ref, t_int(), &cb));
    assert!(!atomic_is_contained(foo_ref, t_named("Foo"), &cb));
}

#[test]
fn reference_refines_mixed_via_top() {
    let cb = empty_world();
    let foo_ref = t_reference("Foo");
    assert!(atomic_is_contained(foo_ref, mixed(), &cb));
}

#[test]
fn never_refines_reference_via_bot() {
    let cb = empty_world();
    let foo_ref = t_reference("Foo");
    assert!(atomic_is_contained(never(), foo_ref, &cb));
}

#[test]
fn member_reference_handle_equality() {
    let cb = empty_world();
    let m1 = t_member_ref("Foo", "BAR");
    let m2 = t_member_ref("Foo", "BAR");
    let m3 = t_member_ref("Foo", "BAZ");
    assert!(atomic_is_contained(m1, m2, &cb));
    assert!(!atomic_is_contained(m1, m3, &cb));
}

#[test]
fn global_reference_handle_equality() {
    let cb = empty_world();
    let g1 = t_global_ref("PHP_INT_MAX");
    let g2 = t_global_ref("PHP_INT_MAX");
    let g3 = t_global_ref("PHP_INT_MIN");
    assert!(atomic_is_contained(g1, g2, &cb));
    assert!(!atomic_is_contained(g1, g3, &cb));
}

#[test]
fn alias_handle_equality() {
    let cb = empty_world();
    let a1 = t_alias("Foo", "MyAlias");
    let a2 = t_alias("Foo", "MyAlias");
    let a3 = t_alias("Foo", "OtherAlias");
    assert!(atomic_is_contained(a1, a2, &cb));
    assert!(!atomic_is_contained(a1, a3, &cb));
}

#[test]
fn conditional_handle_equality() {
    let cb = empty_world();
    let c1 = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT);
    let c2 = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT);
    let c3 = t_conditional(prelude::TYPE_STRING, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT);
    assert!(atomic_is_contained(c1, c2, &cb));
    assert!(!atomic_is_contained(c1, c3, &cb));
}

#[test]
fn derived_handle_equality() {
    let cb = empty_world();
    let d1 = t_keyof(prelude::TYPE_INT);
    let d2 = t_keyof(prelude::TYPE_INT);
    let d3 = t_keyof(prelude::TYPE_STRING);
    assert!(atomic_is_contained(d1, d2, &cb));
    assert!(!atomic_is_contained(d1, d3, &cb));
}

#[test]
fn distinct_indirection_kinds_dont_cross() {
    let cb = empty_world();
    let var = t_variable("T");
    let r = t_reference("T");
    assert!(!atomic_is_contained(var, r, &cb));
    assert!(!atomic_is_contained(r, var, &cb));
}
