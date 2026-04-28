use std::mem::size_of;

use suffete::ElementId;
use suffete::ElementKind;
use suffete::TypeId;
use suffete::prelude;

#[test]
fn ids_are_niche_optimized() {
    assert_eq!(size_of::<ElementId>(), 4);
    assert_eq!(size_of::<Option<ElementId>>(), 4);
    assert_eq!(size_of::<TypeId>(), 8);
    assert_eq!(size_of::<Option<TypeId>>(), 8);
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
        prelude::NULL,
        prelude::NEVER,
        prelude::VOID,
        prelude::PLACEHOLDER,
        prelude::TRUE,
        prelude::FALSE,
        prelude::SCALAR,
        prelude::NUMERIC,
        prelude::ARRAY_KEY,
    ] {
        assert!(id.kind().is_trivial(), "expected trivial kind, got {:?}", id.kind());
        assert_eq!(id.slot(), 0, "trivial elements occupy slot 0");
    }
}

#[test]
fn well_known_int_family_slots_are_ordered() {
    assert_eq!(prelude::INT.slot(), 0);
    assert_eq!(prelude::POSITIVE_INT.slot(), 1);
    assert_eq!(prelude::INT_ZERO.slot(), 6);
    assert_eq!(prelude::INT_MINUS_ONE.slot(), 8);
    assert_eq!(prelude::INT.kind(), ElementKind::Int);
}

#[test]
fn type_id_carries_flags_and_meta_independently_of_arena_slot() {
    use suffete::FlowFlags;

    let base = prelude::TYPE_INT;
    let with_by_ref = base.with_flags(FlowFlags::EMPTY.with_by_reference(true));
    let with_meta_7 = base.with_meta(7);
    let with_both = with_by_ref.with_meta(7);

    // All four share the same arena slot.
    assert!(base.content_eq(with_by_ref));
    assert!(base.content_eq(with_meta_7));
    assert!(base.content_eq(with_both));

    // But they are distinct TypeIds.
    assert_ne!(base, with_by_ref);
    assert_ne!(base, with_meta_7);
    assert_ne!(with_by_ref, with_meta_7);

    // Accessors round-trip.
    assert!(with_by_ref.flags().by_reference());
    assert_eq!(with_meta_7.meta(), 7);
    assert!(with_both.flags().by_reference());
    assert_eq!(with_both.meta(), 7);
}
