//! Property-based lattice axiom tests.
//!
//! Generates a random world (fixed structural shape, randomised
//! template variance + edges + member metadata) plus arbitrary types
//! that reference it, then asserts core lattice axioms across many
//! cases. Failures here are either real algorithm bugs or documented
//! precision gaps; the latter are filtered with `prop_assume!` rather
//! than `prop_assert!`.

mod comparator_common;

use comparator_common::*;

use std::sync::Arc;

use proptest::prelude::*;

use suffete::ElementId;
use suffete::FlowFlags;
use suffete::TypeId;
use suffete::interner::interner;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::lattice::overlaps;
use suffete::lattice::refines;
use suffete::meet;
use suffete::prelude;
use suffete::prelude::FLOAT;
use suffete::prelude::INT;
use suffete::prelude::STRING;
use suffete::subtract;
use suffete::world::Variance;

const CLASSES: &[&str] = &["A", "B", "C", "D", "E"];
const ENUMS: &[&str] = &["Color"];
const METHODS: &[&str] = &["doFoo", "getBar"];
const PROPERTIES: &[&str] = &["id", "name"];
const TEMPLATES: &[&str] = &["T", "U"];

#[derive(Debug, Clone)]
struct WorldHandle(Arc<MockWorld>);

impl std::ops::Deref for WorldHandle {
    type Target = MockWorld;
    fn deref(&self) -> &MockWorld {
        &self.0
    }
}

fn arb_variance() -> impl Strategy<Value = Variance> {
    prop_oneof![Just(Variance::Invariant), Just(Variance::Covariant), Just(Variance::Contravariant),]
}

fn arb_world() -> impl Strategy<Value = WorldHandle> {
    // Every class declares exactly one template parameter so that the
    // `t_generic_named("A", vec![t])` pattern in `arb_type` always
    // supplies the right number of args. Arity-mismatched types
    // (over- / under-supplied annotations) are a separate, deliberate
    // stress and should be tested by their own targeted strategy,
    // not by random collision with the world's declared arity here.
    let class_templates: Vec<_> = CLASSES.iter().map(|_| proptest::collection::vec(arb_variance(), 1)).collect();

    let edge_count = CLASSES.len() * (CLASSES.len() - 1) / 2;
    let edges = proptest::collection::vec(any::<bool>(), edge_count);

    let methods = proptest::collection::vec(any::<bool>(), CLASSES.len() * METHODS.len());
    let properties = proptest::collection::vec(any::<bool>(), CLASSES.len() * PROPERTIES.len());

    (class_templates, edges, methods, properties).prop_map(|(templates, edges, methods, properties)| {
        let mut w = MockWorld::new();

        for (idx, class) in CLASSES.iter().enumerate() {
            let variances = &templates[idx];
            if variances.is_empty() {
                w.declare(class);
            } else {
                let tmpl: Vec<(&str, Variance)> =
                    variances.iter().enumerate().map(|(i, v)| (TEMPLATES[i], *v)).collect();
                w.with_templates(class, &tmpl);
            }
        }

        let mut edge_idx = 0;
        for (i, child) in CLASSES.iter().enumerate() {
            for j in (i + 1)..CLASSES.len() {
                if edges[edge_idx] {
                    let parent_arity = templates[j].len();
                    let parent_args: Vec<TypeId> = (0..parent_arity).map(|_| prelude::TYPE_MIXED).collect();
                    w.with_extended(child, CLASSES[j], parent_args);
                }
                edge_idx += 1;
            }
        }

        for (i, class) in CLASSES.iter().enumerate() {
            for (j, method) in METHODS.iter().enumerate() {
                if methods[i * METHODS.len() + j] {
                    w.with_method(class, method);
                }
            }
        }

        for (i, class) in CLASSES.iter().enumerate() {
            for (j, property) in PROPERTIES.iter().enumerate() {
                if properties[i * PROPERTIES.len() + j] {
                    w.with_property(class, property, prelude::TYPE_MIXED);
                }
            }
        }

        for e in ENUMS {
            w.with_pure_enum(e);
        }

        WorldHandle(Arc::new(w))
    })
}

