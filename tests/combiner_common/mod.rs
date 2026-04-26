#![allow(dead_code)]

//! Test helpers mirroring `mago/crates/codex/tests/combiner_common/mod.rs`,
//! translated to suffete's API. Test files in `tests/combiner_*.rs` consume
//! these helpers so the porting from mago is mechanical.
//!
//! Translation:
//!
//! | mago                         | suffete                                       |
//! |------------------------------|-----------------------------------------------|
//! | `combine(atomics, c, opts)`  | `suffete::join::compute(&[..])`               |
//! | `Vec<TAtomic>`               | `Vec<ElementId>`                              |
//! | `t_lit_int(n)`               | `ElementId::int_literal(n)`                   |
//! | `t_int()`                    | `prelude::INT`                                |
//! | `assert_combines_to(in, ex)` | sort-and-compare both sides as `&[ElementId]` |

use suffete::ElementId;
use suffete::join;
use suffete::prelude;

/// Combine elements via the structural canonicalization pass. Mirrors mago's
/// `combine(Vec<TAtomic>) -> Vec<TAtomic>` signature.
pub fn combine_default(elements: Vec<ElementId>) -> Vec<ElementId> {
    join::compute(&elements)
}

/// Combine with a custom integer literal collapse threshold.
///
/// Suffete does not yet implement threshold-based literal collapse; the
/// threshold argument is accepted for porting symmetry but ignored. Tests
/// that depend on the threshold actually triggering must be `#[ignore]`'d
/// until the feature lands.
pub fn combine_with_int_threshold(elements: Vec<ElementId>, _threshold: u16) -> Vec<ElementId> {
    combine_default(elements)
}

/// Combine with a custom string literal collapse threshold. See
/// [`combine_with_int_threshold`] for the threshold caveat.
pub fn combine_with_string_threshold(elements: Vec<ElementId>, _threshold: u16) -> Vec<ElementId> {
    combine_default(elements)
}

/// Combine with a custom array shape collapse threshold. See
/// [`combine_with_int_threshold`] for the threshold caveat.
pub fn combine_with_array_threshold(elements: Vec<ElementId>, _threshold: u16) -> Vec<ElementId> {
    combine_default(elements)
}

/// Combine with the `overwrite_empty_array` option enabled. Suffete does
/// not yet model this combiner option; tests depending on it must be
/// `#[ignore]`'d.
pub fn combine_overwrite(elements: Vec<ElementId>) -> Vec<ElementId> {
    combine_default(elements)
}

/// Assert that combining `input` produces exactly one element matching
/// `predicate`.
pub fn assert_single<F: Fn(&ElementId) -> bool>(input: Vec<ElementId>, predicate: F) {
    let result = combine_default(input);
    assert_eq!(result.len(), 1, "expected single element, got: {result:?}");
    assert!(predicate(&result[0]), "predicate failed for: {:?}", result[0]);
}

/// Assert that two element vectors are multiset-equal (order-insensitive).
pub fn assert_multiset_eq(actual: &[ElementId], expected: &[ElementId]) {
    let mut a: Vec<ElementId> = actual.to_vec();
    let mut e: Vec<ElementId> = expected.to_vec();
    a.sort();
    e.sort();
    assert_eq!(a, e, "\n  actual:   {actual:?}\n  expected: {expected:?}");
}

/// Convenience: re-export of `mago_atom::atom` for tests that need a raw atom.
pub fn name_atom(s: &str) -> mago_atom::Atom {
    mago_atom::atom(s)
}

/// Assert that combining `input` produces a multiset equal to `expected`.
/// Order is implementation-defined (interning sorts by `ElementId` packed
/// value), so we sort both sides before comparison.
pub fn assert_combines_to(input: Vec<ElementId>, expected: Vec<ElementId>) {
    let mut actual = combine_default(input);
    let mut expected = expected;
    actual.sort();
    expected.sort();
    assert_eq!(actual, expected, "\n  actual:   {actual:?}\n  expected: {expected:?}");
}

pub fn never() -> ElementId {
    prelude::NEVER
}

pub fn null() -> ElementId {
    prelude::NULL
}

pub fn void() -> ElementId {
    prelude::VOID
}

pub fn placeholder() -> ElementId {
    prelude::PLACEHOLDER
}

pub fn mixed() -> ElementId {
    prelude::MIXED
}

pub fn mixed_truthy() -> ElementId {
    prelude::TRUTHY_MIXED
}

pub fn mixed_falsy() -> ElementId {
    prelude::FALSY_MIXED
}

pub fn mixed_nonnull() -> ElementId {
    prelude::NON_NULL_MIXED
}

pub fn t_true() -> ElementId {
    prelude::TRUE
}

pub fn t_false() -> ElementId {
    prelude::FALSE
}

