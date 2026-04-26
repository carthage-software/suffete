#![allow(dead_code)]

//! Test helpers mirroring `mago/crates/codex/tests/combiner_common/mod.rs`,
//! translated to suffete's API. Test files in `tests/combiner_*.rs` consume
//! these helpers so the porting from mago is mechanical.
//!
//! Translation:
//!
//! | mago                         | suffete                                       |
//! |------------------------------|-----------------------------------------------|
//! | `combine(atomics, c, opts)`  | `suffete::combiner::combine(&[..])`           |
//! | `Vec<TAtomic>`               | `Vec<ElementId>`                              |
//! | `t_lit_int(n)`               | `ElementId::int_literal(n)`                   |
//! | `t_int()`                    | `well_known::INT`                             |
//! | `assert_combines_to(in, ex)` | sort-and-compare both sides as `&[ElementId]` |

use suffete::ElementId;
use suffete::combiner;
use suffete::well_known;

/// Combine elements via the structural canonicalization pass. Mirrors mago's
/// `combine(Vec<TAtomic>) -> Vec<TAtomic>` signature.
pub fn combine_default(elements: Vec<ElementId>) -> Vec<ElementId> {
    combiner::combine(&elements)
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
    well_known::NEVER
}

pub fn null() -> ElementId {
    well_known::NULL
}

pub fn void() -> ElementId {
    well_known::VOID
}

pub fn placeholder() -> ElementId {
    well_known::PLACEHOLDER
}

pub fn mixed() -> ElementId {
    well_known::MIXED
}

pub fn mixed_truthy() -> ElementId {
    well_known::TRUTHY_MIXED
}

pub fn mixed_falsy() -> ElementId {
    well_known::FALSY_MIXED
}

pub fn mixed_nonnull() -> ElementId {
    well_known::NON_NULL_MIXED
}

pub fn t_true() -> ElementId {
    well_known::TRUE
}

pub fn t_false() -> ElementId {
    well_known::FALSE
}

pub fn t_bool() -> ElementId {
    well_known::BOOL
}

pub fn t_int() -> ElementId {
    well_known::INT
}

pub fn t_string() -> ElementId {
    well_known::STRING
}

pub fn t_empty_array() -> ElementId {
    well_known::EMPTY_ARRAY
}

pub fn t_object_any() -> ElementId {
    well_known::OBJECT
}

pub fn t_named(name: &str) -> ElementId {
    ElementId::object_named(name)
}

pub fn t_lit_int(value: i64) -> ElementId {
    ElementId::int_literal(value)
}

pub fn t_int_unspec_lit() -> ElementId {
    well_known::LITERAL_INT
}

pub fn t_lit_string(value: &str) -> ElementId {
    ElementId::string_literal(value)
}

pub fn t_unspec_lit_string(non_empty: bool) -> ElementId {
    if non_empty { well_known::NON_EMPTY_LITERAL_STRING } else { well_known::LITERAL_STRING }
}

pub fn t_empty_string() -> ElementId {
    well_known::EMPTY_STRING
}

pub fn t_lit_float(value: f64) -> ElementId {
    ElementId::float_literal(value)
}

pub fn t_unspec_lit_float() -> ElementId {
    well_known::LITERAL_FLOAT
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
    well_known::RESOURCE
}

pub fn t_open_resource() -> ElementId {
    well_known::OPEN_RESOURCE
}

pub fn t_closed_resource() -> ElementId {
    well_known::CLOSED_RESOURCE
}

pub fn t_positive_int() -> ElementId {
    well_known::POSITIVE_INT
}

pub fn t_negative_int() -> ElementId {
    well_known::NEGATIVE_INT
}

pub fn t_non_negative_int() -> ElementId {
    well_known::NON_NEGATIVE_INT
}

pub fn t_non_positive_int() -> ElementId {
    well_known::NON_POSITIVE_INT
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
    well_known::FLOAT
}

pub fn t_array_key() -> ElementId {
    well_known::ARRAY_KEY
}

pub fn t_numeric() -> ElementId {
    well_known::NUMERIC
}

pub fn t_scalar() -> ElementId {
    well_known::SCALAR
}

pub fn t_class_string() -> ElementId {
    well_known::CLASS_STRING
}

pub fn t_interface_string() -> ElementId {
    well_known::INTERFACE_STRING
}

pub fn t_enum_string() -> ElementId {
    well_known::ENUM_STRING
}

pub fn t_trait_string() -> ElementId {
    well_known::TRAIT_STRING
}

pub fn t_non_empty_string() -> ElementId {
    well_known::NON_EMPTY_STRING
}

pub fn t_numeric_string() -> ElementId {
    well_known::NUMERIC_STRING
}

pub fn t_lower_string() -> ElementId {
    well_known::LOWERCASE_STRING
}

pub fn t_upper_string() -> ElementId {
    well_known::UPPERCASE_STRING
}

pub fn t_truthy_string() -> ElementId {
    well_known::TRUTHY_STRING
}

pub fn t_callable_string() -> ElementId {
    well_known::CALLABLE_STRING
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
