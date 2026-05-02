//! Shared helpers for the benchmark suite.
//!
//! Provides:
//! - [`bench_world()`]: a default `NullWorld` for benches that need a `&impl World`.
//! - Input builders for the standard "tiny / small / wide / deep" shapes
//!   used across the suite, so each benchmark tracks the same axes
//!   (singleton, small union, wide union, deeply nested) consistently.

#![allow(
    clippy::significant_drop_tightening,
    clippy::wildcard_imports,
    clippy::missing_const_for_fn,
    clippy::separated_literal_suffix,
    clippy::doc_markdown,
    clippy::redundant_closure,
    clippy::arithmetic_side_effects,
    dead_code,
    clippy::missing_docs_in_private_items,
    clippy::missing_inline_in_public_items,
    clippy::must_use_candidate,
    clippy::absolute_paths,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
use suffete::prelude;
use suffete::world::NullWorld;

/// World instance shared across benches that don't need a class hierarchy.
#[inline]
pub fn bench_world() -> NullWorld {
    NullWorld
}

// -- Atomic singleton handles --------------------------------------------------

#[inline]
pub fn t_int() -> TypeId {
    prelude::TYPE_INT
}

#[inline]
pub fn t_string() -> TypeId {
    prelude::TYPE_STRING
}

#[inline]
pub fn t_float() -> TypeId {
    prelude::TYPE_FLOAT
}

#[inline]
pub fn t_bool() -> TypeId {
    prelude::TYPE_BOOL
}

#[inline]
pub fn t_null() -> TypeId {
    prelude::TYPE_NULL
}

#[inline]
pub fn t_mixed() -> TypeId {
    prelude::TYPE_MIXED
}

#[inline]
pub fn t_never() -> TypeId {
    prelude::TYPE_NEVER
}

#[inline]
pub fn t_object() -> TypeId {
    prelude::TYPE_OBJECT
}

#[inline]
pub fn t_array_key() -> TypeId {
    prelude::TYPE_ARRAY_KEY
}

#[inline]
pub fn t_scalar() -> TypeId {
    prelude::TYPE_SCALAR
}

#[inline]
pub fn t_numeric() -> TypeId {
    prelude::TYPE_NUMERIC
}

// -- Element constructors ------------------------------------------------------

#[inline]
pub fn e_int_lit(value: i64) -> ElementId {
    ElementId::int_literal(value)
}

#[inline]
pub fn e_str_lit(value: &str) -> ElementId {
    ElementId::string_literal(value)
}

#[inline]
pub fn e_named(name: &str) -> ElementId {
    ElementId::object_named(name)
}

#[inline]
pub fn e_list(element_type: TypeId, non_empty: bool) -> ElementId {
    ElementId::list(element_type, non_empty)
}

#[inline]
pub fn e_keyed(key_type: TypeId, value_type: TypeId, non_empty: bool) -> ElementId {
    ElementId::keyed_unsealed(key_type, value_type, non_empty)
}

#[inline]
pub fn e_iterable(key_type: TypeId, value_type: TypeId) -> ElementId {
    ElementId::iterable(key_type, value_type)
}

// -- Type builders -------------------------------------------------------------

/// Singleton union wrapping `elem`.
#[inline]
pub fn ut(elem: ElementId) -> TypeId {
    interner().intern_type(&[elem], FlowFlags::EMPTY)
}

/// Multi-element union from a slice (no canonicalisation; mirrors `TypeId::union`).
#[inline]
pub fn um(elems: &[ElementId]) -> TypeId {
    TypeId::union(elems)
}

// -- Standard input shapes -----------------------------------------------------

/// `int(0)`. Singleton; cheapest non-trivial type.
#[inline]
pub fn tiny_singleton() -> TypeId {
    ut(e_int_lit(0))
}

/// `int|string|null`. 3-element union.
#[inline]
pub fn small_union() -> TypeId {
    um(&[ElementId::int_literal(0), ElementId::int_literal(1), ElementId::int_literal(2)])
}

/// 32 distinct int literals in one union.
#[inline]
pub fn wide_union() -> TypeId {
    let mut elems = Vec::with_capacity(32);
    for i in 0_i64..32 {
        elems.push(e_int_lit(i));
    }
    um(&elems)
}

/// `list<list<list<int>>>`. 3-deep nested generic list.
#[inline]
pub fn deep_nested() -> TypeId {
    let inner = ut(e_list(t_int(), false));
    let mid = ut(e_list(inner, false));
    ut(e_list(mid, false))
}

/// `array<string, list<int>>`. Common shape with payload.
#[inline]
pub fn keyed_array_payload() -> TypeId {
    let value = ut(e_list(t_int(), false));
    ut(e_keyed(t_string(), value, false))
}

/// `Foo` named object with no template args.
#[inline]
pub fn named_object() -> TypeId {
    ut(e_named("Foo"))
}

/// 32 distinct string-literal elements; exercises string-axis canonicalization in join.
#[inline]
pub fn wide_string_literals() -> Vec<ElementId> {
    let mut out = Vec::with_capacity(32);
    for idx in 0..32 {
        out.push(e_str_lit(&format!("s{idx}")));
    }
    out
}

/// 32 distinct int literals; exercises range-merge logic in join.
#[inline]
pub fn wide_int_literals() -> Vec<ElementId> {
    let mut out = Vec::with_capacity(32);
    for idx in 0_i64..32 {
        out.push(e_int_lit(idx));
    }
    out
}

/// Standard input set; pass each into a benchmark group with `bench_with_input`.
pub fn standard_inputs() -> Vec<(&'static str, TypeId)> {
    vec![
        ("tiny", tiny_singleton()),
        ("small_union", small_union()),
        ("wide_union", wide_union()),
        ("deep_nested", deep_nested()),
        ("keyed_array_payload", keyed_array_payload()),
        ("named_object", named_object()),
    ]
}
