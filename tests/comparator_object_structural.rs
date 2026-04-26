//! Structural narrowings for the object family (comparison.md §1.4.6):
//! `HasMethod(m)`, `HasProperty(p)`, and `object{...}` shapes.

mod comparator_common;

use comparator_common::*;

use suffete::prelude;

#[test]
fn has_method_reflexive() {
    let cb = empty_world();
    let h = t_has_method("foo");
    assert!(atomic_is_contained(h, h, &cb));
}

#[test]
fn distinct_has_methods_dont_refine() {
    let cb = empty_world();
    let foo = t_has_method("foo");
    let bar = t_has_method("bar");
    assert!(!atomic_is_contained(foo, bar, &cb));
}

#[test]
fn named_class_with_method_refines_has_method() {
    let mut w = MockWorld::new();
    w.with_method("Foo", "bar");
    assert!(atomic_is_contained(t_named("Foo"), t_has_method("bar"), &w));
}

#[test]
fn named_class_without_method_does_not_refine_has_method() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    assert!(!atomic_is_contained(t_named("Foo"), t_has_method("bar"), &w));
}

#[test]
fn inherited_method_satisfies_has_method() {
    let mut w = MockWorld::new();
    w.with_method("Animal", "name");
    w.add_edge("Dog", "Animal");
    assert!(atomic_is_contained(t_named("Dog"), t_has_method("name"), &w));
}

#[test]
fn unrelated_class_method_does_not_satisfy() {
    let mut w = MockWorld::new();
    w.with_method("Foo", "speak");
    w.declare("Bar");
    assert!(!atomic_is_contained(t_named("Bar"), t_has_method("speak"), &w));
}

#[test]
fn object_any_does_not_refine_has_method() {
    let cb = empty_world();
    assert!(!atomic_is_contained(t_object_any(), t_has_method("foo"), &cb));
}

#[test]
fn has_method_refines_object_any() {
    let cb = empty_world();
    assert!(atomic_is_contained(t_has_method("foo"), t_object_any(), &cb));
}

#[test]
fn has_method_does_not_refine_named() {
    let mut w = MockWorld::new();
    w.with_method("Foo", "bar");
    assert!(!atomic_is_contained(t_has_method("bar"), t_named("Foo"), &w));
}

#[test]
fn has_property_reflexive() {
    let cb = empty_world();
    let h = t_has_property("name");
    assert!(atomic_is_contained(h, h, &cb));
}

#[test]
fn named_class_with_property_refines_has_property() {
    let mut w = MockWorld::new();
    w.with_property("Foo", "name", prelude::TYPE_STRING);
    assert!(atomic_is_contained(t_named("Foo"), t_has_property("name"), &w));
}

#[test]
fn inherited_property_satisfies_has_property() {
    let mut w = MockWorld::new();
    w.with_property("Animal", "name", prelude::TYPE_STRING);
    w.add_edge("Dog", "Animal");
    assert!(atomic_is_contained(t_named("Dog"), t_has_property("name"), &w));
}

#[test]
fn distinct_has_properties_dont_refine() {
    let cb = empty_world();
    assert!(!atomic_is_contained(t_has_property("name"), t_has_property("age"), &cb));
}

#[test]
fn shape_reflexive() {
    let cb = empty_world();
    let s = t_object_shape(&[("name", prelude::TYPE_STRING, false)], false);
    assert!(atomic_is_contained(s, s, &cb));
}

#[test]
fn shape_with_lit_refines_shape_with_general_via_value_covariance() {
    let cb = empty_world();
    let lit = t_object_shape(&[("age", ui(30), false)], false);
    let general = t_object_shape(&[("age", u(t_int()), false)], false);
    assert!(atomic_is_contained(lit, general, &cb));
    assert!(!atomic_is_contained(general, lit, &cb));
}

#[test]
fn shape_required_in_optional_container_refines() {
    let cb = empty_world();
    let req = t_object_shape(&[("name", u(t_string()), false)], false);
    let opt = t_object_shape(&[("name", u(t_string()), true)], false);
    assert!(atomic_is_contained(req, opt, &cb));
}

