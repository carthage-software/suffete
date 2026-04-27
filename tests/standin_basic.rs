//! Standin replacement tests: bound recording at each variance,
//! co-traversal through Object / List / Iterable, and refined-type
//! shape (parameter slot becomes the constraint).

mod comparator_common;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::DefiningEntity;
use suffete::prelude;
use suffete::template;
use suffete::template::Bound;
use suffete::template::BoundKind;
use suffete::template::StandinOptions;
use suffete::template::StandinState;
use suffete::template::TemplateKey;
use suffete::world::Variance;

fn template_param(class: &str, name: &str) -> ElementId {
    suffete::ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), prelude::TYPE_MIXED)
}

fn template_param_with_constraint(class: &str, name: &str, constraint: TypeId) -> ElementId {
    suffete::ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), constraint)
}

fn key_for(class: &str, name: &str) -> TemplateKey {
    let defining_entity = suffete::interner::interner().intern_defining_entity(DefiningEntity::ClassLike(atom(class)));
    TemplateKey { defining_entity, name: atom(name) }
}

#[test]
fn top_level_template_records_invariant_bound() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = u(template_param("Box", "T"));
    let result = template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    // Refined parameter is T's constraint (mixed by default).
    assert_eq!(result, prelude::TYPE_MIXED);
    let key = key_for("Box", "T");
    let bounds = state.bounds_for(key);
    assert_eq!(bounds.len(), 1);
    assert_eq!(
        bounds[0],
        Bound {
            kind: BoundKind::Equality,
            ty: prelude::TYPE_INT,
            argument_offset: 0,
            depth: 0,
            equality_bound_classlike: None,
            span: None,
        }
    );
}

#[test]
fn top_level_template_with_int_constraint_emits_int_standin() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = u(template_param_with_constraint("Box", "T", prelude::TYPE_INT));
    let result = template::standin(t, u(t_lit_int(42)), &cb, &mut state, &opts);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn covariant_default_records_lower_bound() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default().with_default_variance(Variance::Covariant);
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Lower);
}

#[test]
fn contravariant_default_records_upper_bound() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default().with_default_variance(Variance::Contravariant);
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Upper);
}

#[test]
fn argument_offset_is_recorded() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default().with_argument_offset(3);
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].argument_offset, 3);
}

#[test]
fn template_inside_list_records_lower_bound_at_depth_one() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Box", "T");
    let param = u(t_list(u(t), false));
    let arg = u(t_list(prelude::TYPE_INT, false));
    let result = template::standin(param, arg, &cb, &mut state, &opts);
    let expected = u(t_list(prelude::TYPE_MIXED, false));
    assert_eq!(result, expected);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(
        bounds[0],
        Bound {
            kind: BoundKind::Lower,
            ty: prelude::TYPE_INT,
            argument_offset: 0,
            depth: 1,
            equality_bound_classlike: None,
            span: None,
        }
    );
}

#[test]
fn template_inside_list_against_iterable_arg_walks_value() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Box", "T");
    let param = u(t_list(u(t), false));
    let arg = u(t_iterable(prelude::TYPE_INT, prelude::TYPE_STRING));
    template::standin(param, arg, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].ty, prelude::TYPE_STRING);
}

#[test]
fn template_inside_iterable_records_both_key_and_value() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let k = template_param("M", "K");
    let v = template_param("M", "V");
    let param = u(t_iterable(u(k), u(v)));
    let arg = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    template::standin(param, arg, &cb, &mut state, &opts);
    let k_bound = state.bounds_for(key_for("M", "K"));
    let v_bound = state.bounds_for(key_for("M", "V"));
    assert_eq!(k_bound[0].ty, prelude::TYPE_STRING);
    assert_eq!(v_bound[0].ty, prelude::TYPE_INT);
}

