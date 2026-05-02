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

#[test]
fn class_string_reflexive() {
    assert_atomic_subtype(t_class_string(), t_class_string());
}

#[test]
fn interface_string_reflexive() {
    assert_atomic_subtype(t_interface_string(), t_interface_string());
}

#[test]
fn enum_string_reflexive() {
    assert_atomic_subtype(t_enum_string(), t_enum_string());
}

#[test]
fn trait_string_reflexive() {
    assert_atomic_subtype(t_trait_string(), t_trait_string());
}

#[test]
fn lit_class_string_reflexive() {
    for n in ["Foo", "App\\Bar", "Vendor\\Pkg\\X"] {
        assert_atomic_subtype(t_lit_class_string(n), t_lit_class_string(n));
    }
}

#[test]
fn lit_class_string_in_class_string() {
    for n in ["Foo", "Bar", "App\\Service"] {
        assert_atomic_subtype(t_lit_class_string(n), t_class_string());
    }
}

#[test]
fn class_string_not_in_lit_class_string() {
    assert_atomic_not_subtype(t_class_string(), t_lit_class_string("Foo"));
}

#[test]
fn class_string_in_string() {
    assert_atomic_subtype(t_class_string(), t_string());
}

#[test]
fn interface_string_in_string() {
    assert_atomic_subtype(t_interface_string(), t_string());
}

#[test]
fn enum_string_in_string() {
    assert_atomic_subtype(t_enum_string(), t_string());
}

#[test]
fn trait_string_in_string() {
    assert_atomic_subtype(t_trait_string(), t_string());
}

#[test]
fn class_string_in_array_key() {
    assert_atomic_subtype(t_class_string(), t_array_key());
}

#[test]
fn class_string_in_scalar() {
    assert_atomic_subtype(t_class_string(), t_scalar());
}

#[test]
fn class_string_not_in_int() {
    assert_atomic_not_subtype(t_class_string(), t_int());
}

#[test]
fn class_string_not_in_numeric() {
    assert_atomic_not_subtype(t_class_string(), t_numeric());
}

#[test]
fn lit_class_string_in_string() {
    assert_atomic_subtype(t_lit_class_string("Foo"), t_string());
}

#[test]
fn lit_class_string_in_array_key() {
    assert_atomic_subtype(t_lit_class_string("Foo"), t_array_key());
}

#[test]
fn distinct_lit_class_strings_disjoint() {
    assert_atomic_not_subtype(t_lit_class_string("Foo"), t_lit_class_string("Bar"));
}

#[test]
fn many_lit_class_strings_in_class_string() {
    for i in 0..30 {
        let n = format!("Class_{i}");
        assert_atomic_subtype(t_lit_class_string(&n), t_class_string());
    }
}

#[test]
fn lit_class_string_in_class_string_of_self() {
    let foo = t_class_string_of(u(t_named("Foo")));
    assert_atomic_subtype(t_lit_class_string("Foo"), foo);
}

#[test]
fn lit_class_string_in_class_string_of_ancestor() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo");
    let foo = t_class_string_of(u(t_named("Foo")));
    assert!(atomic_is_contained(t_lit_class_string("Bar"), foo, &w));
}

#[test]
fn lit_class_string_not_in_class_string_of_unrelated() {
    let w = MockWorld::new();
    let foo = t_class_string_of(u(t_named("Foo")));
    assert!(!atomic_is_contained(t_lit_class_string("Bar"), foo, &w));
}

#[test]
fn class_string_of_child_in_class_string_of_parent() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo");
    let parent = t_class_string_of(u(t_named("Foo")));
    let child = t_class_string_of(u(t_named("Bar")));
    assert!(atomic_is_contained(child, parent, &w));
}

#[test]
fn class_string_of_parent_not_in_class_string_of_child() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo");
    let parent = t_class_string_of(u(t_named("Foo")));
    let child = t_class_string_of(u(t_named("Bar")));
    assert!(!atomic_is_contained(parent, child, &w));
}

#[test]
fn class_string_of_unrelated_classes_disjoint() {
    let w = MockWorld::new();
    let foo = t_class_string_of(u(t_named("Foo")));
    let bar = t_class_string_of(u(t_named("Bar")));
    assert!(!atomic_is_contained(foo, bar, &w));
    assert!(!atomic_is_contained(bar, foo, &w));
}

#[test]
fn class_string_kinds_disjoint() {
    let w = MockWorld::new();
    let class_of_foo = t_class_string_of(u(t_named("Foo")));
    let interface_of_foo = t_interface_string_of(u(t_named("Foo")));
    assert!(!atomic_is_contained(class_of_foo, interface_of_foo, &w));
    assert!(!atomic_is_contained(interface_of_foo, class_of_foo, &w));
}

#[test]
fn lit_string_of_valid_class_name_fits_class_string() {
    assert_atomic_subtype(t_lit_string("Foo"), t_class_string());
    assert_atomic_subtype(t_lit_string("App\\Service"), t_class_string());
}

#[test]
fn lit_string_of_invalid_class_name_does_not_fit_class_string() {
    assert_atomic_not_subtype(t_lit_string(""), t_class_string());
    assert_atomic_not_subtype(t_lit_string("1Foo"), t_class_string());
    assert_atomic_not_subtype(t_lit_string("Foo Bar"), t_class_string());
    assert_atomic_not_subtype(t_lit_string("Foo\\"), t_class_string());
}

#[test]
fn lit_string_in_class_string_of_matching_class() {
    let foo = t_class_string_of(u(t_named("Foo")));
    let w = MockWorld::new();
    assert!(atomic_is_contained(t_lit_string("Foo"), foo, &w));
}

#[test]
fn lit_string_of_descendant_in_class_string_of_parent() {
    let mut w = MockWorld::new();
    w.add_edge("Bar", "Foo");
    let foo = t_class_string_of(u(t_named("Foo")));
    assert!(atomic_is_contained(t_lit_string("Bar"), foo, &w));
}

#[test]
fn lit_string_of_pure_enum_routes_through_enum_kind() {
    // When the world reports `Color` as an enum, the literal string
    // "Color" must be resolved to an enum element so it fits an
    // enum-string container, not just a class container.
    let mut w = MockWorld::new();
    w.with_pure_enum("Color");
    let any_enum = t_enum_string();
    assert!(atomic_is_contained(t_lit_string("Color"), any_enum, &w));
}

#[test]
fn lit_string_of_class_does_not_fit_enum_string_container() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    let any_enum = t_enum_string();
    // `Foo` is a class in the world's eyes; the kind check rejects it
    // before any object-level comparison runs.
    assert!(!atomic_is_contained(t_lit_string("Foo"), any_enum, &w));
}