fn primitive_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(prelude::TYPE_INT),
        Just(prelude::TYPE_STRING),
        Just(prelude::TYPE_FLOAT),
        Just(prelude::TYPE_BOOL),
        Just(prelude::TYPE_NULL),
        Just(prelude::TYPE_VOID),
        Just(prelude::TYPE_MIXED),
        Just(prelude::TYPE_NEVER),
        Just(prelude::TYPE_OBJECT),
        Just(prelude::TYPE_SCALAR),
        Just(prelude::TYPE_NUMERIC),
        Just(prelude::TYPE_ARRAY_KEY),
    ]
}

fn literal_int_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(u(t_lit_int(0))),
        Just(u(t_lit_int(1))),
        Just(u(t_lit_int(-1))),
        Just(u(t_lit_int(42))),
        Just(u(t_lit_int(-42))),
    ]
}

fn refined_int_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(u(t_positive_int())),
        Just(u(t_negative_int())),
        Just(u(t_non_negative_int())),
        Just(u(t_non_positive_int())),
        Just(u(t_int_range(-10, 10))),
        Just(u(t_int_range(0, 100))),
        Just(u(t_int_from(0))),
        Just(u(t_int_to(0))),
    ]
}

fn literal_string_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(u(t_lit_string("foo"))),
        Just(u(t_lit_string("bar"))),
        Just(u(t_lit_string(""))),
        Just(u(t_lit_string("0"))),
        Just(u(t_lit_string("hello world"))),
    ]
}

fn refined_string_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(u(t_non_empty_string())),
        Just(u(t_numeric_string())),
        Just(u(t_lower_string())),
        Just(u(t_upper_string())),
        Just(u(t_truthy_string())),
        Just(u(t_class_string())),
        Just(u(t_interface_string())),
        Just(u(t_enum_string())),
    ]
}

fn literal_float_type() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(u(t_lit_float(0.0))),
        Just(u(t_lit_float(1.5))),
        Just(u(t_lit_float(-1.5))),
        Just(u(t_lit_float(42.0))),
        Just(u(t_unspec_lit_float())),
    ]
}

fn intersection_object_type() -> impl Strategy<Value = TypeId> {
    let heads = ["A", "B", "C"];
    let conjuncts = [t_named("D"), t_named("E"), t_has_method("doFoo"), t_has_property("id")];
    (proptest::sample::select(heads.to_vec()), proptest::sample::select(conjuncts.to_vec()))
        .prop_map(|(head, conjunct)| u(t_named_intersected(head, &[conjunct])))
}

fn class_object_type() -> impl Strategy<Value = TypeId> {
    proptest::sample::select(CLASSES.to_vec()).prop_map(|name| u(t_named(name)))
}

fn enum_type() -> impl Strategy<Value = TypeId> {
    proptest::sample::select(ENUMS.to_vec()).prop_map(|name| u(t_enum(name)))
}

fn has_method_type() -> impl Strategy<Value = TypeId> {
    proptest::sample::select(METHODS.to_vec()).prop_map(|name| u(t_has_method(name)))
}

fn has_property_type() -> impl Strategy<Value = TypeId> {
    proptest::sample::select(PROPERTIES.to_vec()).prop_map(|name| u(t_has_property(name)))
}

fn template_constraint() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(prelude::TYPE_MIXED),
        Just(prelude::TYPE_INT),
        Just(prelude::TYPE_STRING),
        Just(prelude::TYPE_NUMERIC),
        Just(prelude::TYPE_ARRAY_KEY),
        Just(prelude::TYPE_SCALAR),
        Just(interner().intern_type(&[INT, STRING], FlowFlags::EMPTY)),
        Just(interner().intern_type(&[INT, FLOAT], FlowFlags::EMPTY)),
    ]
}

fn template_type() -> impl Strategy<Value = TypeId> {
    let scopes = ["A", "B", "C"];
    let names = TEMPLATES;
    (proptest::sample::select(scopes.to_vec()), proptest::sample::select(names.to_vec()), template_constraint())
        .prop_map(|(scope, name, constraint)| u(t_template_of(scope, name, constraint)))
}

fn class_string_atom() -> impl Strategy<Value = TypeId> {
    prop_oneof![
        Just(suffete::TypeId::singleton(suffete::prelude::CLASS_STRING)),
        Just(suffete::TypeId::singleton(suffete::prelude::INTERFACE_STRING)),
        Just(suffete::TypeId::singleton(suffete::prelude::ENUM_STRING)),
        Just(suffete::TypeId::singleton(suffete::prelude::TRAIT_STRING)),
    ]
}

