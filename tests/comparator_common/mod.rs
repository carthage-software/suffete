#![allow(dead_code)]

//! Test helpers mirroring `mago/crates/codex/tests/comparator_common/mod.rs`,
//! translated to suffete's lattice API. Test files in `tests/comparator_*.rs`
//! consume these helpers so the porting from mago is mechanical.
//!
//! Translation:
//!
//! | mago                                   | suffete                                          |
//! |----------------------------------------|--------------------------------------------------|
//! | `union_comparator::is_contained_by`    | `lattice::refines`                               |
//! | `atomic_comparator::is_contained_by`   | `lattice::refines` on singleton unions           |
//! | `ComparisonResult`                     | `LatticeReport`                                  |
//! | `is_contained_by(_, _, in, if, ia, _)` | `LatticeOptions { ignore_null, .. }`             |
//! | `CodebaseMetadata`                     | `dyn lattice::Codebase`                          |
//! | `codebase_from_php(code)`              | hand-built [`MockCodebase`]                      |

use std::collections::HashMap;
use std::collections::HashSet;

use mago_atom::Atom;
use mago_atom::atom;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
use suffete::lattice;
use suffete::lattice::Codebase;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::NullCodebase;
use suffete::prelude;

/// A `Codebase` impl built from explicit `(child, parent)` edges. The
/// transitive ancestor closure is recomputed on every [`add_edge`] call so
/// that `is_subclass_of` is O(1) per query (one hash lookup).
///
/// Stands in for mago's `codebase_from_php(php_source)`: instead of parsing
/// PHP, the test names the hierarchy directly.
pub struct MockCodebase {
    /// `child -> {ancestors including child itself}`.
    ancestors: HashMap<Atom, HashSet<Atom>>,
}

impl MockCodebase {
    pub fn new() -> Self {
        Self { ancestors: HashMap::new() }
    }

    /// Add a single `child extends/implements parent` edge and recompute
    /// the closure.
    pub fn add_edge(&mut self, child: &str, parent: &str) -> &mut Self {
        let child = atom(child);
        let parent = atom(parent);
        self.ancestors.entry(child).or_default().insert(child);
        self.ancestors.entry(parent).or_default().insert(parent);
        self.ancestors.get_mut(&child).unwrap().insert(parent);
        self.recompute_closure();
        self
    }

    /// Build from a list of `(child, parent)` pairs in one shot.
    pub fn from_edges(edges: &[(&str, &str)]) -> Self {
        let mut cb = Self::new();
        for (c, p) in edges {
            cb.add_edge(c, p);
        }
        cb
    }

    /// Register a class-like that has no ancestors (so `is_subclass_of(C, C)`
    /// still works for it).
    pub fn declare(&mut self, name: &str) -> &mut Self {
        let n = atom(name);
        self.ancestors.entry(n).or_default().insert(n);
        self
    }

    fn recompute_closure(&mut self) {
        // Floyd-Warshall-ish: keep adding ancestors of ancestors until fixed.
        loop {
            let mut changed = false;
            let names: Vec<Atom> = self.ancestors.keys().copied().collect();
            for name in &names {
                let direct: Vec<Atom> = self.ancestors[name].iter().copied().collect();
                for d in direct {
                    if let Some(ancestors_of_d) = self.ancestors.get(&d).cloned() {
                        let entry = self.ancestors.get_mut(name).unwrap();
                        for a in ancestors_of_d {
                            if entry.insert(a) {
                                changed = true;
                            }
                        }
                    }
                }
            }
            if !changed {
                break;
            }
        }
    }
}

impl Default for MockCodebase {
    fn default() -> Self {
        Self::new()
    }
}

impl Codebase for MockCodebase {
    fn is_subclass_of(&self, child: Atom, parent: Atom) -> bool {
        if child == parent {
            return true;
        }

        self.ancestors.get(&child).is_some_and(|set| set.contains(&parent))
    }
}

/// An empty codebase: nothing knows about anything. Equivalent to mago's
/// `empty_codebase()`.
pub fn empty_codebase() -> NullCodebase {
    NullCodebase
}

// ---------------------------------------------------------------------------
// Refinement queries.
// ---------------------------------------------------------------------------

/// `lattice::refines(input, container, codebase, default options, _)`.
/// Mirrors mago's `union_comparator::is_contained_by`.
pub fn is_contained<C: Codebase>(input: TypeId, container: TypeId, codebase: &C) -> bool {
    let mut report = LatticeReport::new();
    lattice::refines(input, container, codebase, LatticeOptions::default(), &mut report)
}

/// As [`is_contained`], but returns the [`LatticeReport`] alongside the
/// boolean answer so tests can inspect coercion flags.
pub fn is_contained_capturing<C: Codebase>(input: TypeId, container: TypeId, codebase: &C) -> (bool, LatticeReport) {
    let mut report = LatticeReport::new();
    let v = lattice::refines(input, container, codebase, LatticeOptions::default(), &mut report);
    (v, report)
}

