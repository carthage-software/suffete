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
use suffete::ElementKind;
use suffete::element::payload::HasMethodInfo;
use suffete::element::payload::HasPropertyInfo;
use suffete::element::payload::KnownPropertyEntry;
use suffete::element::payload::ObjectFlags;
use suffete::element::payload::ObjectInfo;
use suffete::element::payload::ObjectShapeFlags;
use suffete::element::payload::ObjectShapeInfo;
use suffete::interner::interner;

#[test]
fn primitive_kinds_have_no_intersections_by_default() {
    for elem in [t_int(), t_string(), t_lit_int(42), null(), t_true(), t_false()] {
        assert_eq!(elem.intersection_types(), &[] as &[suffete::ElementId]);
        assert!(!elem.has_intersection_types());
        assert!(elem.can_be_intersected());
    }
}

#[test]
fn object_can_be_intersected_but_has_no_intersections_when_unset() {
    let foo = t_named("Foo");
    assert!(foo.can_be_intersected());
    assert!(!foo.has_intersection_types());
    assert!(foo.intersection_types().is_empty());
}

#[test]
fn object_with_intersections_returns_them() {
    let bar = t_named("Bar");
    let foo_and_bar = t_named_intersected("Foo", &[bar]);
    assert_eq!(foo_and_bar.kind(), ElementKind::Intersected);
    assert!(foo_and_bar.has_intersection_types());
    assert_eq!(foo_and_bar.intersection_types(), &[bar]);
}

#[test]
fn has_method_supports_intersections() {
    let i = interner();
    let other = t_has_method("bar");
    let conjuncts = i.intern_element_list(&[other]);
    let chained = i.intern_has_method(HasMethodInfo { method_name: atom("foo"), intersections: Some(conjuncts) });

    assert_eq!(chained.kind(), ElementKind::Intersected);
    assert!(chained.has_intersection_types());
    assert_eq!(chained.intersection_types(), &[other]);
}

#[test]
fn has_property_supports_intersections() {
    let i = interner();
    let other = t_has_property("y");
    let conjuncts = i.intern_element_list(&[other]);
    let chained = i.intern_has_property(HasPropertyInfo { property_name: atom("x"), intersections: Some(conjuncts) });

    assert_eq!(chained.kind(), ElementKind::Intersected);
    assert!(chained.has_intersection_types());
    assert_eq!(chained.intersection_types(), &[other]);
}

#[test]
fn object_shape_supports_intersections() {
    let i = interner();
    let entries = vec![KnownPropertyEntry { name: atom("a"), value: u(t_int()), optional: false }];
    let known = i.intern_known_properties(&entries);
    let other = t_has_method("doStuff");
    let conjuncts = i.intern_element_list(&[other]);
    let shape = i.intern_object_shape(ObjectShapeInfo {
        known_properties: Some(known),
        intersections: Some(conjuncts),
        flags: ObjectShapeFlags::default().with_sealed(true),
    });

    assert_eq!(shape.kind(), ElementKind::Intersected);
    assert_eq!(shape.intersection_types(), &[other]);
}

#[test]
fn intersection_types_descend_via_inspect() {
    use suffete::inspect;

    let i = interner();
    let inner_int_lit = suffete::ElementId::int_literal(42);
    let inner_obj = i.intern_object(ObjectInfo {
        name: atom("Marker"),
        type_args: Some(i.intern_type_list(&[u(inner_int_lit)])),
        intersections: None,
        flags: ObjectFlags::default(),
    });
    let conjuncts = i.intern_element_list(&[inner_obj]);
    let chained = i.intern_has_method(HasMethodInfo { method_name: atom("foo"), intersections: Some(conjuncts) });

    let ty = u(chained);
    assert!(
        inspect::any(ty, |e| e == inner_int_lit),
        "inspect::any should reach into HasMethod's intersection conjuncts"
    );
}

#[test]
fn intersection_round_trips_through_serializable() {
    let i = interner();
    let conjunct = t_has_method("bar");
    let conjuncts = i.intern_element_list(&[conjunct]);
    let original = i.intern_has_method(HasMethodInfo { method_name: atom("foo"), intersections: Some(conjuncts) });

    let restored = original.to_serializable().intern();
    assert_eq!(original, restored);
    assert_eq!(restored.intersection_types(), &[conjunct]);
}
