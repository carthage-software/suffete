use suffete::Element;
use suffete::well_known;

#[test]
fn trivial_well_knowns_dispatch_to_unit_variants() {
    assert!(matches!(well_known::NULL.view(), Element::Null));
    assert!(matches!(well_known::NEVER.view(), Element::Never));
    assert!(matches!(well_known::VOID.view(), Element::Void));
    assert!(matches!(well_known::PLACEHOLDER.view(), Element::Placeholder));
    assert!(matches!(well_known::BOOL.view(), Element::Bool));
    assert!(matches!(well_known::TRUE.view(), Element::True));
    assert!(matches!(well_known::FALSE.view(), Element::False));
    assert!(matches!(well_known::SCALAR.view(), Element::Scalar));
    assert!(matches!(well_known::NUMERIC.view(), Element::Numeric));
    assert!(matches!(well_known::ARRAY_KEY.view(), Element::ArrayKey));
    assert!(matches!(well_known::OBJECT.view(), Element::ObjectAny));
}

#[test]
fn int_well_knowns_resolve_to_int_variant_with_payload() {
    use suffete::element::payload::scalar::IntInfo;

    assert!(matches!(well_known::INT.view(), Element::Int(info) if matches!(info, IntInfo::Unspecified)));
    assert!(matches!(well_known::INT_ZERO.view(), Element::Int(info) if matches!(info, IntInfo::Literal(0))));
    assert!(matches!(well_known::INT_ONE.view(), Element::Int(info) if matches!(info, IntInfo::Literal(1))));
}

#[test]
fn resource_well_knowns_resolve_to_resource_variant() {
    use suffete::element::payload::ResourceInfo;

    assert!(matches!(well_known::RESOURCE.view(), Element::Resource(ResourceInfo::Any)));
    assert!(matches!(well_known::OPEN_RESOURCE.view(), Element::Resource(ResourceInfo::Open)));
    assert!(matches!(well_known::CLOSED_RESOURCE.view(), Element::Resource(ResourceInfo::Closed)));
}

#[test]
fn iterable_well_known_carries_mixed_key_value_typeids() {
    use suffete::Element;

    let Element::Iterable(info) = well_known::ITERABLE_MIXED_MIXED.view() else {
        panic!("expected Iterable variant");
    };
    assert_eq!(info.key_type, well_known::TYPE_MIXED);
    assert_eq!(info.value_type, well_known::TYPE_MIXED);
}

#[test]
fn array_well_knowns_resolve_to_array_variant() {
    let Element::Array(empty) = well_known::EMPTY_ARRAY.view() else {
        panic!("expected Array variant");
    };
    assert!(empty.is_sealed());

    let Element::Array(any) = well_known::ARRAY_KEY_MIXED.view() else {
        panic!("expected Array variant");
    };
    assert_eq!(any.key_param, Some(well_known::TYPE_ARRAY_KEY));
    assert_eq!(any.value_param, Some(well_known::TYPE_MIXED));
}

#[test]
fn callable_well_known_resolves_to_any_variant() {
    use suffete::element::payload::CallableInfo;

    assert!(matches!(well_known::CALLABLE.view(), Element::Callable(CallableInfo::Any)));
}