#[test]
fn shape_optional_does_not_refine_required() {
    let cb = empty_world();
    let opt = t_object_shape(&[("name", u(t_string()), true)], false);
    let req = t_object_shape(&[("name", u(t_string()), false)], false);
    assert!(!atomic_is_contained(opt, req, &cb));
}

#[test]
fn shape_missing_required_property_does_not_refine() {
    let cb = empty_world();
    let small = t_object_shape(&[("a", u(t_int()), false)], false);
    let big = t_object_shape(&[("a", u(t_int()), false), ("b", u(t_string()), false)], false);
    assert!(!atomic_is_contained(small, big, &cb));
}

#[test]
fn shape_missing_optional_property_still_refines() {
    let cb = empty_world();
    let small = t_object_shape(&[("a", u(t_int()), false)], false);
    let big = t_object_shape(&[("a", u(t_int()), false), ("b", u(t_string()), true)], false);
    assert!(atomic_is_contained(small, big, &cb));
}

#[test]
fn sealed_container_rejects_unsealed_input() {
    let cb = empty_world();
    let unsealed = t_object_shape(&[("a", u(t_int()), false)], false);
    let sealed = t_object_shape(&[("a", u(t_int()), false)], true);
    assert!(!atomic_is_contained(unsealed, sealed, &cb));
}

#[test]
fn sealed_container_rejects_input_with_extra_keys() {
    let cb = empty_world();
    let extras = t_object_shape(&[("a", u(t_int()), false), ("b", u(t_string()), false)], true);
    let sealed_a_only = t_object_shape(&[("a", u(t_int()), false)], true);
    assert!(!atomic_is_contained(extras, sealed_a_only, &cb));
}

#[test]
fn unsealed_container_accepts_input_with_extra_keys() {
    let cb = empty_world();
    let extras = t_object_shape(&[("a", u(t_int()), false), ("b", u(t_string()), false)], false);
    let unsealed_a_only = t_object_shape(&[("a", u(t_int()), false)], false);
    assert!(atomic_is_contained(extras, unsealed_a_only, &cb));
}

#[test]
fn named_class_refines_compatible_shape() {
    let mut w = MockWorld::new();
    w.with_property("User", "name", prelude::TYPE_STRING);
    w.with_property("User", "age", prelude::TYPE_INT);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("age", u(t_int()), false)], false);
    assert!(atomic_is_contained(t_named("User"), shape, &w));
}

#[test]
fn named_class_missing_required_property_rejects_shape() {
    let mut w = MockWorld::new();
    w.with_property("User", "name", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("age", u(t_int()), false)], false);
    assert!(!atomic_is_contained(t_named("User"), shape, &w));
}

#[test]
fn named_class_missing_optional_property_accepts_shape() {
    let mut w = MockWorld::new();
    w.with_property("User", "name", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("age", u(t_int()), true)], false);
    assert!(atomic_is_contained(t_named("User"), shape, &w));
}

#[test]
fn named_class_with_more_specific_property_refines_shape() {
    let mut w = MockWorld::new();
    w.with_property("Const", "value", u(t_lit_int(42)));
    let shape = t_object_shape(&[("value", u(t_int()), false)], false);
    assert!(atomic_is_contained(t_named("Const"), shape, &w));
}

#[test]
fn named_class_with_wrong_property_type_rejects_shape() {
    let mut w = MockWorld::new();
    w.with_property("Foo", "x", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("x", u(t_int()), false)], false);
    assert!(!atomic_is_contained(t_named("Foo"), shape, &w));
}

#[test]
fn inherited_property_satisfies_shape() {
    let mut w = MockWorld::new();
    w.with_property("Base", "id", prelude::TYPE_INT);
    w.add_edge("Sub", "Base");
    let shape = t_object_shape(&[("id", u(t_int()), false)], false);
    assert!(atomic_is_contained(t_named("Sub"), shape, &w));
}
