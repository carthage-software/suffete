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
use suffete::world::World as _;

fn animal_hierarchy() -> MockWorld {
    let mut cb = MockWorld::from_edges(&[
        ("Mammal", "Animal"),
        ("Dog", "Mammal"),
        ("Cat", "Mammal"),
        ("Cocker", "Dog"),
        ("Poodle", "Dog"),
        ("Sloth", "Mammal"),
        ("Sloth", "Sleepable"),
    ]);
    cb.declare("Standalone");
    cb
}

fn enums_hierarchy() -> MockWorld {
    let mut cb = MockWorld::new();
    cb.declare("Status");
    cb.declare("Color");
    cb
}

#[test]
fn object_any_reflexive() {
    let cb = empty_world();
    assert!(atomic_is_contained(t_object_any(), t_object_any(), &cb));
}

#[test]
fn named_in_object_any() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Dog"), t_object_any(), &cb));
    assert!(atomic_is_contained(t_named("Cat"), t_object_any(), &cb));
    assert!(atomic_is_contained(t_named("Standalone"), t_object_any(), &cb));
}

#[test]
fn object_any_not_in_named() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_object_any(), t_named("Dog"), &cb));
}

#[test]
fn class_reflexive() {
    let cb = animal_hierarchy();
    for n in ["Dog", "Cat", "Cocker", "Animal", "Standalone"] {
        assert!(atomic_is_contained(t_named(n), t_named(n), &cb));
    }
}

#[test]
fn subclass_in_superclass() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Cocker"), t_named("Dog"), &cb));
    assert!(atomic_is_contained(t_named("Poodle"), t_named("Dog"), &cb));
}

#[test]
fn superclass_not_in_subclass() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Dog"), t_named("Cocker"), &cb));
}

#[test]
fn class_not_in_sibling() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Cocker"), t_named("Cat"), &cb));
    assert!(!atomic_is_contained(t_named("Cocker"), t_named("Poodle"), &cb));
    assert!(!atomic_is_contained(t_named("Dog"), t_named("Cat"), &cb));
}

#[test]
fn class_implements_interface() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Dog"), t_named("Mammal"), &cb));
    assert!(atomic_is_contained(t_named("Cat"), t_named("Mammal"), &cb));
}

#[test]
fn class_implements_inherited_interface() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Dog"), t_named("Animal"), &cb));
    assert!(atomic_is_contained(t_named("Cocker"), t_named("Animal"), &cb));
}

#[test]
fn interface_in_parent_interface() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Mammal"), t_named("Animal"), &cb));
}

#[test]
fn class_implements_multiple_interfaces() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Sloth"), t_named("Mammal"), &cb));
    assert!(atomic_is_contained(t_named("Sloth"), t_named("Sleepable"), &cb));
    assert!(atomic_is_contained(t_named("Sloth"), t_named("Animal"), &cb));
}

#[test]
fn class_does_not_implement_unrelated_interface() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Dog"), t_named("Sleepable"), &cb));
    assert!(!atomic_is_contained(t_named("Cat"), t_named("Sleepable"), &cb));
    assert!(!atomic_is_contained(t_named("Standalone"), t_named("Animal"), &cb));
}

#[test]
fn parent_interface_not_in_child() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Animal"), t_named("Mammal"), &cb));
}

#[test]
fn interface_not_in_class() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Mammal"), t_named("Dog"), &cb));
    assert!(!atomic_is_contained(t_named("Animal"), t_named("Dog"), &cb));
}

#[test]
fn unrelated_classes_disjoint() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Dog"), t_named("Standalone"), &cb));
    assert!(!atomic_is_contained(t_named("Standalone"), t_named("Dog"), &cb));
}

#[test]
fn class_not_in_object_when_not_a_subtype() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Cat"), t_named("Cocker"), &cb));
}

#[test]
fn enum_reflexive() {
    let cb = enums_hierarchy();
    assert!(atomic_is_contained(t_enum("Status"), t_enum("Status"), &cb));
    assert!(atomic_is_contained(t_enum("Color"), t_enum("Color"), &cb));
}

