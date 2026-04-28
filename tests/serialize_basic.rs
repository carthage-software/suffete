//! Structural serialization round-trip tests. The handle bits (slot
//! number) are not preserved across to_serializable / intern, but
//! `content_eq` always holds.

mod comparator_common;

use comparator_common::*;

use suffete::FlowFlags;
use suffete::TypeId;
use suffete::prelude;

fn assert_round_trip(ty: TypeId) {
    let serial = ty.to_serializable();
    let restored = serial.intern();
    assert!(
        ty.content_eq(restored),
        "content not preserved across round trip\n  original: {:?}\n  restored: {:?}",
        ty.as_ref(),
        restored.as_ref(),
    );
    assert_eq!(ty.flags(), restored.flags(), "flags not preserved");
    assert_eq!(ty.meta(), restored.meta(), "meta not preserved");
}

#[test]
fn primitives_round_trip() {
    assert_round_trip(prelude::TYPE_INT);
    assert_round_trip(prelude::TYPE_STRING);
    assert_round_trip(prelude::TYPE_FLOAT);
    assert_round_trip(prelude::TYPE_BOOL);
    assert_round_trip(prelude::TYPE_NULL);
    assert_round_trip(prelude::TYPE_VOID);
    assert_round_trip(prelude::TYPE_NEVER);
    assert_round_trip(prelude::TYPE_MIXED);
    assert_round_trip(prelude::TYPE_ARRAY_KEY);
    assert_round_trip(prelude::TYPE_NUMERIC);
    assert_round_trip(prelude::TYPE_SCALAR);
    assert_round_trip(prelude::TYPE_OBJECT);
}

#[test]
fn int_literal_round_trips() {
    assert_round_trip(u(t_lit_int(42)));
    assert_round_trip(u(t_lit_int(-1)));
    assert_round_trip(u(t_lit_int(0)));
}

#[test]
fn int_range_round_trips() {
    assert_round_trip(u(t_int_range(0, 10)));
    assert_round_trip(u(t_int_from(5)));
    assert_round_trip(u(t_int_to(100)));
}

#[test]
fn float_literal_round_trips() {
    assert_round_trip(u(t_lit_float(2.5)));
}

#[test]
fn string_literal_and_refinements_round_trip() {
    assert_round_trip(u(t_lit_string("hello")));
    assert_round_trip(u(t_non_empty_string()));
    assert_round_trip(u(t_numeric_string()));
    assert_round_trip(u(t_lower_string()));
}

#[test]
fn unions_round_trip() {
    let int_or_string = u_many(vec![t_int(), t_string()]);
    assert_round_trip(int_or_string);
}

#[test]
fn flags_round_trip() {
    let with_by_ref = prelude::TYPE_INT.with_flags(FlowFlags::EMPTY.with_by_reference(true));
    assert_round_trip(with_by_ref);
}

#[test]
fn meta_round_trips() {
    let with_meta = prelude::TYPE_STRING.with_meta(7);
    assert_round_trip(with_meta);
}

#[test]
fn named_object_round_trips() {
    let foo = u(t_named("Foo"));
    assert_round_trip(foo);
}

#[test]
fn generic_object_round_trips() {
    let box_int = u(t_generic_named("Box", vec![u(t_int())]));
    assert_round_trip(box_int);
}

#[test]
fn list_round_trips() {
    let list_int = u(t_list(u(t_int()), false));
    assert_round_trip(list_int);
}

#[test]
fn keyed_array_round_trips() {
    let arr = u(t_keyed_unsealed(u(t_string()), u(t_int()), false));
    assert_round_trip(arr);
}

#[test]
fn iterable_round_trips() {
    let iter = u(t_iterable(u(t_string()), u(t_int())));
    assert_round_trip(iter);
}

#[test]
fn callable_signature_round_trips() {
    let sig = u(t_callable(&[u(t_int()), u(t_string())], u(t_bool())));
    assert_round_trip(sig);
}

#[test]
fn nested_unions_round_trip() {
    let inner = u_many(vec![t_int(), t_string()]);
    let outer = u(t_list(inner, false));
    assert_round_trip(outer);
}

#[test]
fn restored_within_same_process_uses_same_slot() {
    let ty = u_many(vec![t_int(), t_string()]);
    let restored = ty.to_serializable().intern();
    assert_eq!(ty, restored, "same process should re-intern to identical handle");
}

#[test]
fn element_id_round_trips_via_serializable() {
    let foo = t_named("Foo");
    let restored = foo.to_serializable().intern();
    assert_eq!(foo, restored, "element re-interned in the same process should match");
}

#[test]
fn element_id_with_generic_args_round_trips() {
    let box_int = t_generic_named("Box", vec![u(t_int())]);
    let restored = box_int.to_serializable().intern();
    assert_eq!(box_int, restored);
}

#[cfg(feature = "serde")]
#[test]
fn serde_round_trip_via_json() {
    let ty = u_many(vec![t_int(), t_string()]);
    let json = serde_json::to_string(&ty).expect("serialize");
    let restored: TypeId = serde_json::from_str(&json).expect("deserialize");
    assert!(ty.content_eq(restored));
}

#[cfg(feature = "serde")]
#[test]
fn serde_element_id_round_trip_via_json() {
    let elem = t_named("Bar");
    let json = serde_json::to_string(&elem).expect("serialize");
    let restored: suffete::ElementId = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(elem, restored);
}
