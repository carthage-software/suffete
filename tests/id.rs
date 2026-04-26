use std::mem::size_of;

use suffete::ElementId;
use suffete::ElementKind;
use suffete::TypeId;
use suffete::well_known;

#[test]
fn ids_are_four_bytes_with_niche_optimization() {
    assert_eq!(size_of::<ElementId>(), 4);
    assert_eq!(size_of::<Option<ElementId>>(), 4);
    assert_eq!(size_of::<TypeId>(), 4);
    assert_eq!(size_of::<Option<TypeId>>(), 4);
}

#[test]
fn element_id_roundtrips_kind_and_slot() {
    let cases = [
        (ElementKind::Int, 0),
        (ElementKind::Int, 42),
        (ElementKind::String, 1024),
        (ElementKind::Object, ElementId::MAX_SLOT),
    ];

    for (kind, slot) in cases {
        let id = ElementId::new(kind, slot);
        assert_eq!(id.kind(), kind, "kind mismatch for ({kind:?}, {slot})");
        assert_eq!(id.slot(), slot, "slot mismatch for ({kind:?}, {slot})");
    }
}

#[test]
fn well_known_trivial_elements_have_trivial_kind() {
    for id in [
        well_known::NULL,
        well_known::NEVER,
        well_known::VOID,
        well_known::PLACEHOLDER,
        well_known::TRUE,
        well_known::FALSE,
        well_known::SCALAR,
        well_known::NUMERIC,
        well_known::ARRAY_KEY,
    ] {
        assert!(id.kind().is_trivial(), "expected trivial kind, got {:?}", id.kind());
        assert_eq!(id.slot(), 0, "trivial elements occupy slot 0");
    }
}

#[test]
fn well_known_int_family_slots_are_ordered() {
    assert_eq!(well_known::INT.slot(), 0);
    assert_eq!(well_known::POSITIVE_INT.slot(), 1);
    assert_eq!(well_known::INT_ZERO.slot(), 6);
    assert_eq!(well_known::INT_MINUS_ONE.slot(), 8);
    assert_eq!(well_known::INT.kind(), ElementKind::Int);
}