fn arb_type() -> impl Strategy<Value = TypeId> {
    let leaf = prop_oneof![
        4 => primitive_type(),
        2 => literal_int_type(),
        2 => refined_int_type(),
        2 => literal_string_type(),
        2 => refined_string_type(),
        1 => literal_float_type(),
        2 => class_object_type(),
        1 => enum_type(),
        1 => has_method_type(),
        1 => has_property_type(),
        2 => intersection_object_type(),
        2 => template_type(),
        1 => class_string_atom(),
    ];

    leaf.prop_recursive(5, 64, 6, |inner| {
        prop_oneof![
            inner.clone().prop_map(|t| u(t_list(t, false))),
            inner.clone().prop_map(|t| u(t_list(t, true))),
            (inner.clone(), inner.clone()).prop_map(|(k, v)| u(t_keyed_unsealed(k, v, false))),
            (inner.clone(), inner.clone()).prop_map(|(k, v)| u(t_iterable(k, v))),
            inner.clone().prop_map(|t| u(t_generic_named("A", vec![t]))),
            inner.clone().prop_map(|t| u(t_generic_named("B", vec![t]))),
            inner.clone().prop_map(|t| u(t_generic_named("C", vec![t]))),
            (inner.clone(), inner.clone(), any::<bool>(), any::<bool>())
                .prop_map(|(a, b, opt_a, sealed)| { u(t_object_shape(&[("x", a, opt_a), ("y", b, false)], sealed)) }),
            (inner.clone(), inner.clone()).prop_map(|(p, ret)| u(t_callable(&[p], ret))),
            (inner.clone(), inner.clone(), inner.clone()).prop_map(|(p1, p2, ret)| u(t_callable(&[p1, p2], ret))),
            (inner.clone(), inner.clone()).prop_map(|(a, b)| {
                let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
                elems.extend_from_slice(b.as_ref().elements);
                interner().intern_type(&elems, FlowFlags::EMPTY)
            }),
            (inner.clone(), inner.clone(), inner.clone()).prop_map(|(a, b, c)| {
                let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
                elems.extend_from_slice(b.as_ref().elements);
                elems.extend_from_slice(c.as_ref().elements);
                interner().intern_type(&elems, FlowFlags::EMPTY)
            }),
            (inner.clone(), inner.clone(), inner.clone(), inner.clone()).prop_map(|(a, b, c, d)| {
                let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
                elems.extend_from_slice(b.as_ref().elements);
                elems.extend_from_slice(c.as_ref().elements);
                elems.extend_from_slice(d.as_ref().elements);
                interner().intern_type(&elems, FlowFlags::EMPTY)
            }),
        ]
    })
}

fn arb_world_and_type() -> impl Strategy<Value = (WorldHandle, TypeId)> {
    (arb_world(), arb_type())
}

fn arb_world_and_pair() -> impl Strategy<Value = (WorldHandle, TypeId, TypeId)> {
    (arb_world(), arb_type(), arb_type())
}

fn arb_world_and_triple() -> impl Strategy<Value = (WorldHandle, TypeId, TypeId, TypeId)> {
    (arb_world(), arb_type(), arb_type(), arb_type())
}

fn does_refine(a: TypeId, b: TypeId, w: &MockWorld) -> bool {
    let mut report = LatticeReport::new();
    refines(a, b, w, LatticeOptions::default(), &mut report)
}

/// `true` for types whose every atom is structurally uninhabited
/// (e.g. `non-empty-list<never>`). The lattice keeps these distinct
/// from `never` representationally, so most properties need to skip
/// them rather than treating them like real inhabited types.
fn type_is_value_never(t: TypeId, w: &MockWorld) -> bool {
    if t == prelude::TYPE_NEVER {
        return true;
    }

    let elements = t.as_ref().elements;
    if elements.is_empty() {
        return true;
    }

    elements.iter().all(|e| {
        if element_is_value_never(*e, w) {
            return true;
        }
        let s = interner().intern_type(&[*e], FlowFlags::EMPTY);
        !does_overlap(s, s, w)
    })
}

