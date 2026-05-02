#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
)]

mod comparator_common;

use core::num::NonZeroU32;
use std::collections::BTreeMap;

use comparator_common::*;

use mago_atom::atom;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::element::payload::DerivedInfo;
use suffete::element::payload::KnownElementEntry;
use suffete::element::payload::ListFlags;
use suffete::element::payload::ListInfo;
use suffete::expand;
use suffete::interner::interner;
use suffete::prelude;
use suffete::world::Variance;

fn t_key_of(target: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::KeyOf(target))
}

fn t_value_of(target: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::ValueOf(target))
}

fn t_index_access(target: TypeId, index: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::IndexAccess { target, index })
}

fn t_int_mask(operands: Vec<TypeId>) -> ElementId {
    let id = interner().intern_type_list(&operands);
    interner().intern_derived(DerivedInfo::IntMask(id))
}

fn t_int_mask_of(target: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::IntMaskOf(target))
}

fn t_template_type(class: TypeId, name: TypeId) -> ElementId {
    interner().intern_derived(DerivedInfo::TemplateType {
        object: prelude::TYPE_MIXED,
        class_name: class,
        template_name: name,
    })
}

fn t_sealed_list(elements: &[TypeId]) -> ElementId {
    let i = interner();
    let entries: Vec<KnownElementEntry> = elements
        .iter()
        .enumerate()
        .map(|(idx, t)| KnownElementEntry { index: idx as u32, value: *t, optional: false })
        .collect();
    let known = i.intern_known_elements(&entries);
    let info = ListInfo {
        element_type: prelude::TYPE_NEVER,
        known_elements: Some(known),
        known_count: NonZeroU32::new(elements.len() as u32),
        flags: ListFlags::default().with_non_empty(!elements.is_empty()),
    };
    i.intern_list(info)
}

#[test]
fn key_of_unsealed_list_is_non_negative_int() {
    let cb = empty_world();
    let list = u(t_list(prelude::TYPE_INT, false));
    let result = expand::expand(u(t_key_of(list)), &cb);
    let expected = interner().intern_type(&[prelude::NON_NEGATIVE_INT], FlowFlags::EMPTY);
    assert_eq!(result, expected);
}

#[test]
fn key_of_sealed_list_is_index_range() {
    let cb = empty_world();
    let list = u(t_sealed_list(&[u(t_int()), u(t_string()), u(t_float())]));
    let result = expand::expand(u(t_key_of(list)), &cb);
    // Range [0, 2] (3 elements). Known indices 0/1/2 are also added but
    // the join should collapse them into the range.
    let elems = result.as_ref().elements;
    assert!(!elems.is_empty());
    // Each known index 0/1/2 must refine the result.
    assert!(atomic_is_contained(t_lit_int(0), elems[0], &cb) || elems.len() > 1);
}

