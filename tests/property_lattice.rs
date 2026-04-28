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
    let class_templates: Vec<_> = CLASSES
        .iter()
        .map(|_| (0usize..=2usize).prop_flat_map(|n| proptest::collection::vec(arb_variance(), n)))
        .collect();

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

fn arb_type() -> impl Strategy<Value = TypeId> {
    let leaf = prop_oneof![
        primitive_type(),
        literal_int_type(),
        refined_int_type(),
        literal_string_type(),
        refined_string_type(),
        class_object_type(),
        enum_type(),
        has_method_type(),
        has_property_type(),
    ];

    leaf.prop_recursive(4, 32, 4, |inner| {
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
            (inner.clone(), inner.clone()).prop_map(|(a, b)| {
                let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
                elems.extend_from_slice(b.as_ref().elements);
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

fn does_overlap(a: TypeId, b: TypeId, w: &MockWorld) -> bool {
    let mut report = LatticeReport::new();
    overlaps(a, b, w, LatticeOptions::default(), &mut report)
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
            prop_assert!(does_overlap(a, a, &world), "non-bottom {a:?} must overlap itself");
        }
    }

    #[test]
    fn refines_implies_overlaps((world, a, b) in arb_world_and_pair()) {
        if a != prelude::TYPE_NEVER && does_refine(a, b, &world) {
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
    fn join_is_idempotent_at_element_level((world, a) in arb_world_and_type()) {
        let elems = a.as_ref().elements;
        let canon = suffete::join::compute(elems);
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
    fn join_with_mixed_absorbs((_world, a) in arb_world_and_type()) {
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.push(prelude::MIXED);
        let canon = suffete::join::compute(&elems);
        prop_assert_eq!(canon.as_slice(), [prelude::MIXED].as_slice());
    }
}
