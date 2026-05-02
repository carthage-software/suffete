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

use suffete::Element;
use suffete::prelude;

#[test]
fn trivial_well_knowns_dispatch_to_unit_variants() {
    assert!(matches!(prelude::NULL.view(), Element::Null));
    assert!(matches!(prelude::NEVER.view(), Element::Never));
    assert!(matches!(prelude::VOID.view(), Element::Void));
    assert!(matches!(prelude::PLACEHOLDER.view(), Element::Placeholder));
    assert!(matches!(prelude::BOOL.view(), Element::Bool));
    assert!(matches!(prelude::TRUE.view(), Element::True));
    assert!(matches!(prelude::FALSE.view(), Element::False));
    assert!(matches!(prelude::SCALAR.view(), Element::Scalar));
    assert!(matches!(prelude::NUMERIC.view(), Element::Numeric));
    assert!(matches!(prelude::ARRAY_KEY.view(), Element::ArrayKey));
    assert!(matches!(prelude::OBJECT.view(), Element::ObjectAny));
}

#[test]
fn int_well_knowns_resolve_to_int_variant_with_payload() {
    use suffete::element::payload::scalar::IntInfo;

    assert!(matches!(prelude::INT.view(), Element::Int(info) if matches!(info, IntInfo::Unspecified)));
    assert!(matches!(prelude::INT_ZERO.view(), Element::Int(info) if matches!(info, IntInfo::Literal(0))));
    assert!(matches!(prelude::INT_ONE.view(), Element::Int(info) if matches!(info, IntInfo::Literal(1))));
}

#[test]
fn resource_well_knowns_resolve_to_resource_variant() {
    use suffete::element::payload::ResourceInfo;

    assert!(matches!(prelude::RESOURCE.view(), Element::Resource(ResourceInfo::Any)));
    assert!(matches!(prelude::OPEN_RESOURCE.view(), Element::Resource(ResourceInfo::Open)));
    assert!(matches!(prelude::CLOSED_RESOURCE.view(), Element::Resource(ResourceInfo::Closed)));
}

#[test]
fn iterable_well_known_carries_mixed_key_value_typeids() {
    use suffete::Element;

    let Element::Iterable(info) = prelude::ITERABLE_MIXED_MIXED.view() else {
        panic!("expected Iterable variant");
    };
    assert_eq!(info.key_type, prelude::TYPE_MIXED);
    assert_eq!(info.value_type, prelude::TYPE_MIXED);
}

#[test]
fn array_well_knowns_resolve_to_array_variant() {
    let Element::Array(empty) = prelude::EMPTY_ARRAY.view() else {
        panic!("expected Array variant");
    };
    assert!(empty.is_sealed());

    let Element::Array(any) = prelude::ARRAY_KEY_MIXED.view() else {
        panic!("expected Array variant");
    };
    assert_eq!(any.key_param, Some(prelude::TYPE_ARRAY_KEY));
    assert_eq!(any.value_param, Some(prelude::TYPE_MIXED));
}

#[test]
fn callable_well_known_resolves_to_any_variant() {
    use suffete::element::payload::CallableInfo;

    assert!(matches!(prelude::CALLABLE.view(), Element::Callable(CallableInfo::Any)));
}
