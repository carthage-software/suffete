//! Cast semantics (report §20): per-target sanity for each input
//! element kind. First-cut accuracy — the API surface is the contract,
//! literal-precision improvements land later.

mod comparator_common;

use comparator_common::*;

use suffete::cast;
use suffete::cast::CastTarget;
use suffete::prelude;

#[test]
fn cast_int_literal_to_int_is_lossless_and_preserves() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_int(42)), CastTarget::Int, &cb);
    assert!(!r.flags.lossy());
    assert!(!r.flags.may_throw());
    assert_eq!(r.ty, u(t_lit_int(42)));
}

#[test]
fn cast_float_to_int_is_lossy() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_float(3.7)), CastTarget::Int, &cb);
    assert!(r.flags.lossy());
    assert!(!r.flags.may_throw());
    assert_eq!(r.ty, u(t_lit_int(3)));
}

#[test]
fn cast_numeric_string_literal_to_int_preserves() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_string("42")), CastTarget::Int, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_int(42)));
}

#[test]
fn cast_non_numeric_string_to_int_is_lossy_zero() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_string("hello")), CastTarget::Int, &cb);
    assert!(r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_int(0)));
}

#[test]
fn cast_true_to_int_is_one() {
    let cb = empty_world();
    let r = cast::cast(u(t_true()), CastTarget::Int, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_int(1)));
}

#[test]
fn cast_null_to_int_is_zero() {
    let cb = empty_world();
    let r = cast::cast(u(null()), CastTarget::Int, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_int(0)));
}

#[test]
fn cast_object_to_int_may_throw() {
    let cb = empty_world();
    let r = cast::cast(u(t_object_any()), CastTarget::Int, &cb);
    assert!(r.flags.may_throw());
}

#[test]
fn cast_int_literal_to_string_preserves() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_int(42)), CastTarget::String, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_string("42")));
}

#[test]
fn cast_true_to_string_is_one_literal() {
    let cb = empty_world();
    let r = cast::cast(u(t_true()), CastTarget::String, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_string("1")));
}

#[test]
fn cast_false_to_string_is_empty_literal() {
    let cb = empty_world();
    let r = cast::cast(u(t_false()), CastTarget::String, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(prelude::EMPTY_STRING));
}

#[test]
fn cast_array_to_string_may_throw() {
    let cb = empty_world();
    let r = cast::cast(u(t_empty_array()), CastTarget::String, &cb);
    assert!(r.flags.may_throw());
}

#[test]
fn cast_falsy_literals_to_bool_collapse_to_false() {
    let cb = empty_world();
    for atom in [t_lit_int(0), t_lit_string(""), t_lit_string("0"), null()] {
        let r = cast::cast(u(atom), CastTarget::Bool, &cb);
        assert_eq!(r.ty, u(t_false()), "expected {atom:?} → false");
    }
}

#[test]
fn cast_truthy_literals_to_bool_collapse_to_true() {
    let cb = empty_world();
    for atom in [t_lit_int(1), t_lit_int(-1), t_lit_string("hello")] {
        let r = cast::cast(u(atom), CastTarget::Bool, &cb);
        assert_eq!(r.ty, u(t_true()), "expected {atom:?} → true");
    }
}

#[test]
fn cast_object_to_bool_is_always_true() {
    let cb = empty_world();
    let r = cast::cast(u(t_object_any()), CastTarget::Bool, &cb);
    assert_eq!(r.ty, u(t_true()));
    assert!(!r.flags.lossy());
}

#[test]
fn cast_general_int_to_bool_widens_to_bool() {
    let cb = empty_world();
    let r = cast::cast(u(t_int()), CastTarget::Bool, &cb);
    assert_eq!(r.ty, prelude::TYPE_BOOL);
}

#[test]
fn cast_int_to_float_is_lossless() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_int(42)), CastTarget::Float, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_lit_float(42.0)));
}

#[test]
fn cast_null_to_array_is_empty_array() {
    let cb = empty_world();
    let r = cast::cast(u(null()), CastTarget::Array, &cb);
    assert!(!r.flags.lossy());
    assert_eq!(r.ty, u(t_empty_array()));
}

#[test]
fn cast_int_to_array_is_lossy() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_int(1)), CastTarget::Array, &cb);
    assert!(r.flags.lossy());
}

#[test]
fn cast_object_to_object_is_lossless_passthrough() {
    let cb = empty_world();
    let foo = u(t_named("Foo"));
    let r = cast::cast(foo, CastTarget::Object, &cb);
    assert_eq!(r.ty, foo);
    assert!(!r.flags.lossy());
}

#[test]
fn cast_int_to_object_is_lossy_stdclass() {
    let cb = empty_world();
    let r = cast::cast(u(t_lit_int(1)), CastTarget::Object, &cb);
    assert!(r.flags.lossy());
    let elements = r.ty.as_ref().elements;
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].kind(), suffete::ElementKind::Object);
}

#[test]
fn cast_distributes_over_union_with_worst_classification() {
    let cb = empty_world();
    let input = u_many(vec![t_lit_int(42), t_object_any()]);
    let r = cast::cast(input, CastTarget::Int, &cb);
    assert!(r.flags.may_throw(), "object branch propagates may-throw flag");
}