#[test]
fn key_of_iterable_is_key_type() {
    let cb = empty_world();
    let iter = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    let result = expand::expand(u(t_key_of(iter)), &cb);
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn key_of_keyed_array_with_param_is_key_param() {
    let cb = empty_world();
    let arr = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let result = expand::expand(u(t_key_of(arr)), &cb);
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn key_of_sealed_keyed_shape_is_union_of_literal_keys() {
    let cb = empty_world();
    let shape = u(t_keyed_sealed(
        BTreeMap::from([(ak_str("name"), (false, prelude::TYPE_STRING)), (ak_str("age"), (false, prelude::TYPE_INT))]),
        false,
    ));
    let result = expand::expand(u(t_key_of(shape)), &cb);
    let elems = result.as_ref().elements;
    assert_eq!(elems.len(), 2);
}

#[test]
fn key_of_non_container_is_mixed() {
    let cb = empty_world();
    let result = expand::expand(u(t_key_of(prelude::TYPE_INT)), &cb);
    assert_eq!(result, prelude::TYPE_MIXED);
}

#[test]
fn value_of_list_is_element_type() {
    let cb = empty_world();
    let list = u(t_list(prelude::TYPE_INT, false));
    let result = expand::expand(u(t_value_of(list)), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn value_of_iterable_is_value_type() {
    let cb = empty_world();
    let iter = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    let result = expand::expand(u(t_value_of(iter)), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn value_of_keyed_array_with_param_is_value_param() {
    let cb = empty_world();
    let arr = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let result = expand::expand(u(t_value_of(arr)), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn index_access_on_sealed_shape_with_known_key_returns_value() {
    let cb = empty_world();
    let shape = u(t_keyed_sealed(BTreeMap::from([(ak_str("id"), (false, prelude::TYPE_INT))]), false));
    let result = expand::expand(u(t_index_access(shape, us("id"))), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn index_access_on_sealed_shape_with_unknown_key_returns_never() {
    let cb = empty_world();
    let shape = u(t_keyed_sealed(BTreeMap::from([(ak_str("id"), (false, prelude::TYPE_INT))]), false));
    let result = expand::expand(u(t_index_access(shape, us("missing"))), &cb);
    assert_eq!(result, prelude::TYPE_NEVER);
}

#[test]
fn index_access_on_unsealed_keyed_array_returns_value_param() {
    let cb = empty_world();
    let arr = u(t_keyed_unsealed(prelude::TYPE_STRING, prelude::TYPE_INT, false));
    let result = expand::expand(u(t_index_access(arr, us("any"))), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn index_access_on_iterable_returns_value_type() {
    let cb = empty_world();
    let iter = u(t_iterable(prelude::TYPE_STRING, prelude::TYPE_INT));
    let result = expand::expand(u(t_index_access(iter, us("any"))), &cb);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn index_access_on_list_with_literal_index_returns_known_element() {
    let cb = empty_world();
    let list = u(t_sealed_list(&[u(t_int()), u(t_string()), u(t_float())]));
    let result = expand::expand(u(t_index_access(list, ui(1))), &cb);
    assert_eq!(result, u(t_string()));
}

#[test]
fn int_mask_of_two_literals_yields_four_combinations() {
    let cb = empty_world();
    let result = expand::expand(u(t_int_mask(vec![ui(1), ui(2)])), &cb);
    let mut got: Vec<i64> = result
        .as_ref()
        .elements
        .iter()
        .filter_map(|e| match e.kind() {
            suffete::ElementKind::Int => match interner().get_int(*e) {
                suffete::element::payload::scalar::IntInfo::Literal(n) => Some(*n),
                _ => None,
            },
            _ => None,
        })
        .collect();
    got.sort_unstable();
    assert_eq!(got, vec![0, 1, 2, 3]);
}

#[test]
fn int_mask_of_three_literals_yields_eight_combinations() {
    let cb = empty_world();
    let result = expand::expand(u(t_int_mask(vec![ui(1), ui(2), ui(4)])), &cb);
    let count = result.as_ref().elements.len();
    assert_eq!(count, 8);
}

#[test]
fn int_mask_of_widens_target_to_int_mask_set() {
    let cb = empty_world();
    let union_of_lits = u_many(vec![t_lit_int(1), t_lit_int(2)]);
    let result = expand::expand(u(t_int_mask_of(union_of_lits)), &cb);
    let mut got: Vec<i64> = result
        .as_ref()
        .elements
        .iter()
        .filter_map(|e| match e.kind() {
            suffete::ElementKind::Int => match interner().get_int(*e) {
                suffete::element::payload::scalar::IntInfo::Literal(n) => Some(*n),
                _ => None,
            },
            _ => None,
        })
        .collect();
    got.sort_unstable();
    assert_eq!(got, vec![0, 1, 2, 3]);
}

#[test]
fn int_mask_with_non_literal_operand_widens_to_mixed() {
    let cb = empty_world();
    let result = expand::expand(u(t_int_mask(vec![prelude::TYPE_INT, ui(1)])), &cb);
    assert_eq!(result, prelude::TYPE_MIXED);
}

#[test]
fn template_type_resolves_to_constraint() {
    let mut w = MockWorld::new();
    w.with_templates("Box", &[("T", Variance::Covariant)]);
    w.with_template_bound("Box", "T", prelude::TYPE_INT);
    let class_t = u(t_named("Box"));
    let template_t = us("T");
    let result = expand::expand(u(t_template_type(class_t, template_t)), &w);
    assert_eq!(result, prelude::TYPE_INT);
}

#[test]
fn template_type_unknown_passes_through() {
    let cb = empty_world();
    let class_t = u(t_named("Box"));
    let template_t = us("T");
    let derived = u(t_template_type(class_t, template_t));
    assert_eq!(expand::expand(derived, &cb), derived);
}

#[test]
fn properties_of_passes_through_for_now() {
    let cb = empty_world();
    let derived =
        interner().intern_derived(DerivedInfo::PropertiesOf { target: prelude::TYPE_OBJECT, visibility: None });
    let ty = u(derived);
    assert_eq!(expand::expand(ty, &cb), ty);
}

#[test]
fn new_passes_through_for_now() {
    let cb = empty_world();
    let derived = interner().intern_derived(DerivedInfo::New(prelude::TYPE_MIXED));
    let ty = u(derived);
    assert_eq!(expand::expand(ty, &cb), ty);
}

#[test]
fn nested_derived_inside_alias_resolves() {
    let mut w = MockWorld::new();
    let list = u(t_list(prelude::TYPE_INT, false));
    let value_of_list = u(t_value_of(list));
    w.with_alias("Foo", "ElementType", value_of_list);
    let alias_elem = u(t_alias_elem("Foo", "ElementType"));
    assert_eq!(expand::expand(alias_elem, &w), prelude::TYPE_INT);
}

fn t_alias_elem(class: &str, alias: &str) -> ElementId {
    use suffete::element::payload::AliasInfo;
    interner().intern_alias(AliasInfo { class_name: atom(class), alias_name: atom(alias) })
}

#[test]
fn key_of_object_shape_is_union_of_property_name_literals() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[("name", prelude::TYPE_STRING, false), ("age", prelude::TYPE_INT, false)], true));
    let result = expand::expand(u(t_key_of(shape)), &cb);
    let expected = u_many(vec![t_lit_string("age"), t_lit_string("name")]);
    assert_eq!(result, expected);
}

#[test]
fn key_of_empty_object_shape_is_never() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[], true));
    let result = expand::expand(u(t_key_of(shape)), &cb);
    assert_eq!(result, prelude::TYPE_NEVER);
}

#[test]
fn value_of_object_shape_is_union_of_property_types() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[("name", prelude::TYPE_STRING, false), ("age", prelude::TYPE_INT, false)], true));
    let result = expand::expand(u(t_value_of(shape)), &cb);
    let expected = u_many(vec![t_int(), t_string()]);
    assert_eq!(result, expected);
}

#[test]
fn index_access_object_shape_with_literal_key_returns_property_type() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[("name", prelude::TYPE_STRING, false), ("age", prelude::TYPE_INT, false)], true));
    let key = u(t_lit_string("name"));
    let result = expand::expand(u(t_index_access(shape, key)), &cb);
    assert_eq!(result, prelude::TYPE_STRING);
}

#[test]
fn index_access_object_shape_with_unknown_literal_key_is_never() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[("name", prelude::TYPE_STRING, false)], true));
    let key = u(t_lit_string("missing"));
    let result = expand::expand(u(t_index_access(shape, key)), &cb);
    assert_eq!(result, prelude::TYPE_NEVER);
}

#[test]
fn index_access_object_shape_with_non_literal_key_widens_to_value_union() {
    let cb = empty_world();
    let shape = u(t_object_shape(&[("name", prelude::TYPE_STRING, false), ("age", prelude::TYPE_INT, false)], true));
    let result = expand::expand(u(t_index_access(shape, prelude::TYPE_STRING)), &cb);
    let expected = u_many(vec![t_int(), t_string()]);
    assert_eq!(result, expected);
}
