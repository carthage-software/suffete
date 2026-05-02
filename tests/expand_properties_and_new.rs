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

use std::collections::BTreeMap;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::DerivedInfo;
use suffete::element::payload::Visibility;
use suffete::expand;
use suffete::interner::interner;
use suffete::prelude;
use suffete::world::Variance;

fn t_properties_of(target: TypeId, visibility: Option<Visibility>) -> ElementId {
    interner().intern_derived(DerivedInfo::PropertiesOf { target, visibility })
}

fn t_new(target: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::New(target))
}

#[test]
fn properties_of_unknown_class_yields_empty_shape() {
    // The spec defines `properties-of<C>` as "constructs a sealed
    // keyed-array shape mapping each declared property". An unknown
    // class is observationally indistinguishable from a class with no
    // declared properties, so both produce `array{}`.
    let cb = empty_world();
    let derived = u(t_properties_of(u(t_named("Foo")), None));
    let result = expand::expand(derived, &cb);
    let expected = u(t_keyed_sealed(BTreeMap::new(), false));
    assert_eq!(result, expected);
}

#[test]
fn properties_of_class_with_no_properties_returns_empty_shape() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    let derived = u(t_properties_of(u(t_named("Foo")), None));
    let result = expand::expand(derived, &w);
    let expected = u(t_keyed_sealed(BTreeMap::new(), false));
    assert_eq!(result, expected);
}

#[test]
fn properties_of_class_with_two_properties_returns_shape() {
    let mut w = MockWorld::new();
    w.with_property("User", "name", prelude::TYPE_STRING);
    w.with_property("User", "age", prelude::TYPE_INT);
    let derived = u(t_properties_of(u(t_named("User")), None));
    let result = expand::expand(derived, &w);
    let expected = u(t_keyed_sealed(
        BTreeMap::from([(ak_str("name"), (false, prelude::TYPE_STRING)), (ak_str("age"), (false, prelude::TYPE_INT))]),
        false,
    ));
    assert_eq!(result, expected);
}

#[test]
fn properties_of_with_public_filter_drops_private() {
    let mut w = MockWorld::new();
    w.with_visible_property("User", "name", prelude::TYPE_STRING, Visibility::Public);
    w.with_visible_property("User", "secret", prelude::TYPE_STRING, Visibility::Private);
    let derived = u(t_properties_of(u(t_named("User")), Some(Visibility::Public)));
    let result = expand::expand(derived, &w);
    let expected = u(t_keyed_sealed(BTreeMap::from([(ak_str("name"), (false, prelude::TYPE_STRING))]), false));
    assert_eq!(result, expected);
}

#[test]
fn properties_of_walks_inheritance() {
    let mut w = MockWorld::new();
    w.with_property("Base", "id", prelude::TYPE_INT);
    w.with_property("Sub", "name", prelude::TYPE_STRING);
    w.add_edge("Sub", "Base");
    let derived = u(t_properties_of(u(t_named("Sub")), None));
    let result = expand::expand(derived, &w);
    let elements = result.as_ref().elements;
    assert_eq!(elements.len(), 1);
    let info = interner().get_array(elements[0]);
    let known = interner().get_known_items(info.known_items.unwrap());
    assert_eq!(known.len(), 2);
}

#[test]
fn properties_of_subclass_overrides_inherited_property() {
    // Sub redeclares "id" with a different type. Subclass-declared
    // properties precede inherited ones in the iteration order, so the
    // subclass's declaration wins in the resulting shape.
    let mut w = MockWorld::new();
    w.with_property("Base", "id", prelude::TYPE_INT);
    w.with_property("Sub", "id", prelude::TYPE_STRING);
    w.add_edge("Sub", "Base");
    let derived = u(t_properties_of(u(t_named("Sub")), None));
    let result = expand::expand(derived, &w);
    let info = interner().get_array(result.as_ref().elements[0]);
    let known = interner().get_known_items(info.known_items.unwrap());
    assert_eq!(known.len(), 1);
    assert_eq!(known[0].value, prelude::TYPE_STRING);
}

#[test]
fn properties_of_target_resolves_through_alias_first() {
    let mut w = MockWorld::new();
    w.with_property("Foo", "x", prelude::TYPE_INT);
    w.with_alias("Other", "FooAlias", u(t_named("Foo")));
    let alias_t = u(t_alias_elem("Other", "FooAlias"));
    let derived = u(t_properties_of(alias_t, None));
    let result = expand::expand(derived, &w);
    let expected = u(t_keyed_sealed(BTreeMap::from([(ak_str("x"), (false, prelude::TYPE_INT))]), false));
    assert_eq!(result, expected);
}

#[test]
fn new_with_non_generic_class_returns_object() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    let derived = u(t_new(u(t_named("Foo"))));
    let result = expand::expand(derived, &w);
    assert_eq!(result, u(t_named("Foo")));
}

#[test]
fn new_with_class_string_literal_returns_named_object() {
    let cb = empty_world();
    let class_str = u(t_lit_class_string("Foo"));
    let derived = u(t_new(class_str));
    let result = expand::expand(derived, &cb);
    assert_eq!(result, u(t_named("Foo")));
}

#[test]
fn new_with_generic_class_fills_args_with_constraints_or_mixed() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    let derived = u(t_new(u(t_named("Box"))));
    let result = expand::expand(derived, &w);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_MIXED]));
    assert_eq!(result, expected);
}

#[test]
fn new_with_generic_class_uses_template_upper_bound() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    w.with_template_bound("Box", "T", prelude::TYPE_INT);
    let derived = u(t_new(u(t_named("Box"))));
    let result = expand::expand(derived, &w);
    let expected = u(t_generic_named("Box", vec![prelude::TYPE_INT]));
    assert_eq!(result, expected);
}

#[test]
fn new_with_unresolved_target_passes_through() {
    let cb = empty_world();
    // Can't extract a single class name from a union.
    let union_target = u_many(vec![t_named("Foo"), t_named("Bar")]);
    let derived = u(t_new(union_target));
    assert_eq!(expand::expand(derived, &cb), derived);
}

#[test]
fn properties_of_inside_alias_resolves() {
    let mut w = MockWorld::new();
    w.with_property("User", "name", prelude::TYPE_STRING);
    let derived_t = u(t_properties_of(u(t_named("User")), None));
    w.with_alias("Foo", "UserShape", derived_t);
    let alias = u(t_alias_elem("Foo", "UserShape"));
    let result = expand::expand(alias, &w);
    let expected = u(t_keyed_sealed(BTreeMap::from([(ak_str("name"), (false, prelude::TYPE_STRING))]), false));
    assert_eq!(result, expected);
}

fn t_alias_elem(class: &str, alias: &str) -> ElementId {
    use suffete::element::payload::AliasInfo;
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}
