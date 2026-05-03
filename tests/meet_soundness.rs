//! Soundness invariants for [`meet`]: the result must be a subset of
//! both inputs. Each test pins a regression for a soundness bug
//! previously found by audit (where `meet(a, b)` returned values
//! outside `a` or `b`).

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

mod comparator_common;

use comparator_common::*;
use mago_atom::atom;
use suffete::ElementId;
use suffete::ElementKind;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::element::payload::KnownElementEntry;
use suffete::element::payload::ListFlags;
use suffete::element::payload::ListInfo;
use suffete::element::payload::MixedInfo;
use suffete::element::payload::Truthiness;
use suffete::element::payload::scalar::StringCasing;
use suffete::element::payload::scalar::StringInfo;
use suffete::element::payload::scalar::StringLiteral;
use suffete::element::payload::scalar::StringRefinementFlags;
use suffete::interner::interner;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::refines;
use suffete::meet;
use suffete::prelude;
use suffete::world::World;

fn lattice_meet<W: World>(a: TypeId, b: TypeId, w: &W) -> TypeId {
    meet::compute(a, b, w, LatticeOptions::default(), &mut LatticeReport::new())
}

fn does_refine<W: World>(a: TypeId, b: TypeId, w: &W) -> bool {
    refines(a, b, w, LatticeOptions::default(), &mut LatticeReport::new())
}

/// `meet(a, b) <: a` AND `meet(a, b) <: b`. The defining axiom of
/// meet ; any violation is a soundness bug.
#[track_caller]
fn assert_meet_is_subset(a: TypeId, b: TypeId) {
    let w = empty_world();
    let m = lattice_meet(a, b, &w);
    assert!(does_refine(m, a, &w), "meet({a}, {b}) = {m} must refine {a}");
    assert!(does_refine(m, b, &w), "meet({a}, {b}) = {m} must refine {b}");
}

#[test]
fn truthy_mixed_meet_literal_zero_string_is_never() {
    let lhs = u(mixed_truthy());
    let rhs = u(t_lit_string("0"));
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "truthy ∩ string('0') must be never (got {m})");
}

#[test]
fn truthy_mixed_meet_literal_empty_string_is_never() {
    let lhs = u(mixed_truthy());
    let rhs = u(t_lit_string(""));
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "truthy ∩ string('') must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_non_empty_string_excludes_empty_literal() {
    let i = interner();
    let non_empty = i.intern_string(StringInfo {
        literal: StringLiteral::None,
        casing: StringCasing::Unspecified,
        flags: StringRefinementFlags::EMPTY.with_is_non_empty(true),
    });
    let lhs = u(mixed_falsy());
    let rhs = u(non_empty);
    assert_meet_is_subset(lhs, rhs);
    let empty_lit = u(t_lit_string(""));
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert!(
        !does_refine(empty_lit, m, &empty_world()),
        "string('') is empty so cannot be in falsy ∩ non-empty-string (got meet {m})"
    );
}

#[test]
fn falsy_mixed_meet_truthy_string_is_never() {
    let i = interner();
    let truthy_string = i.intern_string(StringInfo {
        literal: StringLiteral::None,
        casing: StringCasing::Unspecified,
        flags: StringRefinementFlags::EMPTY.with_is_truthy(true).with_is_non_empty(true),
    });
    let lhs = u(mixed_falsy());
    let rhs = u(truthy_string);
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "falsy ∩ truthy-string must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_list_is_empty_list_only() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_list(prelude::TYPE_INT, false));
    assert_meet_is_subset(lhs, rhs);
    let non_empty = u(t_list(prelude::TYPE_INT, true));
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert!(
        !does_refine(non_empty, m, &empty_world()),
        "non-empty-list<int> values are truthy so cannot be in falsy ∩ list<int> (got meet {m})"
    );
}

