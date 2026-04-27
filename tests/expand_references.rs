//! Stage 2 expander tests: `SymbolReference` -> `Object`,
//! `MemberReference` -> class-constant type, `GlobalReference` ->
//! global-constant type. Wildcard selectors pass through.

mod comparator_common;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::GlobalReference;
use suffete::element::payload::MemberReference;
use suffete::element::payload::NameSelector;
use suffete::element::payload::SymbolReference;
use suffete::expand;
use suffete::interner::interner;
use suffete::prelude;

fn t_reference(name: &str) -> ElementId {
    interner().intern_reference(SymbolReference { name: atom(name), type_args: None, intersections: None })
}

fn t_reference_generic(name: &str, args: Vec<TypeId>) -> ElementId {
    let i = interner();
    interner().intern_reference(SymbolReference {
        name: atom(name),
        type_args: Some(i.intern_type_list(&args)),
        intersections: None,
    })
}

fn t_reference_intersected(name: &str, conjuncts: &[ElementId]) -> ElementId {
    let i = interner();
    interner().intern_reference(SymbolReference {
        name: atom(name),
        type_args: None,
        intersections: Some(i.intern_element_list(conjuncts)),
    })
}

fn t_member_ref(class: &str, member: &str) -> ElementId {
    interner().intern_member_reference(MemberReference {
        class_like_name: atom(class),
        selector: NameSelector::Identifier(atom(member)),
    })
}

fn t_member_ref_wildcard(class: &str) -> ElementId {
    interner()
        .intern_member_reference(MemberReference { class_like_name: atom(class), selector: NameSelector::Wildcard })
}

fn t_global_ref(name: &str) -> ElementId {
    interner().intern_global_reference(GlobalReference { selector: NameSelector::Identifier(atom(name)) })
}

#[test]
fn symbol_reference_resolves_to_named_object() {
    let cb = empty_world();
    let r = u(t_reference("Foo"));
    let result = expand::expand(r, &cb);
    let expected = u(t_named("Foo"));
    assert_eq!(result, expected);
}

#[test]
fn symbol_reference_with_type_args_resolves_to_generic_object() {
    let cb = empty_world();
    let r = u(t_reference_generic("Box", vec![prelude::TYPE_INT]));
    let result = expand::expand(r, &cb);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn nested_alias_inside_reference_type_args_expands() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", prelude::TYPE_INT);
    let alias_t = u(t_alias_via_world("Foo", "Id"));
    let r = u(t_reference_generic("Box", vec![alias_t]));
    let result = expand::expand(r, &w);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn intersected_reference_resolves_to_intersected_object() {
    let cb = empty_world();
    let r = u(t_reference_intersected("Foo", &[t_named("Bar")]));
    let result = expand::expand(r, &cb);
    // The expanded form is an Object("Foo", intersections=[Object("Bar")]).
    // Refines tests the structural shape by going through Int-L / Int-R.
    let foo_and_bar = u(t_named_intersected("Foo", &[t_named("Bar")]));
    assert_eq!(result, foo_and_bar);
}

#[test]
fn member_reference_resolves_to_constant_type() {
    let mut w = MockWorld::new();
    w.with_class_constant("Status", "ACTIVE", prelude::TYPE_INT);
    let r = u(t_member_ref("Status", "ACTIVE"));
    let result = expand::expand(r, &w);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn unknown_member_reference_passes_through() {
    let cb = empty_world();
    let r = u(t_member_ref("Status", "UNKNOWN"));
    assert_eq!(expand::expand(r, &cb), r);
}

#[test]
fn member_reference_inherits_from_ancestor() {
    let mut w = MockWorld::new();
    w.with_class_constant("Base", "ID", prelude::TYPE_STRING);
    w.add_edge("Sub", "Base");
    let r = u(t_member_ref("Sub", "ID"));
    assert_eq!(expand::expand(r, &w), prelude::TYPE_STRING);
}

#[test]
fn member_reference_with_wildcard_passes_through() {
    let mut w = MockWorld::new();
    w.with_class_constant("Status", "ACTIVE", prelude::TYPE_INT);
    let r = u(t_member_ref_wildcard("Status"));
    assert_eq!(expand::expand(r, &w), r);
}

#[test]
fn global_reference_resolves_to_constant_type() {
    let mut w = MockWorld::new();
    w.with_global_constant("PHP_INT_MAX", prelude::TYPE_INT);
    let r = u(t_global_ref("PHP_INT_MAX"));
    assert_eq!(expand::expand(r, &w), prelude::TYPE_INT);
}

#[test]
fn unknown_global_reference_passes_through() {
    let cb = empty_world();
    let r = u(t_global_ref("UNKNOWN"));
    assert_eq!(expand::expand(r, &cb), r);
}

#[test]
fn member_reference_to_union_constant_flat_merges() {
    let mut w = MockWorld::new();
    w.with_class_constant("Foo", "MIXED", prelude::TYPE_INT_OR_STRING);
    let r = u(t_member_ref("Foo", "MIXED"));
    assert_eq!(expand::expand(r, &w), prelude::TYPE_INT_OR_STRING);
}

#[test]
fn reference_inside_list_expands() {
    let cb = empty_world();
    let inner = u(t_reference("Foo"));
    let list = u(t_list(inner, false));
    let result = expand::expand(list, &cb);
    let expected = u(t_list(u(t_named("Foo")), false));
    assert_eq!(result, expected);
}

#[test]
fn member_reference_inside_iterable_expands() {
    let mut w = MockWorld::new();
    w.with_class_constant("Foo", "K", prelude::TYPE_STRING);
    let key = u(t_member_ref("Foo", "K"));
    let iter = u(t_iterable(key, prelude::TYPE_INT));
    let result = expand::expand(iter, &w);
    let expected = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    assert_eq!(result, expected);
}

#[test]
fn chained_alias_then_reference_resolves() {
    // type Foo::Id = Reference("Bar"); resolves to Object("Bar").
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Id", u(t_reference("Bar")));
    let alias_t = u(t_alias_via_world("Foo", "Id"));
    assert_eq!(expand::expand(alias_t, &w), u(t_named("Bar")));
}

// ---- helpers ----

fn t_alias_via_world(class: &str, alias: &str) -> ElementId {
    use suffete::element::payload::AliasInfo;
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}