pub fn t_bool() -> ElementId {
    prelude::BOOL
}

pub fn t_int() -> ElementId {
    prelude::INT
}

pub fn t_string() -> ElementId {
    prelude::STRING
}

pub fn t_empty_array() -> ElementId {
    prelude::EMPTY_ARRAY
}

pub fn t_object_any() -> ElementId {
    prelude::OBJECT
}

pub fn t_named(name: &str) -> ElementId {
    ElementId::object_named(name)
}

pub fn t_lit_int(value: i64) -> ElementId {
    ElementId::int_literal(value)
}

pub fn t_int_unspec_lit() -> ElementId {
    prelude::LITERAL_INT
}

pub fn t_lit_string(value: &str) -> ElementId {
    ElementId::string_literal(value)
}

pub fn t_unspec_lit_string(non_empty: bool) -> ElementId {
    if non_empty { prelude::NON_EMPTY_LITERAL_STRING } else { prelude::LITERAL_STRING }
}

pub fn t_empty_string() -> ElementId {
    prelude::EMPTY_STRING
}

pub fn t_lit_float(value: f64) -> ElementId {
    ElementId::float_literal(value)
}

pub fn t_unspec_lit_float() -> ElementId {
    prelude::LITERAL_FLOAT
}

pub fn t_enum(name: &str) -> ElementId {
    ElementId::enum_any(name)
}

pub fn t_enum_case(name: &str, case: &str) -> ElementId {
    ElementId::enum_case(name, case)
}

pub fn t_lit_class_string(name: &str) -> ElementId {
    ElementId::class_string_literal(name)
}

pub fn t_resource() -> ElementId {
    prelude::RESOURCE
}

pub fn t_open_resource() -> ElementId {
    prelude::OPEN_RESOURCE
}

pub fn t_closed_resource() -> ElementId {
    prelude::CLOSED_RESOURCE
}

pub fn t_positive_int() -> ElementId {
    prelude::POSITIVE_INT
}

pub fn t_negative_int() -> ElementId {
    prelude::NEGATIVE_INT
}

pub fn t_non_negative_int() -> ElementId {
    prelude::NON_NEGATIVE_INT
}

pub fn t_non_positive_int() -> ElementId {
    prelude::NON_POSITIVE_INT
}

pub fn t_int_range(lo: i64, hi: i64) -> ElementId {
    ElementId::int_range(Some(lo), Some(hi))
}

pub fn t_int_from(from: i64) -> ElementId {
    ElementId::int_range(Some(from), None)
}

pub fn t_int_to(to: i64) -> ElementId {
    ElementId::int_range(None, Some(to))
}

pub fn t_float() -> ElementId {
    prelude::FLOAT
}

pub fn t_array_key() -> ElementId {
    prelude::ARRAY_KEY
}

pub fn t_numeric() -> ElementId {
    prelude::NUMERIC
}

pub fn t_scalar() -> ElementId {
    prelude::SCALAR
}

pub fn t_class_string() -> ElementId {
    prelude::CLASS_STRING
}

pub fn t_interface_string() -> ElementId {
    prelude::INTERFACE_STRING
}

pub fn t_enum_string() -> ElementId {
    prelude::ENUM_STRING
}

pub fn t_trait_string() -> ElementId {
    prelude::TRAIT_STRING
}

pub fn t_non_empty_string() -> ElementId {
    prelude::NON_EMPTY_STRING
}

pub fn t_numeric_string() -> ElementId {
    prelude::NUMERIC_STRING
}

pub fn t_lower_string() -> ElementId {
    prelude::LOWERCASE_STRING
}

pub fn t_upper_string() -> ElementId {
    prelude::UPPERCASE_STRING
}

pub fn t_truthy_string() -> ElementId {
    prelude::TRUTHY_STRING
}

pub fn t_callable_string() -> ElementId {
    prelude::CALLABLE_STRING
}

/// Combine `n` copies of an element and assert the result is exactly `[a]`
/// (idempotency under self-combination).
pub fn assert_self_idempotent(a: ElementId, n: usize) {
    let input: Vec<ElementId> = std::iter::repeat_n(a, n).collect();
    let out = combine_default(input);
    assert_eq!(out.len(), 1, "self-combination should produce 1 element for {a:?}, got {out:?}");
    assert_eq!(out[0], a, "self-combination should preserve identity for {a:?}");
}

/// Assert that `combine([a, b]) == combine([b, a])` (commutativity).
pub fn assert_commutative(a: ElementId, b: ElementId) {
    let mut ab = combine_default(vec![a, b]);
    let mut ba = combine_default(vec![b, a]);
    ab.sort();
    ba.sort();
    assert_eq!(ab, ba, "combine is not commutative for {a:?} | {b:?}");
}
