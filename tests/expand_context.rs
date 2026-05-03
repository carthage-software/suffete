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
use suffete::TypeId;
use suffete::element::payload::ConditionalInfo;
use suffete::element::payload::ObjectFlags;
use suffete::element::payload::ObjectInfo;
use suffete::expand;
use suffete::expand::ExpansionContext;
use suffete::interner::interner;
use suffete::prelude;

fn t_object_with_flags(name: &str, is_static: bool, is_this: bool) -> ElementId {
    interner().intern_object(ObjectInfo {
        name: atom(name),
        type_args: None,
        flags: ObjectFlags::default().with_is_static(is_static).with_is_this(is_this),
    })
}

fn t_conditional(subject: TypeId, target: TypeId, then: TypeId, otherwise: TypeId, negated: bool) -> ElementId {
    interner().intern_conditional(ConditionalInfo { subject, target, then, otherwise, negated })
}

#[test]
fn self_keyword_substitutes_with_self_class() {
    let cb = empty_world();
    let self_obj = u(t_named("self"));
    let ctx = ExpansionContext::default().with_self_class(atom("Foo"));
    assert_eq!(expand::expand_with(self_obj, &cb, &ctx), u(t_named("Foo")));
}

#[test]
fn static_keyword_substitutes_with_static_class() {
    let cb = empty_world();
    let static_obj = u(t_named("static"));
    let ctx = ExpansionContext::default().with_static_class(atom("Foo"));
    assert_eq!(expand::expand_with(static_obj, &cb, &ctx), u(t_named("Foo")));
}

#[test]
fn parent_keyword_substitutes_with_parent_class() {
    let cb = empty_world();
    let parent_obj = u(t_named("parent"));
    let ctx = ExpansionContext::default().with_parent_class(atom("Animal"));
    assert_eq!(expand::expand_with(parent_obj, &cb, &ctx), u(t_named("Animal")));
}

#[test]
fn is_static_flag_resolved_with_static_class() {
    let cb = empty_world();
    let elem = t_object_with_flags("Foo", true, false);
    let ctx = ExpansionContext::default().with_static_class(atom("Sub"));
    let result = expand::expand_with(u(elem), &cb, &ctx);
    assert_eq!(result, u(t_named("Sub")));
}

#[test]
fn is_this_flag_resolved_with_static_class() {
    let cb = empty_world();
    let elem = t_object_with_flags("Foo", false, true);
    let ctx = ExpansionContext::default().with_static_class(atom("Sub"));
    let result = expand::expand_with(u(elem), &cb, &ctx);
    assert_eq!(result, u(t_named("Sub")));
}

#[test]
fn keyword_without_context_passes_through() {
    let cb = empty_world();
    let self_obj = u(t_named("self"));
    let ctx = ExpansionContext::default();
    assert_eq!(expand::expand_with(self_obj, &cb, &ctx), self_obj);
}

#[test]
fn plain_named_object_unaffected_by_context() {
    let cb = empty_world();
    let foo = u(t_named("Foo"));
    let ctx = ExpansionContext::default().with_self_class(atom("Bar"));
    assert_eq!(expand::expand_with(foo, &cb, &ctx), foo);
}

#[test]
fn keyword_inside_list_resolves() {
    let cb = empty_world();
    let self_obj = u(t_named("self"));
    let list = u(t_list(self_obj, false));
    let ctx = ExpansionContext::default().with_self_class(atom("Foo"));
    let result = expand::expand_with(list, &cb, &ctx);
    let expected = u(t_list(u(t_named("Foo")), false));
    assert_eq!(result, expected);
}

#[test]
fn conditional_passes_through_when_eval_off() {
    let cb = empty_world();
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT, false);
    let ty = u(cond);
    let ctx = ExpansionContext::default(); // eval_conditional = false
    assert_eq!(expand::expand_with(ty, &cb, &ctx), ty);
}