/// `is_contained` with the `ignore_null` / `ignore_false` / `inside_assertion`
/// option flags. Mirrors mago's
/// `is_contained_by(..., ignore_null, ignore_false, inside_assertion, _)`.
pub fn is_contained_with<C: Codebase>(
    input: TypeId,
    container: TypeId,
    codebase: &C,
    ignore_null: bool,
    ignore_false: bool,
    inside_assertion: bool,
) -> bool {
    let options = LatticeOptions { ignore_null, ignore_false, inside_assertion };
    let mut report = LatticeReport::new();
    lattice::refines(input, container, codebase, options, &mut report)
}

/// Element-vs-element refinement query: wraps both in singleton unions and
/// calls [`is_contained`].
pub fn atomic_is_contained<C: Codebase>(input: ElementId, container: ElementId, codebase: &C) -> bool {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained(it, ct, codebase)
}

pub fn atomic_is_contained_capturing<C: Codebase>(
    input: ElementId,
    container: ElementId,
    codebase: &C,
) -> (bool, LatticeReport) {
    let i = interner();
    let it = i.intern_type(&[input], FlowFlags::EMPTY);
    let ct = i.intern_type(&[container], FlowFlags::EMPTY);
    is_contained_capturing(it, ct, codebase)
}

// ---------------------------------------------------------------------------
// Assertion helpers.
// ---------------------------------------------------------------------------

#[track_caller]
pub fn assert_subtype(input: &TypeId, container: &TypeId) {
    let cb = empty_codebase();
    assert!(is_contained(*input, *container, &cb), "expected {input:?} <: {container:?} but it is not");
}

#[track_caller]
pub fn assert_not_subtype(input: &TypeId, container: &TypeId) {
    let cb = empty_codebase();
    assert!(!is_contained(*input, *container, &cb), "expected NOT ({input:?} <: {container:?}) but it is");
}

#[track_caller]
pub fn assert_atomic_subtype(input: &ElementId, container: &ElementId) {
    let cb = empty_codebase();
    assert!(atomic_is_contained(*input, *container, &cb), "expected atomic {input:?} <: {container:?}");
}

#[track_caller]
pub fn assert_atomic_not_subtype(input: &ElementId, container: &ElementId) {
    let cb = empty_codebase();
    assert!(!atomic_is_contained(*input, *container, &cb), "expected NOT (atomic {input:?} <: {container:?})");
}

// ---------------------------------------------------------------------------
// Element constructors. Mirrors mago's `t_*` helpers.
// ---------------------------------------------------------------------------

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
pub fn t_lit_int(v: i64) -> ElementId {
    ElementId::int_literal(v)
}
pub fn t_int_from(from: i64) -> ElementId {
    ElementId::int_range(Some(from), None)
}
pub fn t_int_to(to: i64) -> ElementId {
    ElementId::int_range(None, Some(to))
}
pub fn t_int_range(lo: i64, hi: i64) -> ElementId {
    ElementId::int_range(Some(lo), Some(hi))
}
pub fn t_int_unspec_lit() -> ElementId {
    prelude::LITERAL_INT
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

pub fn t_float() -> ElementId {
    prelude::FLOAT
}
pub fn t_lit_float(v: f64) -> ElementId {
    ElementId::float_literal(v)
}
pub fn t_unspec_lit_float() -> ElementId {
    prelude::LITERAL_FLOAT
}

pub fn t_string() -> ElementId {
    prelude::STRING
}
pub fn t_lit_string(s: &str) -> ElementId {
    ElementId::string_literal(s)
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
pub fn t_unspec_lit_string(non_empty: bool) -> ElementId {
    if non_empty { prelude::NON_EMPTY_LITERAL_STRING } else { prelude::LITERAL_STRING }
}
pub fn t_callable_string() -> ElementId {
    prelude::CALLABLE_STRING
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

pub fn t_object_any() -> ElementId {
    prelude::OBJECT
}
pub fn t_named(name: &str) -> ElementId {
    ElementId::object_named(name)
}
pub fn t_enum(name: &str) -> ElementId {
    ElementId::enum_any(name)
}
pub fn t_enum_case(name: &str, case: &str) -> ElementId {
    ElementId::enum_case(name, case)
}

pub fn t_empty_array() -> ElementId {
    prelude::EMPTY_ARRAY
}

// ---------------------------------------------------------------------------
// Union builders. Mirrors mago's `u`, `u_many`, `ui`, `us`.
// ---------------------------------------------------------------------------

pub fn u(a: ElementId) -> TypeId {
    interner().intern_type(&[a], FlowFlags::EMPTY)
}

pub fn u_many(atoms: Vec<ElementId>) -> TypeId {
    interner().intern_type(&atoms, FlowFlags::EMPTY)
}

pub fn ui(v: i64) -> TypeId {
    u(t_lit_int(v))
}

pub fn us(s: &str) -> TypeId {
    u(t_lit_string(s))
}

pub fn name(s: &str) -> Atom {
    atom(s)
}