#[test]
fn falsy_mixed_meet_non_empty_list_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_list(prelude::TYPE_INT, true));
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "non-empty-list is always truthy ; falsy meet must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_unsealed_array_is_empty_array_only() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_keyed_unsealed(prelude::TYPE_ARRAY_KEY, prelude::TYPE_MIXED, false));
    assert_meet_is_subset(lhs, rhs);
    let non_empty = u(t_keyed_unsealed(prelude::TYPE_ARRAY_KEY, prelude::TYPE_MIXED, true));
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert!(
        !does_refine(non_empty, m, &empty_world()),
        "non-empty arrays are truthy so cannot be in falsy ∩ array (got meet {m})"
    );
}

#[test]
fn falsy_mixed_meet_iterable_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_iterable(prelude::TYPE_ARRAY_KEY, prelude::TYPE_MIXED));
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "iterable is conservatively truthy ; falsy meet must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_class_like_string_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_class_string());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "class-like-string is always truthy ; falsy meet must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_callable_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_callable_any());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "callable is always truthy ; falsy meet must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_resource_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_resource());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "resource is always truthy ; falsy meet must be never (got {m})");
}

#[test]
fn falsy_mixed_meet_object_is_never() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_object_any());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "objects are always truthy ; falsy meet must be never (got {m})");
}

#[test]
fn mixed_meet_preserves_is_empty_flag() {
    let i = interner();
    let empty_mixed = i.intern_mixed(MixedInfo::EMPTY.with_is_empty(true));
    let non_null_mixed = i.intern_mixed(MixedInfo::EMPTY.with_is_non_null(true));
    let lhs = u(empty_mixed);
    let rhs = u(non_null_mixed);
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    let m_atom = m.as_ref().elements[0];
    assert_eq!(m_atom.kind(), ElementKind::Mixed);
    let m_info = *i.get_mixed(m_atom);
    assert!(m_info.is_empty(), "meet must preserve is_empty (got {m_info:?})");
    assert!(m_info.is_non_null(), "meet must preserve is_non_null (got {m_info:?})");
}

#[test]
fn mixed_meet_preserves_is_isset_from_loop_flag() {
    let i = interner();
    let isset_mixed = i.intern_mixed(MixedInfo::EMPTY.with_is_isset_from_loop(true));
    let non_null_mixed = i.intern_mixed(MixedInfo::EMPTY.with_is_non_null(true));
    let lhs = u(isset_mixed);
    let rhs = u(non_null_mixed);
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    let m_atom = m.as_ref().elements[0];
    let m_info = *i.get_mixed(m_atom);
    assert!(m_info.is_isset_from_loop(), "meet must preserve is_isset_from_loop");
    assert!(m_info.is_non_null(), "meet must preserve is_non_null");
}

#[test]
fn truthy_mixed_meet_truthy_mixed_is_truthy() {
    let i = interner();
    let lhs = u(mixed_truthy());
    let rhs = u(mixed_truthy());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    let m_atom = m.as_ref().elements[0];
    let m_info = *i.get_mixed(m_atom);
    assert!(matches!(m_info.truthiness(), Truthiness::Truthy));
}

#[test]
fn truthy_mixed_meet_non_null_mixed_is_truthy_non_null() {
    let i = interner();
    let lhs = u(mixed_truthy());
    let rhs = u(mixed_nonnull());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    let m_atom = m.as_ref().elements[0];
    let m_info = *i.get_mixed(m_atom);
    assert!(matches!(m_info.truthiness(), Truthiness::Truthy));
    assert!(m_info.is_non_null());
}

#[test]
fn truthy_mixed_meet_object_passes_through() {
    let lhs = u(mixed_truthy());
    let rhs = u(t_object_any());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    assert!(does_refine(rhs, m, &w), "objects are truthy so truthy ∩ object should equal object");
}

#[test]
fn falsy_mixed_meet_int_is_zero_only() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_int());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let one = u(t_lit_int(1));
    assert!(!does_refine(one, m, &w), "int(1) is truthy so cannot be in falsy ∩ int (got meet {m})");
    let zero = u(t_lit_int(0));
    assert!(does_refine(zero, m, &w), "int(0) is falsy so must be in falsy ∩ int (got meet {m})");
}