fn does_overlap(a: TypeId, b: TypeId, w: &MockWorld) -> bool {
    let mut report = LatticeReport::new();
    overlaps(a, b, w, LatticeOptions::default(), &mut report)
}

/// `true` for an atom whose value-set is empty even though it isn't
/// the canonical `NEVER`: the obvious case is an object with an
/// intersection list that mixes nominal classes from different,
/// unrelated branches of the inheritance graph (`A & D` when neither
/// descends the other) — no runtime instance can be both at once.
fn element_is_value_never(elem: suffete::ElementId, w: &MockWorld) -> bool {
    if elem.kind() != suffete::ElementKind::Object {
        return false;
    }
    let info = interner().get_object(elem);
    let Some(intersections_id) = info.intersections else { return false };
    let mut classes: Vec<mago_atom::Atom> = vec![info.name];
    for &conjunct in interner().get_element_list(intersections_id) {
        if conjunct.kind() == suffete::ElementKind::Object {
            classes.push(interner().get_object(conjunct).name);
        }
    }
    use suffete::world::World;
    for (idx, &left) in classes.iter().enumerate() {
        for &right in &classes[idx + 1..] {
            if left == right {
                continue;
            }
            if !w.descends_from(left, right) && !w.descends_from(right, left) {
                return true;
            }
        }
    }
    false
}

