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

use mago_atom::atom;

use suffete::ElementId;
use suffete::element::payload::AliasInfo;
use suffete::element::payload::DefiningEntity;
use suffete::element::payload::GlobalReference;
use suffete::element::payload::MemberReference;
use suffete::element::payload::NameSelector;
use suffete::element::payload::ObjectFlags;
use suffete::element::payload::ObjectInfo;
use suffete::expand;
use suffete::expand::ExpansionContext;
use suffete::interner::interner;
use suffete::prelude;
use suffete::world::Variance;

fn alias_atom(class: &str, alias: &str) -> ElementId {
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}

fn class_const_atom(class: &str, name: &str) -> ElementId {
    interner().intern_member_reference(MemberReference {
        class_like_name: atom(class),
        selector: NameSelector::Identifier(atom(name)),
    })
}

fn global_const_atom(name: &str) -> ElementId {
    interner().intern_global_reference(GlobalReference { selector: NameSelector::Identifier(atom(name)) })
}

fn template_atom(class: &str, name: &str, constraint: suffete::TypeId) -> ElementId {
    suffete::ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), constraint)
}

#[test]
fn alias_resolves_when_eval_aliases_on() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Body", prelude::TYPE_INT);
    let ty = u(alias_atom("Foo", "Body"));
    assert_eq!(expand::expand_with(ty, &w, &ExpansionContext::default()), prelude::TYPE_INT);
}

#[test]
fn alias_passes_through_when_eval_aliases_off() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Body", prelude::TYPE_INT);
    let ty = u(alias_atom("Foo", "Body"));
    let ctx = ExpansionContext::default().with_eval_aliases(false);
    assert_eq!(expand::expand_with(ty, &w, &ctx), ty);
}

#[test]
fn class_constant_resolves_when_flag_on() {
    let mut w = MockWorld::new();
    w.with_class_constant("Foo", "BAR", prelude::TYPE_INT);
    let ty = u(class_const_atom("Foo", "BAR"));
    assert_eq!(expand::expand_with(ty, &w, &ExpansionContext::default()), prelude::TYPE_INT);
}

#[test]
fn class_constant_passes_through_when_flag_off() {
    let mut w = MockWorld::new();
    w.with_class_constant("Foo", "BAR", prelude::TYPE_INT);
    let ty = u(class_const_atom("Foo", "BAR"));
    let ctx = ExpansionContext::default().with_eval_class_constants(false);
    assert_eq!(expand::expand_with(ty, &w, &ctx), ty);
}

#[test]
fn global_constant_resolves_when_flag_on() {
    let mut w = MockWorld::new();
    w.with_global_constant("VERSION", prelude::TYPE_STRING);
    let ty = u(global_const_atom("VERSION"));
    assert_eq!(expand::expand_with(ty, &w, &ExpansionContext::default()), prelude::TYPE_STRING);
}

#[test]
fn global_constant_passes_through_when_flag_off() {
    let mut w = MockWorld::new();
    w.with_global_constant("VERSION", prelude::TYPE_STRING);
    let ty = u(global_const_atom("VERSION"));
    let ctx = ExpansionContext::default().with_eval_global_constants(false);
    assert_eq!(expand::expand_with(ty, &w, &ctx), ty);
}

#[test]
fn unfilled_generic_object_filled_when_flag_on() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    let unfilled = u(t_named("Box"));
    let ctx = ExpansionContext::default().with_fill_template_defaults(true);
    let result = expand::expand_with(unfilled, &w, &ctx);
    assert_eq!(result, u(t_generic_named("Box", vec![prelude::TYPE_MIXED])));
}

#[test]
fn unfilled_generic_object_uses_declared_upper_bound_when_present() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    w.with_template_bound("Box", "T", prelude::TYPE_INT);
    let unfilled = u(t_named("Box"));
    let ctx = ExpansionContext::default().with_fill_template_defaults(true);
    let result = expand::expand_with(unfilled, &w, &ctx);
    assert_eq!(result, u(t_generic_named("Box", vec![prelude::TYPE_INT])));
}

#[test]
fn unfilled_generic_object_unchanged_when_flag_off() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Invariant)]);
    let unfilled = u(t_named("Box"));
    assert_eq!(expand::expand_with(unfilled, &w, &ExpansionContext::default()), unfilled);
}

#[test]
fn template_constraint_substituted_when_flag_on() {
    let cb = empty_world();
    let t = u(template_atom("Foo", "T", prelude::TYPE_INT));
    let ctx = ExpansionContext::default().with_substitute_template_constraints(true);
    assert_eq!(expand::expand_with(t, &cb, &ctx), prelude::TYPE_INT);
}

#[test]
fn template_passes_through_when_constraint_flag_off() {
    let cb = empty_world();
    let t = u(template_atom("Foo", "T", prelude::TYPE_INT));
    assert_eq!(expand::expand_with(t, &cb, &ExpansionContext::default()), t);
}

#[test]
fn function_is_final_collapses_static_modality_without_static_class() {
    let cb = empty_world();
    let static_obj = interner().intern_object(ObjectInfo {
        name: atom("Foo"),
        type_args: None,
        flags: ObjectFlags::default().with_is_static(true),
    });
    let plain = u(t_named("Foo"));
    let ty = u(static_obj);
    let ctx = ExpansionContext::default().with_function_is_final(true);
    assert_eq!(expand::expand_with(ty, &cb, &ctx), plain);
}

#[test]
fn static_modality_preserved_when_function_is_final_off() {
    let cb = empty_world();
    let static_obj = interner().intern_object(ObjectInfo {
        name: atom("Foo"),
        type_args: None,
        flags: ObjectFlags::default().with_is_static(true),
    });
    let ty = u(static_obj);
    assert_eq!(expand::expand_with(ty, &cb, &ExpansionContext::default()), ty);
}
