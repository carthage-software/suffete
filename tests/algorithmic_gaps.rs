//! Failing-by-design tests that pin known algorithmic gaps in the
//! lattice. Each test is `#[ignore]`d so the suite stays green in CI;
//! the test body shows the *expected* outcome once the gap is
//! implemented and panics today, with a comment block documenting
//! the missing rule and where it should land in the source tree.
//!
//! Run with `cargo test --test algorithmic_gaps -- --ignored` to
//! follow progress as gaps close.
//!
//! Layout: one `#[test]` per gap; each is preceded by a `// GAP N:`
//! comment that matches the followup queue in
//! `memory/project_perfection_mandate.md`.

mod comparator_common;

use comparator_common::*;

use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::refines;
use suffete::meet;
use suffete::prelude;
use suffete::subtract;
use suffete::world::Variance;

fn lattice_meet<W: suffete::world::World>(a: suffete::TypeId, b: suffete::TypeId, w: &W) -> suffete::TypeId {
    meet::compute(a, b, w, LatticeOptions::default(), &mut LatticeReport::new())
}

fn lattice_subtract<W: suffete::world::World>(a: suffete::TypeId, b: suffete::TypeId, w: &W) -> suffete::TypeId {
    subtract::compute(a, b, w, LatticeOptions::default(), &mut LatticeReport::new())
}

fn does_refine<W: suffete::world::World>(a: suffete::TypeId, b: suffete::TypeId, w: &W) -> bool {
    refines(a, b, w, LatticeOptions::default(), &mut LatticeReport::new())
}

// GAP 1: descendant template-argument resolution in `compose`.
//
// When two object atoms share a `descends_from` relationship, the
// compose path in `src/meet/family/object.rs::compose_object_intersection`
// should resolve the descendant's view of the ancestor through
// `World::inherited_template_argument` and reconcile the args under
// the ancestor's variance. Today the compose just glues the two
// participants regardless of the relationship, which leaves invariant
// arg mismatches uncaught.
//
// Concretely: `class B<T> extends C<T>`. `B<int>` viewed as `C` is
// `C<int>`. `meet(B<int>, C<object>)` under invariant T must check
// `int ≡ object` and collapse to `never` because they don't match.
// Today the compose returns `B<int> & C<object>` which is uninhabited
// but represented as inhabited.
#[test]
#[ignore = "algorithmic gap: compose doesn't resolve descendant template args"]
fn gap_compose_descendant_invariant_mismatch_collapses_to_never() {
    let mut w = MockWorld::new();
    w.with_templates("B", &[("T", Variance::Invariant)]);
    w.with_templates("C", &[("T", Variance::Invariant)]);
    let t_param = u(t_template("B", "T"));
    w.with_extended("B", "C", vec![t_param]);

    let b_int = u(t_generic_named("B", vec![u(t_int())]));
    let c_object = u(t_generic_named("C", vec![u(t_object_any())]));

    let m = lattice_meet(b_int, c_object, &w);
    assert_eq!(
        m,
        prelude::TYPE_NEVER,
        "B<int> as C is C<int>; meet with C<object> under invariant T should be never (got {m})"
    );
}

// GAP 2: subtract fan-out for true-union dominator kinds.
//
// `scalar`, `numeric`, `array-key`, `mixed` decompose into disjoint
// sub-families. `subtract(scalar, int)` should split `scalar` into
// `bool | int | float | string` and remove `int`, yielding
// `bool | float | string`. Today `src/subtract/mod.rs::atom_minus`
// returns identity (the broad `scalar` atom) because no family rule
// fires for `Scalar × Int`.
#[test]
#[ignore = "algorithmic gap: subtract has no fan-out for true-union dominators"]
fn gap_subtract_scalar_minus_int_splits_to_other_scalars() {
    let cb = empty_world();
    let s = lattice_subtract(prelude::TYPE_SCALAR, prelude::TYPE_INT, &cb);
    let expected = u_many(vec![t_bool(), t_float(), t_string()]);
    assert!(
        does_refine(s, expected, &cb) && does_refine(expected, s, &cb),
        "scalar \\ int should equal bool|float|string (got {s})"
    );
}

#[test]
#[ignore = "algorithmic gap: subtract has no fan-out for true-union dominators"]
fn gap_subtract_array_key_minus_int_yields_string() {
    let cb = empty_world();
    let s = lattice_subtract(prelude::TYPE_ARRAY_KEY, prelude::TYPE_INT, &cb);
    let expected = u(t_string());
    assert!(
        does_refine(s, expected, &cb) && does_refine(expected, s, &cb),
        "array-key \\ int should equal string (got {s})"
    );
}