fn meet_of(a: TypeId, b: TypeId, w: &MockWorld) -> TypeId {
    let mut report = LatticeReport::new();
    meet::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn subtract_of(a: TypeId, b: TypeId, w: &MockWorld) -> TypeId {
    let mut report = LatticeReport::new();
    subtract::compute(a, b, w, LatticeOptions::default(), &mut report)
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 512,
        max_shrink_iters: 1000,
        ..ProptestConfig::default()
    })]

    #[test]
    fn refines_is_reflexive((world, a) in arb_world_and_type()) {
        prop_assert!(does_refine(a, a, &world), "refines({a:?}, {a:?}) should be true");
    }

    #[test]
    fn refines_bottom_axiom((world, a) in arb_world_and_type()) {
        prop_assert!(does_refine(prelude::TYPE_NEVER, a, &world), "NEVER must refine {a:?}");
    }

    #[test]
    fn refines_top_axiom((world, a) in arb_world_and_type()) {
        prop_assert!(does_refine(a, prelude::TYPE_MIXED, &world), "{a:?} must refine MIXED");
    }

    #[test]
    fn refines_is_transitive((world, a, b, c) in arb_world_and_triple()) {
        if does_refine(a, b, &world) && does_refine(b, c, &world) {
            prop_assert!(
                does_refine(a, c, &world),
                "transitivity: {a:?} <: {b:?} <: {c:?} should imply {a:?} <: {c:?}"
            );
        }
    }

    #[test]
    fn overlaps_is_symmetric((world, a, b) in arb_world_and_pair()) {
        prop_assert_eq!(
            does_overlap(a, b, &world),
            does_overlap(b, a, &world),
            "overlaps should be symmetric for {:?} and {:?}", a, b
        );
    }

    #[test]
    fn overlaps_is_reflexive_for_non_bottom((world, a) in arb_world_and_type()) {
        if a != prelude::TYPE_NEVER {
            let elements = a.as_ref().elements;
            if elements.iter().all(|e| {
                let t = interner().intern_type(&[*e], FlowFlags::EMPTY);
                let u = interner().intern_type(&[*e], FlowFlags::EMPTY);
                !does_overlap(t, u, &world)
            }) {
                return Ok(());
            }

            prop_assert!(does_overlap(a, a, &world), "non-bottom {a:?} must overlap itself");
        }
    }

    #[test]
    fn refines_implies_overlaps((world, a, b) in arb_world_and_pair()) {
        // `type_is_value_never` filters out atoms that are
        // structurally non-`never` but inhabit nothing
        // (`non-empty-list<never>`, `A&D` with unrelated nominal
        // classes). They refine real types via vacuous truth, while
        // overlap correctly reports them as bottom.
        if type_is_value_never(a, &world) {
            return Ok(());
        }
        if does_refine(a, b, &world) {
            prop_assert!(
                does_overlap(a, b, &world),
                "refines implies overlaps for non-bottom: {a:?} <: {b:?}"
            );
        }
    }

    #[test]
    fn meet_is_idempotent((world, a) in arb_world_and_type()) {
        let m = meet_of(a, a, &world);
        prop_assert!(does_refine(m, a, &world), "meet(a, a) should refine a");
        prop_assert!(does_refine(a, m, &world), "a should refine meet(a, a)");
    }

    #[test]
    fn meet_is_commutative((world, a, b) in arb_world_and_pair()) {
        let ab = meet_of(a, b, &world);
        let ba = meet_of(b, a, &world);
        prop_assert!(
            does_refine(ab, ba, &world),
            "meet(a, b) <: meet(b, a)\n  a = {a}\n  b = {b}\n  meet(a,b) = {ab}\n  meet(b,a) = {ba}"
        );
        prop_assert!(
            does_refine(ba, ab, &world),
            "meet(b, a) <: meet(a, b)\n  a = {a}\n  b = {b}\n  meet(a,b) = {ab}\n  meet(b,a) = {ba}"
        );
    }

    #[test]
    fn meet_is_lower_bound((world, a, b) in arb_world_and_pair()) {
        let m = meet_of(a, b, &world);
        prop_assert!(does_refine(m, a, &world), "meet(a, b) should refine a");
        prop_assert!(does_refine(m, b, &world), "meet(a, b) should refine b");
    }

    #[test]
    fn meet_with_mixed_is_identity((world, a) in arb_world_and_type()) {
        if type_is_value_never(a, &world) {
            return Ok(());
        }

        let m = meet_of(a, prelude::TYPE_MIXED, &world);
        prop_assert!(does_refine(m, a, &world));
        prop_assert!(does_refine(a, m, &world));
    }

    #[test]
    fn subtract_with_never_is_identity((world, a) in arb_world_and_type()) {
        let s = subtract_of(a, prelude::TYPE_NEVER, &world);
        prop_assert!(does_refine(s, a, &world));
        prop_assert!(does_refine(a, s, &world));
    }

    #[test]
    fn subtract_self_is_never((world, a) in arb_world_and_type()) {
        let s = subtract_of(a, a, &world);
        prop_assert!(does_refine(s, prelude::TYPE_NEVER, &world));
    }

    #[test]
    fn subtract_is_sound((world, a, b) in arb_world_and_pair()) {
        let s = subtract_of(a, b, &world);
        prop_assert!(does_refine(s, a, &world), "subtract(a, b) must refine a");
    }

    #[test]
    fn meet_when_overlapping_is_non_empty((world, a, b) in arb_world_and_pair()) {
        // Precision: overlap=true should normally imply a non-NEVER
        // meet. Skipped cases: representation gaps where suffete has no
        // atom for the value-level intersection (cross-kind crossings
        // like `class-string ∧ lowercase-string`, or differently-named
        // generic parameters that overlap via their constraints but
        // can't be reified as a single template).
        if !does_overlap(a, b, &world) {
            return Ok(());
        }
        let has_generic = a.as_ref().elements.iter().chain(b.as_ref().elements.iter()).any(|e| {
            e.kind() == suffete::ElementKind::GenericParameter
        });
        prop_assume!(!has_generic);
        let has_same_kind = a.as_ref().elements.iter().any(|x| {
            b.as_ref().elements.iter().any(|y| x.kind() == y.kind())
        });
        prop_assume!(has_same_kind);
        let m = meet_of(a, b, &world);
        prop_assert!(
            m != prelude::TYPE_NEVER,
            "meet returned NEVER despite overlap\n  a = {a}\n  b = {b}"
        );
    }

    #[test]
    fn meet_subtract_partition((world, a, b) in arb_world_and_pair()) {
        // (A ∩ B) ∪ (A \ B) ⊇ A: every value of A is either in B
        // or not, so the union of meet+subtract must contain all of A.
        // Soundness check: catches cases where the result loses values.
        let m = meet_of(a, b, &world);
        let s = subtract_of(a, b, &world);
        let mut elems: Vec<ElementId> = m.as_ref().elements.to_vec();
        elems.extend_from_slice(s.as_ref().elements);
        let union = interner().intern_type(&elems, FlowFlags::EMPTY);
        prop_assert!(
            does_refine(a, union, &world),
            "A should refine meet(A, B) ∪ subtract(A, B)\n  a = {a}\n  b = {b}\n  meet = {m}\n  subtract = {s}\n  union = {union}"
        );
    }

    #[test]
    fn subtract_disjoint_is_identity((world, a, b) in arb_world_and_pair()) {
        if a != prelude::TYPE_NEVER && !does_overlap(a, a, &world) {
            return Ok(());
        }

        if does_overlap(a, b, &world) {
            return Ok(());
        }

        let s = subtract_of(a, b, &world);
        prop_assert!(
            does_refine(s, a, &world) && does_refine(a, s, &world),
            "disjoint subtract should be identity\n  a = {a}\n  b = {b}\n  result = {s}"
        );
    }

    #[test]
    fn subtract_when_subset_is_empty((world, a, b) in arb_world_and_pair()) {
        // When A <: B, every value of A is also in B, so A \ B ≡ ⊥.
        // Precision check: catches imprecise subtract.
        if a == prelude::TYPE_NEVER || !does_refine(a, b, &world) {
            return Ok(());
        }
        let s = subtract_of(a, b, &world);
        prop_assert!(
            does_refine(s, prelude::TYPE_NEVER, &world),
            "subtract(A, B) should be never when A <: B\n  a = {a}\n  b = {b}\n  result = {s}"
        );
    }

    #[test]
    fn meet_then_subtract_same_is_empty((world, a, b) in arb_world_and_pair()) {
        // (A ∩ B) \ B ≡ ⊥: meet picks values in both; subtracting B
        // must leave nothing. Precision check.
        let m = meet_of(a, b, &world);
        if m == prelude::TYPE_NEVER {
            return Ok(());
        }
        let s = subtract_of(m, b, &world);
        prop_assert!(
            does_refine(s, prelude::TYPE_NEVER, &world),
            "subtract(meet(A, B), B) should be never\n  a = {a}\n  b = {b}\n  meet = {m}\n  result = {s}"
        );
    }

    #[test]
    fn structural_join_is_idempotent_at_element_level((world, a) in arb_world_and_type()) {
        // The structural-only preset (sort + dedup + same-kind dominator)
        // produces a type equivalent to the original. The canonical
        // preset is allowed to widen (e.g. `lower | upper → string`).
        let elems = a.as_ref().elements;
        let opts = suffete::join::JoinOptions::structural();
        let canon = suffete::join::compute_with(elems, &opts);
        let rebuilt = interner().intern_type(&canon, FlowFlags::EMPTY);
        prop_assert!(
            does_refine(rebuilt, a, &world),
            "rebuilt should refine original\n  a = {a}\n  rebuilt = {rebuilt}"
        );
        prop_assert!(
            does_refine(a, rebuilt, &world),
            "original should refine rebuilt\n  a = {a}\n  rebuilt = {rebuilt}"
        );
    }

    #[test]
    fn canonical_join_widens_or_preserves((world, a) in arb_world_and_type()) {
        // The full canonical preset must produce a supertype of the
        // original (it's a join), but may strictly widen it.
        let elems = a.as_ref().elements;
        let canon = suffete::join::compute(elems);
        let rebuilt = interner().intern_type(&canon, FlowFlags::EMPTY);
        prop_assert!(
            does_refine(a, rebuilt, &world),
            "original should refine canonical\n  a = {a}\n  rebuilt = {rebuilt}"
        );
    }

    #[test]
    fn join_with_mixed_absorbs((_world, a) in arb_world_and_type()) {
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.push(prelude::MIXED);
        let canon = suffete::join::compute(&elems);
        prop_assert_eq!(canon.as_slice(), [prelude::MIXED].as_slice());
    }

    #[test]
    fn meet_with_never_is_never((world, a) in arb_world_and_type()) {
        let m = meet_of(a, prelude::TYPE_NEVER, &world);
        prop_assert_eq!(m, prelude::TYPE_NEVER, "meet(a, NEVER) should be NEVER for a={}", a);
        let m = meet_of(prelude::TYPE_NEVER, a, &world);
        prop_assert_eq!(m, prelude::TYPE_NEVER, "meet(NEVER, a) should be NEVER for a={}", a);
    }

    #[test]
    fn meet_is_associative_lower_bound((world, a, b, c) in arb_world_and_triple()) {
        let l = meet_of(meet_of(a, b, &world), c, &world);
        let r = meet_of(a, meet_of(b, c, &world), &world);
        prop_assert!(does_refine(l, a, &world), "(a∩b)∩c should refine a; got {l}");
        prop_assert!(does_refine(l, b, &world), "(a∩b)∩c should refine b; got {l}");
        prop_assert!(does_refine(l, c, &world), "(a∩b)∩c should refine c; got {l}");
        prop_assert!(does_refine(r, a, &world), "a∩(b∩c) should refine a; got {r}");
        prop_assert!(does_refine(r, b, &world), "a∩(b∩c) should refine b; got {r}");
        prop_assert!(does_refine(r, c, &world), "a∩(b∩c) should refine c; got {r}");
    }

    #[test]
    fn meet_monotonic_in_rhs((world, a, b, c) in arb_world_and_triple()) {
        if !does_refine(b, c, &world) {
            return Ok(());
        }
        let ab = meet_of(a, b, &world);
        let ac = meet_of(a, c, &world);
        prop_assert!(
            does_refine(ab, ac, &world),
            "monotonicity: b<:c implies meet(a,b)<:meet(a,c)\n  a={a}\n  b={b}\n  c={c}\n  meet(a,b)={ab}\n  meet(a,c)={ac}"
        );
    }

    #[test]
    fn subtract_monotonic_in_rhs((world, a, b, c) in arb_world_and_triple()) {
        if !does_refine(b, c, &world) {
            return Ok(());
        }
        let ac = subtract_of(a, c, &world);
        let ab = subtract_of(a, b, &world);
        prop_assert!(
            does_refine(ac, ab, &world),
            "anti-monotonicity: b<:c implies (a\\c)<:(a\\b)\n  a={a}\n  b={b}\n  c={c}\n  a\\b={ab}\n  a\\c={ac}"
        );
    }

    #[test]
    fn subtract_is_idempotent((world, a, b) in arb_world_and_pair()) {
        let s1 = subtract_of(a, b, &world);
        let s2 = subtract_of(s1, b, &world);
        prop_assert!(does_refine(s2, s1, &world), "(a\\b)\\b should refine a\\b");
        prop_assert!(does_refine(s1, s2, &world), "a\\b should refine (a\\b)\\b");
    }

    #[test]
    fn meet_idempotent_left((world, a, b) in arb_world_and_pair()) {
        let m1 = meet_of(a, b, &world);
        let m2 = meet_of(m1, b, &world);
        prop_assert!(does_refine(m2, m1, &world), "meet(meet(a,b),b) should refine meet(a,b)");
        prop_assert!(does_refine(m1, m2, &world), "meet(a,b) should refine meet(meet(a,b),b)");
    }

    #[test]
    fn meet_implies_refines_both((world, a, b) in arb_world_and_pair()) {
        let m = meet_of(a, b, &world);
        if m != prelude::TYPE_NEVER {
            prop_assert!(does_refine(m, a, &world), "meet result must refine a");
            prop_assert!(does_refine(m, b, &world), "meet result must refine b");
        }
    }

    #[test]
    fn meet_overlaps_iff_non_never((world, a, b) in arb_world_and_pair()) {
        if a != prelude::TYPE_NEVER && !does_overlap(a, a, &world) {
            return Ok(());
        }

        if b != prelude::TYPE_NEVER && !does_overlap(b, b, &world) {
            return Ok(());
        }

        let m = meet_of(a, b, &world);
        if m != prelude::TYPE_NEVER {
            prop_assert!(does_overlap(a, b, &world), "non-never meet should imply overlap\n  a={a}\n  b={b}\n  m={m}");
        }
    }

    #[test]
    fn refines_implies_meet_is_input((world, a, b) in arb_world_and_pair()) {
        if !does_refine(a, b, &world) {
            return Ok(());
        }
        let m = meet_of(a, b, &world);
        prop_assert!(
            does_refine(m, a, &world) && does_refine(a, m, &world),
            "a<:b implies meet(a,b)≡a\n  a={a}\n  b={b}\n  meet={m}"
        );
    }

    #[test]
    fn subtract_followed_by_meet_with_b_is_empty((world, a, b) in arb_world_and_pair()) {
        let s = subtract_of(a, b, &world);
        if s == prelude::TYPE_NEVER {
            return Ok(());
        }
        // Skip when subtract didn't strictly narrow (`a <: s`): the
        // precision invariant only holds when subtract actually
        // removed values. Identity-equivalent subtract is a documented
        // precision loss, not a soundness one.
        if does_refine(a, s, &world) {
            return Ok(());
        }
        let recheck = meet_of(s, b, &world);
        // Skip when the surviving overlap lives in a generic parameter:
        // narrowing `T extends numeric` by `\ int` would require a
        // `non-int-numeric` representation we don't model, so subtract
        // leaves the constraint intact and meet legitimately re-finds
        // values via the constraint.
        let has_generic = recheck.as_ref().elements.iter().any(|e| {
            e.kind() == suffete::ElementKind::GenericParameter
        });
        if has_generic {
            return Ok(());
        }
        prop_assert!(
            does_refine(recheck, prelude::TYPE_NEVER, &world),
            "(a\\b) ∩ b should be never\n  a={a}\n  b={b}\n  a\\b={s}\n  result={recheck}"
        );
    }

    #[test]
    fn join_is_upper_bound((world, a, b) in arb_world_and_pair()) {
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.extend_from_slice(b.as_ref().elements);
        let joined_atoms = suffete::join::compute(&elems);
        let joined = interner().intern_type(&joined_atoms, FlowFlags::EMPTY);
        prop_assert!(does_refine(a, joined, &world), "a should refine join(a,b)\n  a={a}\n  b={b}\n  join={joined}");
        prop_assert!(does_refine(b, joined, &world), "b should refine join(a,b)\n  a={a}\n  b={b}\n  join={joined}");
    }

    #[test]
    fn join_with_never_is_identity((world, a) in arb_world_and_type()) {
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.push(prelude::NEVER);
        let opts = suffete::join::JoinOptions::structural();
        let canon = suffete::join::compute_with(&elems, &opts);
        let rebuilt = interner().intern_type(&canon, FlowFlags::EMPTY);
        prop_assert!(does_refine(rebuilt, a, &world), "structural-join(a, NEVER) should refine a");
        prop_assert!(does_refine(a, rebuilt, &world), "a should refine structural-join(a, NEVER)");
    }

    #[test]
    fn refines_is_antisymmetric_modulo_equivalence((world, a, b) in arb_world_and_pair()) {
        if does_refine(a, b, &world) && does_refine(b, a, &world) {
            let mab = meet_of(a, b, &world);
            prop_assert!(does_refine(mab, a, &world));
            prop_assert!(does_refine(a, mab, &world));
            prop_assert!(does_refine(mab, b, &world));
            prop_assert!(does_refine(b, mab, &world));
        }
    }

    #[test]
    fn overlaps_with_never_is_false((world, a) in arb_world_and_type()) {
        prop_assert!(!does_overlap(a, prelude::TYPE_NEVER, &world), "nothing overlaps NEVER");
        prop_assert!(!does_overlap(prelude::TYPE_NEVER, a, &world), "NEVER overlaps nothing");
    }

    #[test]
    fn disjoint_implies_meet_never((world, a, b) in arb_world_and_pair()) {
        if type_is_value_never(a, &world) || type_is_value_never(b, &world) {
            return Ok(());
        }

        if does_overlap(a, b, &world) {
            return Ok(());
        }

        let m = meet_of(a, b, &world);
        prop_assert_eq!(m, prelude::TYPE_NEVER, "disjoint inputs must meet to NEVER\n  a={}\n  b={}", a, b);
    }

    #[test]
    fn double_subtract_with_swapped_args_equivalent((world, a, b, c) in arb_world_and_triple()) {
        let bc = subtract_of(subtract_of(a, b, &world), c, &world);
        let cb = subtract_of(subtract_of(a, c, &world), b, &world);
        prop_assert!(does_refine(bc, cb, &world), "(a\\b)\\c should refine (a\\c)\\b");
        prop_assert!(does_refine(cb, bc, &world), "(a\\c)\\b should refine (a\\b)\\c");
    }
}