#[test]
fn template_inside_object_uses_world_variance() {
    let mut w = MockWorld::new();
    w.with_templates("Container", &[("T", Variance::Covariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Container", "T");
    let param = u(t_generic_named("Container", vec![u(t)]));
    let arg = u(t_generic_named("Container", vec![prelude::TYPE_INT]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Container", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Lower);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn template_inside_object_with_invariant_records_equality_bound() {
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Cell", "T");
    let param = u(t_generic_named("Cell", vec![u(t)]));
    let arg = u(t_generic_named("Cell", vec![prelude::TYPE_INT]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Cell", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Equality);
}

#[test]
fn object_with_unrelated_arg_passes_parameter_through() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Covariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Box", "T");
    let param = u(t_generic_named("Box", vec![u(t)]));
    // Argument is a different class — no inference.
    let arg = u(t_generic_named("Bag", vec![prelude::TYPE_INT]));
    let result = template::standin(param, arg, &w, &mut state, &opts);
    assert_eq!(result, param);
    assert!(state.bounds_for(key_for("Box", "T")).is_empty());
}

#[test]
fn nested_object_template_records_at_correct_depth() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Covariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Box", "T");
    let inner_param = u(t_list(u(t), false));
    let param = u(t_generic_named("Box", vec![inner_param]));
    let inner_arg = u(t_list(prelude::TYPE_INT, false));
    let arg = u(t_generic_named("Box", vec![inner_arg]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    // Object args walk → depth 1; list element walk → depth 2.
    assert_eq!(bounds[0].depth, 2);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn equal_param_and_argument_short_circuits_no_changes() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let result = template::standin(prelude::TYPE_INT, prelude::TYPE_INT, &cb, &mut state, &opts);
    assert_eq!(result, prelude::TYPE_INT);
    // No template parameter mentioned → no bounds recorded.
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn invariant_object_walk_records_introducing_class_on_equality_bound() {
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Cell", "T");
    let param = u(t_generic_named("Cell", vec![u(t)]));
    let arg = u(t_generic_named("Cell", vec![prelude::TYPE_INT]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Cell", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Equality);
    assert_eq!(bounds[0].equality_bound_classlike, Some(atom("Cell")));
}

#[test]
fn covariant_object_walk_does_not_set_equality_classlike() {
    let mut w = MockWorld::new();
    w.with_templates("Container", &[("T", Variance::Covariant)]);
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("Container", "T");
    let param = u(t_generic_named("Container", vec![u(t)]));
    let arg = u(t_generic_named("Container", vec![prelude::TYPE_INT]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Container", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Lower);
    assert_eq!(bounds[0].equality_bound_classlike, None);
}

#[test]
fn top_level_invariant_walk_outside_class_has_no_classlike() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = u(template_param("Free", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Free", "T"));
    assert_eq!(bounds[0].kind, BoundKind::Equality);
    assert_eq!(bounds[0].equality_bound_classlike, None);
}

#[test]
fn span_from_options_propagates_to_recorded_bound() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let span = mago_span::Span::dummy(10, 20);
    let opts = StandinOptions::default().with_span(span);
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].span, Some(span));
}

#[test]
fn walk_auto_declares_encountered_template() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let key = key_for("Box", "T");
    assert!(state.is_declared(key));
    let decl = state.declaration(key).unwrap();
    assert_eq!(decl.constraint, prelude::TYPE_MIXED);
}

#[test]
fn walk_preserves_existing_declaration_constraint() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let key = key_for("Box", "T");
    state.declare(key, prelude::TYPE_INT);
    let opts = StandinOptions::default();
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_STRING, &cb, &mut state, &opts);
    let decl = state.declaration(key).unwrap();
    assert_eq!(decl.constraint, prelude::TYPE_INT);
}

#[test]
fn declared_but_unbound_distinguishable_from_undeclared() {
    let mut state = StandinState::new();
    let bound_key = key_for("Box", "T");
    let unbound_key = key_for("Box", "U");
    let absent_key = key_for("Box", "Z");

    state.declare(bound_key, prelude::TYPE_MIXED);
    state.declare(unbound_key, prelude::TYPE_MIXED);

    assert!(state.is_declared(bound_key));
    assert!(state.is_declared(unbound_key));
    assert!(!state.is_declared(absent_key));

    assert!(state.bounds_for(bound_key).is_empty());
    assert!(state.bounds_for(unbound_key).is_empty());
}

#[test]
fn declarations_iter_yields_every_declared_template() {
    let mut state = StandinState::new();
    state.declare(key_for("Foo", "T"), prelude::TYPE_MIXED);
    state.declare(key_for("Bar", "U"), prelude::TYPE_INT);
    let names: Vec<_> = state.declarations().map(|(k, _)| k.name).collect();
    assert!(names.contains(&atom("T")));
    assert!(names.contains(&atom("U")));
}

#[test]
fn span_threads_through_nested_walk() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Covariant)]);
    let mut state = StandinState::new();
    let span = mago_span::Span::dummy(100, 110);
    let opts = StandinOptions::default().with_span(span);
    let t = template_param("Box", "T");
    let param = u(t_generic_named("Box", vec![u(t_list(u(t), false))]));
    let arg = u(t_generic_named("Box", vec![u(t_list(prelude::TYPE_INT, false))]));
    template::standin(param, arg, &w, &mut state, &opts);
    let bounds = state.bounds_for(key_for("Box", "T"));
    assert_eq!(bounds[0].span, Some(span));
    assert_eq!(bounds[0].depth, 2);
}

#[test]
fn parameter_without_templates_passes_through_unchanged() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let result = template::standin(prelude::TYPE_INT, prelude::TYPE_STRING, &cb, &mut state, &opts);
    assert_eq!(result, prelude::TYPE_INT);
    assert_eq!(state.iter().count(), 0);
}

#[test]
fn multiple_arguments_share_state_and_accumulate_bounds() {
    let cb = empty_world();
    let mut state = StandinState::new();

    let t = template_param("F", "T");
    let param = u(t);

    let opts0 = StandinOptions::default().with_argument_offset(0).with_default_variance(Variance::Covariant);
    template::standin(param, prelude::TYPE_INT, &cb, &mut state, &opts0);

    let opts1 = StandinOptions::default().with_argument_offset(1).with_default_variance(Variance::Covariant);
    template::standin(param, prelude::TYPE_STRING, &cb, &mut state, &opts1);

    let bounds = state.bounds_for(key_for("F", "T"));
    assert_eq!(bounds.len(), 2);
    assert_eq!(bounds[0].argument_offset, 0);
    assert_eq!(bounds[0].ty, prelude::TYPE_INT);
    assert_eq!(bounds[1].argument_offset, 1);
    assert_eq!(bounds[1].ty, prelude::TYPE_STRING);
}

#[test]
fn distinct_template_parameters_recorded_separately() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let u_var = template_param("F", "U");
    let param = u(t_iterable(u(t), u(u_var)));
    let arg = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    template::standin(param, arg, &cb, &mut state, &opts);

    let t_bounds = state.bounds_for(key_for("F", "T"));
    let u_bounds = state.bounds_for(key_for("F", "U"));
    assert_eq!(t_bounds.len(), 1);
    assert_eq!(u_bounds.len(), 1);
    assert_eq!(t_bounds[0].ty, prelude::TYPE_STRING);
    assert_eq!(u_bounds[0].ty, prelude::TYPE_INT);
}

#[test]
fn iter_returns_all_recorded_keys() {
    let cb = empty_world();
    let mut state = StandinState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let u_var = template_param("F", "U");
    let param = u(t_iterable(u(t), u(u_var)));
    let arg = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    template::standin(param, arg, &cb, &mut state, &opts);

    let count = state.iter().count();
    assert_eq!(count, 2);
}