#[test]
#[ignore = "algorithmic gap: subtract has no fan-out for true-union dominators"]
fn gap_subtract_numeric_minus_int_yields_float_or_numeric_string() {
    let cb = empty_world();
    let s = lattice_subtract(prelude::TYPE_NUMERIC, prelude::TYPE_INT, &cb);
    let expected = u_many(vec![t_float(), t_numeric_string()]);
    assert!(
        does_refine(s, expected, &cb) && does_refine(expected, s, &cb),
        "numeric \\ int should equal float|numeric-string (got {s})"
    );
}

// GAP 3: subtract for object descendants.
//
// `class A extends B`. `B \ A` should yield "B-instances that are not
// also A-instances". A precise lattice would either represent this as
// a refined object atom (e.g. `B & !A`) or, when `B` is final and
// `A`'s descendants do not exhaust `B`, drop to `never`. Today
// `src/subtract/mod.rs::atom_minus` returns the unchanged `B` because
// no rule covers `Object × Object` subtract.
#[test]
#[ignore = "algorithmic gap: subtract doesn't remove descendant classes"]
fn gap_subtract_b_minus_descendant_a_excludes_a_instances() {
    let mut w = MockWorld::new();
    w.declare("B");
    w.declare("A");
    w.with_extended("A", "B", vec![]);

    let b = u(t_named("B"));
    let a = u(t_named("A"));
    let s = lattice_subtract(b, a, &w);

    // The minimal correctness check: meet of (B \ A) with A should
    // be never — every A is removed from the surviving B-instances.
    let m = lattice_meet(s, a, &w);
    assert_eq!(m, prelude::TYPE_NEVER, "(B \\ A) ∩ A should be never (got {m}; B \\ A = {s})");
}

// GAP 4: cross-kind meet — `Iterable × Array` (the `Iterable × List`
// case already works via refines subsumption: a list refines an
// iterable directly, so meet returns the list).
//
// `iterable<K, V>` ranges over `array` shapes, `Generator`, and any
// `\Traversable`. Meeting it with `array<K', V'>` should restrict
// to `array<K∩K', V∩V'>`. Today no rule fires in
// `family_atom_meet` for the pair `(Iterable, Array)` and the meet
// collapses to `never`.
#[test]
#[ignore = "algorithmic gap: meet has no Iterable×Array rule"]
fn gap_meet_iterable_with_array_yields_array() {
    let cb = empty_world();
    let it = u(t_iterable(u(t_int()), u(t_string())));
    let arr = u(t_keyed_unsealed(u(t_array_key()), u(t_string()), false));
    let m = lattice_meet(it, arr, &cb);
    let expected = u(t_keyed_unsealed(u(t_int()), u(t_string()), false));
    assert!(
        does_refine(m, expected, &cb) && does_refine(expected, m, &cb),
        "meet(iterable<int,string>, array<array-key,string>) should narrow to array<int,string> (got {m})"
    );
}

// GAP 6: refines fan-out for refined-string complements.
//
// `string` should refine `non-empty-string | string('')` (this works,
// via `string_union_covers`). The natural extension is the casing
// axis: `string` should refine `lowercase-string | (string &
// has-uppercase)` once we have a `has-uppercase` (or
// `non-lowercase-string`) atom. The test pins the form once the
// representation lands.
#[test]
#[ignore = "algorithmic gap: requires non-lowercase-string complement representation"]
fn gap_refines_string_covered_by_lowercase_and_non_lowercase() {
    let cb = empty_world();
    // Today there is no `non-lowercase-string` representation, so the
    // test asserts the property once it lands. The placeholder uses
    // `t_upper_string()` as a stand-in: every string is either
    // lowercase OR has at least one uppercase letter, but
    // `upper_string` only captures the *all-uppercase* subset and
    // is therefore strictly smaller than the true complement.
    let s = u(t_string());
    let split = u_many(vec![t_lower_string(), t_upper_string()]);
    assert!(
        does_refine(s, split, &cb),
        "string should refine lowercase-string | <non-lowercase-complement> once the complement atom exists"
    );
}

// GAP 7: refines arity-mismatched object args consistency.
//
// The lattice currently treats arity-0 classes that carry explicit
// args (e.g. `Foo<int>` where `Foo` has zero declared template
// parameters) inconsistently across `refines`, `overlaps`, and
// `meet`. The principled answer: such atoms are syntactically
// invalid and should reduce to the bare class. The test pins the
// reduction.
#[test]
#[ignore = "algorithmic gap: arity-0 + explicit args should canonicalize to bare class"]
fn gap_arity_zero_class_with_explicit_args_reduces_to_bare() {
    let mut w = MockWorld::new();
    w.declare("Foo");
    let with_args = u(t_generic_named("Foo", vec![u(t_int())]));
    let bare = u(t_named("Foo"));
    assert!(
        does_refine(with_args, bare, &w) && does_refine(bare, with_args, &w),
        "arity-0 Foo<int> should be value-equivalent to bare Foo\n  with_args={with_args}\n  bare={bare}"
    );
}