#[test]
fn enum_case_in_enum() {
    let cb = enums_hierarchy();
    assert!(atomic_is_contained(t_enum_case("Status", "Active"), t_enum("Status"), &cb));
    assert!(atomic_is_contained(t_enum_case("Status", "Inactive"), t_enum("Status"), &cb));
    assert!(atomic_is_contained(t_enum_case("Color", "Red"), t_enum("Color"), &cb));
}

#[test]
fn enum_not_in_enum_case() {
    let cb = enums_hierarchy();
    assert!(!atomic_is_contained(t_enum("Status"), t_enum_case("Status", "Active"), &cb));
}

#[test]
fn enum_case_reflexive() {
    let cb = enums_hierarchy();
    assert!(atomic_is_contained(t_enum_case("Status", "Active"), t_enum_case("Status", "Active"), &cb));
}

#[test]
fn enum_cases_disjoint() {
    let cb = enums_hierarchy();
    assert!(!atomic_is_contained(t_enum_case("Status", "Active"), t_enum_case("Status", "Inactive"), &cb));
}

#[test]
fn distinct_enums_disjoint() {
    let cb = enums_hierarchy();
    assert!(!atomic_is_contained(t_enum("Status"), t_enum("Color"), &cb));
    assert!(!atomic_is_contained(t_enum_case("Status", "Active"), t_enum("Color"), &cb));
}

#[test]
fn class_not_in_int() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Dog"), t_int(), &cb));
}

#[test]
fn class_not_in_string() {
    let cb = animal_hierarchy();
    assert!(!atomic_is_contained(t_named("Dog"), t_string(), &cb));
}

#[test]
fn class_in_mixed() {
    let cb = animal_hierarchy();
    assert!(atomic_is_contained(t_named("Dog"), mixed(), &cb));
    assert!(atomic_is_contained(t_object_any(), mixed(), &cb));
}

#[test]
fn many_class_hierarchy_relations() {
    let cb = animal_hierarchy();
    let pairs_subtype = [
        ("Cocker", "Dog"),
        ("Cocker", "Animal"),
        ("Cocker", "Mammal"),
        ("Poodle", "Dog"),
        ("Dog", "Mammal"),
        ("Cat", "Mammal"),
        ("Mammal", "Animal"),
        ("Sloth", "Mammal"),
        ("Sloth", "Sleepable"),
        ("Sloth", "Animal"),
    ];
    for (sub, sup) in pairs_subtype {
        assert!(atomic_is_contained(t_named(sub), t_named(sup), &cb), "{sub} should be a subtype of {sup}");
    }
}

#[test]
fn many_class_hierarchy_non_relations() {
    let cb = animal_hierarchy();
    let pairs_not_subtype = [
        ("Dog", "Cat"),
        ("Cocker", "Cat"),
        ("Cocker", "Poodle"),
        ("Animal", "Mammal"),
        ("Animal", "Dog"),
        ("Mammal", "Dog"),
        ("Dog", "Sleepable"),
        ("Cat", "Sleepable"),
        ("Standalone", "Animal"),
        ("Dog", "Standalone"),
        ("Standalone", "Dog"),
    ];
    for (sub, sup) in pairs_not_subtype {
        assert!(!atomic_is_contained(t_named(sub), t_named(sup), &cb), "{sub} should NOT be a subtype of {sup}");
    }
}

// Sanity check that MockWorld reports the closure correctly.
#[test]
fn mock_world_transitive_closure() {
    let cb = animal_hierarchy();
    assert!(cb.descends_from(name("Cocker"), name("Animal")));
    assert!(cb.descends_from(name("Cocker"), name("Mammal")));
    assert!(cb.descends_from(name("Sloth"), name("Animal")));
    assert!(cb.descends_from(name("Mammal"), name("Animal")));
    assert!(!cb.descends_from(name("Animal"), name("Mammal")));
    assert!(!cb.descends_from(name("Cocker"), name("Cat")));
}