#[test]
fn truthy_mixed_meet_int_excludes_zero() {
    let lhs = u(mixed_truthy());
    let rhs = u(t_int());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let zero = u(t_lit_int(0));
    assert!(!does_refine(zero, m, &w), "int(0) is falsy so cannot be in truthy ∩ int (got meet {m})");
    let one = u(t_lit_int(1));
    assert!(does_refine(one, m, &w), "int(1) is truthy so must be in truthy ∩ int (got meet {m})");
}

#[test]
fn falsy_mixed_meet_float_is_zero_only() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_float());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let one_float = u(ElementId::float_literal(1.5));
    assert!(!does_refine(one_float, m, &w), "float(1.5) is truthy so cannot be in falsy ∩ float (got meet {m})");
}

#[test]
fn truthy_mixed_meet_bool_is_true() {
    let lhs = u(mixed_truthy());
    let rhs = u(t_bool());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let truevalue = u(t_true());
    assert!(does_refine(truevalue, m, &w));
    let falsevalue = u(t_false());
    assert!(!does_refine(falsevalue, m, &w), "false is falsy so cannot be in truthy ∩ bool (got meet {m})");
}

#[test]
fn falsy_mixed_meet_bool_is_false() {
    let lhs = u(mixed_falsy());
    let rhs = u(t_bool());
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let falsevalue = u(t_false());
    assert!(does_refine(falsevalue, m, &w));
    let truevalue = u(t_true());
    assert!(!does_refine(truevalue, m, &w), "true is truthy so cannot be in falsy ∩ bool (got meet {m})");
}

#[test]
fn falsy_mixed_meet_uppercase_string_excludes_truthy_uppercase_literals() {
    let i = interner();
    let upper_string = i.intern_string(StringInfo {
        literal: StringLiteral::None,
        casing: StringCasing::Uppercase,
        flags: StringRefinementFlags::EMPTY,
    });
    let lhs = u(mixed_falsy());
    let rhs = u(upper_string);
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let abc = u(t_lit_string("ABC"));
    assert!(!does_refine(abc, m, &w), "string('ABC') is truthy ; falsy ∩ uppercase-string excludes it (got {m})");
}

#[test]
fn truthy_mixed_meet_lowercase_string_excludes_falsy_literals() {
    let i = interner();
    let lower_string = i.intern_string(StringInfo {
        literal: StringLiteral::None,
        casing: StringCasing::Lowercase,
        flags: StringRefinementFlags::EMPTY,
    });
    let lhs = u(mixed_truthy());
    let rhs = u(lower_string);
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let empty_lit = u(t_lit_string(""));
    let zero_lit = u(t_lit_string("0"));
    assert!(!does_refine(empty_lit, m, &w), "'' is falsy ; truthy ∩ lowercase-string excludes it");
    assert!(!does_refine(zero_lit, m, &w), "'0' is falsy ; truthy ∩ lowercase-string excludes it");
}

#[test]
fn falsy_mixed_meet_numeric_string_includes_zero_excludes_one() {
    let i = interner();
    let numeric_string = i.intern_string(StringInfo {
        literal: StringLiteral::None,
        casing: StringCasing::Unspecified,
        flags: StringRefinementFlags::EMPTY.with_is_numeric(true).with_is_non_empty(true),
    });
    let lhs = u(mixed_falsy());
    let rhs = u(numeric_string);
    assert_meet_is_subset(lhs, rhs);
    let w = empty_world();
    let m = lattice_meet(lhs, rhs, &w);
    let one_lit = u(t_lit_string("1"));
    assert!(!does_refine(one_lit, m, &w), "'1' is truthy ; falsy ∩ numeric-string excludes it (got {m})");
    let _ = atom("dummy");
}

#[test]
fn nonnull_mixed_meet_null_is_never() {
    let lhs = u(mixed_nonnull());
    let rhs = u(null());
    assert_meet_is_subset(lhs, rhs);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER);
}

#[test]
fn sealed_list_with_required_never_is_uninhabited() {
    let i = interner();
    let entries = vec![KnownElementEntry { index: 0, value: prelude::TYPE_NEVER, optional: false }];
    let known = i.intern_known_elements(&entries);
    let bad = i.intern_list(ListInfo {
        element_type: prelude::TYPE_NEVER,
        known_elements: Some(known),
        known_count: core::num::NonZeroU32::new(1),
        intersections: None,
        flags: ListFlags::default(),
    });

    let bad_t = u(bad);
    let other = u(t_list(prelude::TYPE_INT, false));
    let m = lattice_meet(bad_t, other, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "list with required never element is uninhabited (got {m})");
}

