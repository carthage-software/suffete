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

use mago_atom::atom;
use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::element::payload::ArrayKey;
use suffete::element::payload::KnownItemEntry;
use suffete::interner::interner;
use suffete::join;
use suffete::prelude;

/// Combine elements with the test-suite default options: structural
/// canonicalization plus subtype-driven absorption, integer-range
/// merging, array-shape merging, and threshold-based literal collapse
/// at the standard 128 / 128 / 128 thresholds.
pub fn combine_default(elements: Vec<ElementId>) -> Vec<ElementId> {
    let opts = join::JoinOptions::default()
        .with_absorb_refinements(true)
        .with_merge_int_ranges(true)
        .with_merge_array_shapes(true)
        .with_int_literal_collapse_threshold(128)
        .with_string_literal_collapse_threshold(128)
        .with_float_literal_collapse_threshold(128)
        .with_array_shape_collapse_threshold(128);
    join::compute_with(&elements, &opts)
}

/// Combine with a custom integer literal collapse threshold.
pub fn combine_with_int_threshold(elements: Vec<ElementId>, threshold: u16) -> Vec<ElementId> {
    let opts = join::JoinOptions::default().with_int_literal_collapse_threshold(threshold);
    join::compute_with(&elements, &opts)
}

/// Combine with a custom string literal collapse threshold.
pub fn combine_with_string_threshold(elements: Vec<ElementId>, threshold: u16) -> Vec<ElementId> {
    let opts = join::JoinOptions::default().with_string_literal_collapse_threshold(threshold);
    join::compute_with(&elements, &opts)
}

/// Combine with a custom float literal collapse threshold.
pub fn combine_with_float_threshold(elements: Vec<ElementId>, threshold: u16) -> Vec<ElementId> {
    let opts = join::JoinOptions::default().with_float_literal_collapse_threshold(threshold);
    join::compute_with(&elements, &opts)
}

/// Combine with a custom array shape collapse threshold.
pub fn combine_with_array_threshold(elements: Vec<ElementId>, threshold: u16) -> Vec<ElementId> {
    let opts = join::JoinOptions::default().with_array_shape_collapse_threshold(threshold);
    join::compute_with(&elements, &opts)
}

/// Combine with the `overwrite_empty_array` option enabled.
pub fn combine_overwrite(elements: Vec<ElementId>) -> Vec<ElementId> {
    let opts = join::JoinOptions::default().with_overwrite_empty_array(true);
    join::compute_with(&elements, &opts)
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

pub fn t_list(element: TypeId, non_empty: bool) -> ElementId {
    ElementId::list(element, non_empty)
}

pub fn t_sealed_list(elements: &[(TypeId, bool)], non_empty: bool) -> ElementId {
    use suffete::element::payload::KnownElementEntry;
    let entries: Vec<KnownElementEntry> = elements
        .iter()
        .enumerate()
        .map(|(idx, (value, optional))| KnownElementEntry { index: idx as u32, value: *value, optional: *optional })
        .collect();
    ElementId::sealed_list(&entries, non_empty)
}

pub fn t_keyed_unsealed(key: TypeId, value: TypeId, non_empty: bool) -> ElementId {
    ElementId::keyed_unsealed(key, value, non_empty)
}

pub fn t_keyed_sealed(items: std::collections::BTreeMap<ArrayKey, (bool, TypeId)>, non_empty: bool) -> ElementId {
    let entries: Vec<KnownItemEntry> =
        items.into_iter().map(|(key, (optional, value))| KnownItemEntry { key, value, optional }).collect();
    ElementId::keyed_sealed(&entries, non_empty)
}

pub fn t_iterable(key: TypeId, value: TypeId) -> ElementId {
    ElementId::iterable(key, value)
}

pub fn t_generic_named(name: &str, args: Vec<TypeId>) -> ElementId {
    use suffete::element::payload::ObjectFlags;
    use suffete::element::payload::ObjectInfo;
    let i = interner();
    let info = ObjectInfo {
        name: atom(name),
        type_args: Some(i.intern_type_list(&args)),
        intersections: None,
        flags: ObjectFlags::default(),
    };
    i.intern_object(info)
}

pub fn t_callable_mixed() -> ElementId {
    ElementId::callable_mixed()
}

pub fn t_closure_mixed() -> ElementId {
    ElementId::closure_mixed()
}

pub fn t_callable_sig(params: &[(TypeId, bool, bool, bool)], return_type: TypeId, pure: bool) -> ElementId {
    use suffete::element::payload::CallableInfo;
    use suffete::element::payload::ParamFlags;
    use suffete::element::payload::ParamInfo;
    use suffete::element::payload::Signature;
    use suffete::element::payload::SignatureFlags;
    let i = interner();
    let info_params: Vec<ParamInfo> = params
        .iter()
        .enumerate()
        .map(|(idx, (ty, has_default, by_ref, variadic))| ParamInfo {
            name: atom(&format!("p{idx}")),
            type_: *ty,
            flags: ParamFlags::EMPTY.with_has_default(*has_default).with_by_reference(*by_ref).with_variadic(*variadic),
        })
        .collect();
    let trailing_variadic = info_params.last().is_some_and(|p| p.flags.variadic());
    let param_list = if info_params.is_empty() { None } else { Some(i.intern_param_list(&info_params)) };
    let sig = i.intern_signature(Signature {
        parameters: param_list,
        return_type,
        throws: None,
        flags: SignatureFlags::EMPTY.with_is_variadic(trailing_variadic).with_is_pure(pure),
    });
    i.intern_callable(CallableInfo::Signature(sig))
}

pub fn t_callable(params: &[TypeId], return_type: TypeId) -> ElementId {
    let p: Vec<(TypeId, bool, bool, bool)> = params.iter().map(|t| (*t, false, false, false)).collect();
    t_callable_sig(&p, return_type, false)
}

pub fn ak_int(n: i64) -> ArrayKey {
    ArrayKey::Int(n)
}

pub fn ak_str(s: &str) -> ArrayKey {
    ArrayKey::String(atom(s))
}

pub fn u(a: ElementId) -> TypeId {
    interner().intern_type(&[a], FlowFlags::EMPTY)
}

pub fn u_many(atoms: Vec<ElementId>) -> TypeId {
    interner().intern_type(&atoms, FlowFlags::EMPTY)
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
