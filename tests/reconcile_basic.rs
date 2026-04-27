//! Bound reconciliation (`generics.md §6`) tests: §6.3 selection rule
//! (shallowest depth, equality-marker propagation, offset matching),
//! §6.5 fallback, and the `TemplateState::witness` integration.

mod comparator_common;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::DefiningEntity;
use suffete::interner::interner;
use suffete::prelude;
use suffete::template;
use suffete::template::Bound;
use suffete::template::BoundKind;
use suffete::template::StandinOptions;
use suffete::template::TemplateState;
use suffete::template::TemplateKey;
use suffete::world::Variance;

fn template_param(class: &str, name: &str) -> ElementId {
    suffete::ElementId::generic_parameter(name, DefiningEntity::ClassLike(atom(class)), prelude::TYPE_MIXED)
}

fn key_for(class: &str, name: &str) -> TemplateKey {
    let defining_entity = interner().intern_defining_entity(DefiningEntity::ClassLike(atom(class)));
    TemplateKey { defining_entity, name: atom(name) }
}

fn bound(kind: BoundKind, ty: TypeId, depth: u32, offset: u32) -> Bound {
    Bound { kind, ty, depth, argument_offset: offset, equality_bound_classlike: None, span: None }
}

#[test]
fn empty_bounds_returns_none() {
    assert!(template::reconcile(&[]).is_none());
}

#[test]
fn single_bound_yields_that_type() {
    let r = template::reconcile(&[bound(BoundKind::Lower, prelude::TYPE_INT, 0, 0)]);
    assert_eq!(r, Some(prelude::TYPE_INT));
}

#[test]
fn two_shallow_bounds_at_same_depth_union() {
    let r = template::reconcile(&[
        bound(BoundKind::Lower, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 0, 1),
    ]);
    assert_eq!(r, Some(prelude::TYPE_INT_OR_STRING));
}

#[test]
fn deeper_bound_discarded_without_equality_marker() {
    // Two bounds at different depths, no equality marker → only
    // baseline (shallowest) is included.
    let r = template::reconcile(&[
        bound(BoundKind::Lower, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 1, 0),
    ]);
    assert_eq!(r, Some(prelude::TYPE_INT));
}

#[test]
fn deeper_bound_included_when_baseline_is_equality_marker_and_offset_matches() {
    let r = template::reconcile(&[
        bound(BoundKind::Equality, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 1, 0),
    ]);
    assert_eq!(r, Some(prelude::TYPE_INT_OR_STRING));
}

#[test]
fn deeper_bound_discarded_when_equality_marker_present_but_offset_differs() {
    let r = template::reconcile(&[
        bound(BoundKind::Equality, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 1, 1),
    ]);
    assert_eq!(r, Some(prelude::TYPE_INT));
}

#[test]
fn equality_marker_propagates_to_further_depths() {
    // Three bounds: shallow Equality at offset 0, middle Lower at
    // offset 0, deep Lower at offset 0. With equality seen at
    // baseline, both deeper bounds are included.
    let r = template::reconcile(&[
        bound(BoundKind::Equality, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 1, 0),
        bound(BoundKind::Lower, prelude::TYPE_FLOAT, 2, 0),
    ]);
    let elements = r.unwrap().as_ref().elements;
    assert_eq!(elements.len(), 3);
}

#[test]
fn unsorted_input_is_handled_correctly() {
    // Same as previous, but shuffled.
    let r = template::reconcile(&[
        bound(BoundKind::Lower, prelude::TYPE_FLOAT, 2, 0),
        bound(BoundKind::Equality, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 1, 0),
    ]);
    let elements = r.unwrap().as_ref().elements;
    assert_eq!(elements.len(), 3);
}

#[test]
fn multiple_baseline_bounds_with_equality_propagate() {
    // Two shallowest bounds, one is Equality. Equality marker is
    // seen, so a deeper bound at the same offset is included.
    let r = template::reconcile(&[
        bound(BoundKind::Lower, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Equality, prelude::TYPE_STRING, 0, 1),
        bound(BoundKind::Lower, prelude::TYPE_FLOAT, 1, 0),
    ]);
    let elements = r.unwrap().as_ref().elements;
    assert_eq!(elements.len(), 3);
}

#[test]
fn witness_falls_back_when_no_bound_collected() {
    let state = TemplateState::new();
    let key = key_for("Box", "T");
    let result = state.witness(key, prelude::TYPE_MIXED);
    assert_eq!(result, prelude::TYPE_MIXED);
}

#[test]
fn witness_uses_recorded_bounds_when_present() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let opts = StandinOptions::default().with_default_variance(Variance::Covariant);
    let t = u(template_param("Box", "T"));
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts);
    let result = state.witness(key_for("Box", "T"), prelude::TYPE_MIXED);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn witness_after_two_arguments_unions_bounds() {
    let cb = empty_world();
    let mut state = TemplateState::new();
    let t = u(template_param("F", "T"));

    let opts0 = StandinOptions::default().with_argument_offset(0).with_default_variance(Variance::Covariant);
    template::standin(t, prelude::TYPE_INT, &cb, &mut state, &opts0);
    let opts1 = StandinOptions::default().with_argument_offset(1).with_default_variance(Variance::Covariant);
    template::standin(t, prelude::TYPE_STRING, &cb, &mut state, &opts1);

    let result = state.witness(key_for("F", "T"), prelude::TYPE_MIXED);
    assert_eq!(result, prelude::TYPE_INT_OR_STRING);
}

#[test]
fn witness_after_invariant_then_nested_arg_keeps_deep_bound() {
    let mut w = MockWorld::new();
    w.with_templates("Cell", &[("T", Variance::Invariant)]);
    let mut state = TemplateState::new();
    let opts = StandinOptions::default();
    let t = template_param("F", "T");
    let param = u(t_generic_named("Cell", vec![u(t)]));
    let arg = u(t_generic_named("Cell", vec![prelude::TYPE_INT]));
    template::standin(param, arg, &w, &mut state, &opts);
    let r = state.witness(key_for("F", "T"), prelude::TYPE_MIXED);
    assert_eq!(r, prelude::TYPE_INT);
}

#[test]
fn covariant_only_bounds_keep_shallowest_only() {
    // Two arguments at different depths but only covariant. Only the
    // shallowest bound is the witness.
    let r = template::reconcile(&[
        bound(BoundKind::Lower, prelude::TYPE_INT, 0, 0),
        bound(BoundKind::Lower, prelude::TYPE_STRING, 5, 0),
    ]);
    assert_eq!(r, Some(prelude::TYPE_INT));
}

#[test]
fn upper_bound_alone_is_returned_as_witness() {
    let r = template::reconcile(&[bound(BoundKind::Upper, prelude::TYPE_STRING, 0, 0)]);
    assert_eq!(r, Some(prelude::TYPE_STRING));
}
