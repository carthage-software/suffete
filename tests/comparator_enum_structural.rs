//! Enum cases are structurally `object{name: non-empty-string}` for pure
//! enums and `object{name: non-empty-string, value: <backing>}` for
//! backed enums. These tests pin down the resulting subtype edges.

mod comparator_common;

use comparator_common::*;

use suffete::prelude;

#[test]
fn pure_enum_has_name_property() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Status");
    assert!(atomic_is_contained(t_enum("Status"), t_has_property("name"), &w));
}

#[test]
fn pure_enum_does_not_have_value_property() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Status");
    assert!(!atomic_is_contained(t_enum("Status"), t_has_property("value"), &w));
}

#[test]
fn backed_enum_has_value_property() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Status", prelude::TYPE_STRING);
    assert!(atomic_is_contained(t_enum("Status"), t_has_property("value"), &w));
    assert!(atomic_is_contained(t_enum("Status"), t_has_property("name"), &w));
}

#[test]
fn unknown_enum_rejects_value_but_keeps_name() {
    // The element-kind tag (`Enum`) already certifies the value is an
    // enum case, so `name` is always present even without a World entry.
    // `value` requires confirming the backing, which an empty world cannot.
    let cb = empty_world();
    assert!(!atomic_is_contained(t_enum("Status"), t_has_property("value"), &cb));
    assert!(atomic_is_contained(t_enum("Status"), t_has_property("name"), &cb));
}

#[test]
fn backed_string_enum_refines_name_value_string_shape() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Status", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_string()), false)], false);
    assert!(atomic_is_contained(t_enum("Status"), shape, &w));
}

#[test]
fn backed_string_enum_does_not_refine_int_value_shape() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Status", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_int()), false)], false);
    assert!(!atomic_is_contained(t_enum("Status"), shape, &w));
}

#[test]
fn backed_int_enum_refines_int_value_shape() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Priority", prelude::TYPE_INT);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_int()), false)], false);
    assert!(atomic_is_contained(t_enum("Priority"), shape, &w));
}

#[test]
fn pure_enum_refines_name_only_shape() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", u(t_string()), false)], false);
    assert!(atomic_is_contained(t_enum("Color"), shape, &w));
}

#[test]
fn pure_enum_does_not_refine_shape_demanding_value() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_string()), false)], false);
    assert!(!atomic_is_contained(t_enum("Color"), shape, &w));
}

#[test]
fn pure_enum_refines_shape_with_optional_value() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_string()), true)], false);
    assert!(atomic_is_contained(t_enum("Color"), shape, &w));
}

#[test]
fn enum_name_property_is_non_empty_string() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", u(t_non_empty_string()), false)], false);
    assert!(atomic_is_contained(t_enum("Color"), shape, &w));
}

#[test]
fn specific_enum_case_refines_shape_with_lit_name() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", us("Red"), false)], false);
    assert!(atomic_is_contained(t_enum_case("Color", "Red"), shape, &w));
}

#[test]
fn specific_enum_case_does_not_refine_lit_name_of_different_case() {
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let shape = t_object_shape(&[("name", us("Blue"), false)], false);
    assert!(!atomic_is_contained(t_enum_case("Color", "Red"), shape, &w));
}

#[test]
fn enum_does_not_refine_sealed_shape_with_extra_required_key() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Status", prelude::TYPE_STRING);
    let shape = t_object_shape(
        &[("name", u(t_string()), false), ("value", u(t_string()), false), ("extra", u(t_int()), false)],
        true,
    );
    assert!(!atomic_is_contained(t_enum("Status"), shape, &w));
}

#[test]
fn enum_refines_sealed_shape_matching_exactly() {
    let mut w = MockWorld::new();
    w.with_backed_enum("Status", prelude::TYPE_STRING);
    let shape = t_object_shape(&[("name", u(t_string()), false), ("value", u(t_string()), false)], true);
    assert!(atomic_is_contained(t_enum("Status"), shape, &w));
}

#[test]
fn unknown_enum_rejects_object_shape() {
    let cb = empty_world();
    let shape = t_object_shape(&[("name", u(t_string()), false)], false);
    assert!(!atomic_is_contained(t_enum("Status"), shape, &cb));
}
