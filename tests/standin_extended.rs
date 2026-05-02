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

use std::collections::BTreeMap;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::element::payload::DefiningEntity;
use suffete::interner::interner;
use suffete::prelude;
use suffete::template;
use suffete::template::BoundKind;
use suffete::template::StandinOptions;
use suffete::template::TemplateKey;
use suffete::template::TemplateState;
use suffete::world::Variance;

fn template_param(class: &str, name: &str) -> ElementId {
    suffete::ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), prelude::TYPE_MIXED)
}

fn key_for(class: &str, name: &str) -> TemplateKey {
    let defining_entity = interner().intern_defining_entity(DefiningEntity::ClassLike(atom(class)));
    TemplateKey { defining_entity, name: atom(name) }
}

#[test]
fn keyed_array_value_param_records_lower_bound() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_keyed_unsealed(prelude::TYPE_STRING, u(t), false));
    let arg = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    template::standin(param, arg, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].kind, BoundKind::Lower);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn keyed_array_known_item_walked_when_arg_has_matching_key() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_keyed_sealed(BTreeMap::from([(ak_str("name"), (false, u(t)))]), false));
    let arg = u(t_keyed_sealed(BTreeMap::from([(ak_str("name"), (false, prelude::TYPE_STRING))]), false));
    template::standin(param, arg, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds[0].ty, prelude::TYPE_STRING);
}

#[test]
fn keyed_array_against_iterable_walks_key_and_value_params() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let k = template_param("F", "K");
    let v = template_param("F", "V");
    let param = u(t_keyed_unsealed(u(k), u(v), false));
    let arg = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    template::standin(param, arg, &cb, &mut state, &opts);
    assert_eq!(state.bounds_for(key_for("F", "K"))[0].ty, prelude::TYPE_STRING);
    assert_eq!(state.bounds_for(key_for("F", "V"))[0].ty, prelude::TYPE_INT);
}

#[test]
fn callable_return_walked_covariantly() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_callable(&[], u(t)));
    let arg = u(t_callable(&[], prelude::TYPE_INT));
    template::standin(param, arg, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].kind, BoundKind::Lower);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn callable_parameter_walked_contravariantly() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_callable(&[u(t)], prelude::TYPE_VOID));
    let arg = u(t_callable(&[prelude::TYPE_INT], prelude::TYPE_VOID));
    template::standin(param, arg, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].kind, BoundKind::Upper);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn callable_records_both_param_and_return_bounds() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let p = template_param("F", "P");
    let r = template_param("F", "R");
    let param = u(t_callable(&[u(p)], u(r)));
    let arg = u(t_callable(&[prelude::TYPE_INT], prelude::TYPE_STRING));
    template::standin(param, arg, &cb, &mut state, &opts);
    assert_eq!(state.bounds_for(key_for("F", "P"))[0].kind, BoundKind::Upper);
    assert_eq!(state.bounds_for(key_for("F", "P"))[0].ty, prelude::TYPE_INT);
    assert_eq!(state.bounds_for(key_for("F", "R"))[0].kind, BoundKind::Lower);
    assert_eq!(state.bounds_for(key_for("F", "R"))[0].ty, prelude::TYPE_STRING);
}

#[test]
fn descendant_class_arg_threads_through_extension_binding() {
    // class A<T>; class B extends A<int>;
    // Param: A<T_F> against argument B → T_F should infer to int.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Covariant)]);
    w.declare("B");
    w.with_extended("B", "A", vec![prelude::TYPE_INT]);

    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_generic_named("A", vec![u(t)]));
    let arg = u(t_named("B"));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds.len(), 1);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn descendant_class_arg_substitutes_own_template_args() {
    // class A<T>; class B<U> extends A<U>;
    // Param: A<T_F> against argument B<string> → T_F should infer to string.
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Covariant)]);
    w.with_templates("B", &[("U", Variance::Covariant)]);
    w.with_extended("B", "A", vec![u(t_template("B", "U"))]);

    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_generic_named("A", vec![u(t)]));
    let arg = u(t_generic_named("B", vec![prelude::TYPE_STRING]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds[0].ty, prelude::TYPE_STRING);
}

#[test]
fn iteration_depth_cutoff_replaces_template_with_constraint() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default().with_max_depth(0);
    let t = template_param("F", "T");
    let param = u(t_list(u(t), false));
    let arg = u(t_list(prelude::TYPE_INT, false));
    template::standin(param, arg, &cb, &mut state, &opts);
    // Walking past depth 0 collapses to the constraint without recording.
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn iteration_depth_zero_records_top_level_binding() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default().with_max_depth(0);
    let t = template_param("F", "T");
    let param = u(t);
    template::standin(param, prelude::TYPE_INT, &cb, &mut state, &opts);
    // Depth 0 walk fires before the cutoff check on its descent.
    assert_eq!(state.bounds_for(key_for("F", "T")).len(), 1);
}

#[test]
fn keyed_array_unchanged_when_no_template() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let param = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let arg = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let result = template::standin(param, arg, &cb, &mut state, &opts);
    assert_eq!(result, param);
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn callable_unchanged_when_no_template() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let param = u(t_callable(&[prelude::TYPE_INT], prelude::TYPE_STRING));
    let result = template::standin(param, param, &cb, &mut state, &opts);
    assert_eq!(result, param);
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn descendant_with_no_extension_binding_passes_through() {
    let mut w = MockWorld::new();
    w.with_templates("A", &[("T", Variance::Covariant)]);
    w.declare("B");
    w.add_edge("B", "A");
    // No `with_extended` for B → A: world has no inherited binding.

    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_generic_named("A", vec![u(t)]));
    let arg = u(t_named("B"));
    let result = template::standin(param, arg, &w, &mut state, &opts);
    assert_eq!(result, param);
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn keyed_array_known_value_against_lit_walks_to_lit_bound() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_keyed_sealed(BTreeMap::from([(ak_str("v"), (false, u(t)))]), false));
    let arg = u(t_keyed_sealed(BTreeMap::from([(ak_str("v"), (false, ui(42)))]), false));
    template::standin(param, arg, &cb, &mut state, &opts);
    assert_eq!(state.bounds_for(key_for("F", "T"))[0].ty, ui(42));
}