#[test]
fn intersected_int_excluding_zero_refines_truthy_mixed() {
    let i = interner();
    let zero_t = i.intern_type(&[prelude::INT_ZERO], FlowFlags::EMPTY);
    let neg_zero = ElementId::negated(zero_t);
    let nonzero_int = ElementId::intersected(prelude::INT, &[neg_zero]);
    let nonzero_t = u(nonzero_int);
    let truthy = u(mixed_truthy());
    assert!(
        does_refine(nonzero_t, truthy, &empty_world()),
        "int & !int(0) is non-zero int, all values truthy ; must refine truthy-mixed (got {nonzero_t})"
    );
}

#[test]
fn empty_list_singleton_refines_falsy_mixed() {
    let i = interner();
    let empty_list = i.intern_list(ListInfo {
        element_type: prelude::TYPE_NEVER,
        known_elements: None,
        known_count: None,
        intersections: None,
        flags: ListFlags::default(),
    });

    let empty_t = u(empty_list);
    let falsy = u(mixed_falsy());
    assert!(
        does_refine(empty_t, falsy, &empty_world()),
        "empty list is falsy ; must refine falsy-mixed (got {empty_t})"
    );
}

#[test]
fn empty_array_refines_falsy_mixed() {
    let empty_t = u(t_empty_array());
    let falsy = u(mixed_falsy());
    assert!(does_refine(empty_t, falsy, &empty_world()), "empty array is falsy ; must refine falsy-mixed");
}

#[test]
fn mixed_with_is_empty_implies_falsy_truthiness() {
    let i = interner();
    let empty_mixed = i.intern_mixed(MixedInfo::EMPTY.with_is_empty(true));
    let empty_t = u(empty_mixed);
    let falsy = u(mixed_falsy());
    assert!(
        does_refine(empty_t, falsy, &empty_world()),
        "mixed with is_empty is by definition falsy ; must refine falsy-mixed (got {empty_t})"
    );
}

#[test]
fn class_string_refines_truthy_mixed() {
    let cs = u(t_class_string());
    let truthy = u(mixed_truthy());
    assert!(does_refine(cs, truthy, &empty_world()), "class-strings are non-empty/non-zero ; must refine truthy-mixed");
}

#[test]
fn callable_refines_truthy_mixed() {
    let c = u(t_callable_any());
    let truthy = u(mixed_truthy());
    assert!(does_refine(c, truthy, &empty_world()), "callables are objects/closures, always truthy");
}

#[test]
fn resource_refines_truthy_mixed() {
    let r = u(t_resource());
    let truthy = u(mixed_truthy());
    assert!(does_refine(r, truthy, &empty_world()), "resources are always truthy");
}

#[test]
fn intersected_mixed_meet_drops_no_constraint_axis() {
    let i = interner();
    let truthy_nonnull_empty = i.intern_mixed(
        MixedInfo::EMPTY.with_truthiness(Truthiness::Truthy).with_is_non_null(true).with_is_isset_from_loop(true),
    );

    let target = u(truthy_nonnull_empty);
    let m = lattice_meet(target, target, &empty_world());
    assert_eq!(m, target, "self-meet should be identity (got {m})");
}

#[test]
fn intersected_truthy_int_in_falsy_mixed_meet_is_never() {
    let i = interner();
    let zero_t = i.intern_type(&[prelude::INT_ZERO], FlowFlags::EMPTY);
    let neg_zero = ElementId::negated(zero_t);
    let nonzero_int = ElementId::intersected(prelude::INT, &[neg_zero]);
    let lhs = u(mixed_falsy());
    let rhs = u(nonzero_int);
    let m = lattice_meet(lhs, rhs, &empty_world());
    assert_eq!(m, prelude::TYPE_NEVER, "falsy ∩ (int & !int(0)) must be never (got {m})");
}