#[test]
fn conditional_picks_then_branch_when_test_passes() {
    let cb = empty_world();
    // (int <: int) ? string : float -> string
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT, false);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    assert_eq!(expand::expand_with(u(cond), &cb, &ctx), prelude::TYPE_STRING);
}

#[test]
fn conditional_picks_otherwise_when_test_disjoint() {
    let cb = empty_world();
    // (int <: string)? string : float -> int and string are disjoint -> float
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_STRING, prelude::TYPE_FLOAT, false);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    assert_eq!(expand::expand_with(u(cond), &cb, &ctx), prelude::TYPE_FLOAT);
}

#[test]
fn conditional_widens_to_union_when_undecidable() {
    let cb = empty_world();
    // (int|string) is int ? float : bool
    // int|string neither refines int (string isn't an int) nor disjoint (int is in both).
    let mixed_input = u_many(vec![t_int(), t_string()]);
    let cond = t_conditional(mixed_input, prelude::TYPE_INT, prelude::TYPE_FLOAT, prelude::TYPE_BOOL, false);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    let result = expand::expand_with(u(cond), &cb, &ctx);
    let expected = u_many(vec![suffete::prelude::FLOAT, suffete::prelude::BOOL]);
    assert_eq!(result, expected);
}

#[test]
fn negated_conditional_swaps_branches_on_pass() {
    let cb = empty_world();
    // (int is not int) ? string : float -> test "is not" fails since int IS int -> otherwise = float
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT, true);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    assert_eq!(expand::expand_with(u(cond), &cb, &ctx), prelude::TYPE_FLOAT);
}

#[test]
fn negated_conditional_swaps_branches_on_disjoint() {
    let cb = empty_world();
    // (int is not string) ? string : float -> int IS NOT string -> then = string
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_STRING, prelude::TYPE_FLOAT, true);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    assert_eq!(expand::expand_with(u(cond), &cb, &ctx), prelude::TYPE_STRING);
}

#[test]
fn conditional_with_alias_inside_branch_resolves_after_pick() {
    let mut w = MockWorld::new();
    w.with_alias("Foo", "Result", prelude::TYPE_STRING);
    let alias = u(suffete::ElementId::int_literal(0)); // dummy placeholder; replaced below
    let _ = alias;
    let alias_t = {
        use suffete::element::payload::AliasInfo;
        u(interner().intern_alias(AliasInfo { class_name: atom("Foo"), alias_name: atom("Result") }))
    };
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, alias_t, prelude::TYPE_FLOAT, false);
    let ctx = ExpansionContext::default().with_eval_conditional(true);
    assert_eq!(expand::expand_with(u(cond), &w, &ctx), prelude::TYPE_STRING);
}

#[test]
fn expand_default_wrapper_uses_default_context() {
    let cb = empty_world();
    // Without an explicit context, conditionals don't evaluate.
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, prelude::TYPE_STRING, prelude::TYPE_FLOAT, false);
    let ty = u(cond);
    assert_eq!(expand::expand(ty, &cb), ty);
}

#[test]
fn keyword_inside_generic_object_args_resolves() {
    let cb = empty_world();
    let self_obj = u(t_named("self"));
    let box_of_self = u(t_generic_named("Box", vec![self_obj]));
    let ctx = ExpansionContext::default().with_self_class(atom("Foo"));
    let result = expand::expand_with(box_of_self, &cb, &ctx);
    let expected = u(t_generic_named("Box", vec![u(t_named("Foo"))]));
    assert_eq!(result, expected);
}

#[test]
fn keyword_in_conditional_branch_resolves() {
    let cb = empty_world();
    let self_obj = u(t_named("self"));
    let cond = t_conditional(prelude::TYPE_INT, prelude::TYPE_INT, self_obj, prelude::TYPE_FLOAT, false);
    let ctx = ExpansionContext::default().with_eval_conditional(true).with_self_class(atom("Bar"));
    assert_eq!(expand::expand_with(u(cond), &cb, &ctx), u(t_named("Bar")));
}
