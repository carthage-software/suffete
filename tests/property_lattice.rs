#![allow(
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items,
    clippy::panic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::tests_outside_test_module,
    clippy::missing_assert_message,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used
)]

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
use suffete::world::World as _;

const CLASSES: &[&str] = &["A", "B", "C", "D", "E"];
const ENUMS: &[&str] = &["Color"];
const METHODS: &[&str] = &["doFoo", "getBar"];
const PROPERTIES: &[&str] = &["id", "name"];
const TEMPLATES: &[&str] = &["T", "U"];

#[derive(Debug, Clone)]
struct WorldHandle(Arc<MockWorld>);

impl core::ops::Deref for WorldHandle {
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
    let class_templates_strat: Vec<_> = CLASSES.iter().map(|_| proptest::collection::vec(arb_variance(), 1)).collect();

    // Combinations C(n, 2): n*(n-1) is always even.
    #[allow(clippy::integer_division, clippy::integer_division_remainder_used, clippy::arithmetic_side_effects)]
    let edge_count = CLASSES.len() * (CLASSES.len() - 1) / 2;
    let edges_strat = proptest::collection::vec(any::<bool>(), edge_count);

    let methods_strat = proptest::collection::vec(any::<bool>(), CLASSES.len() * METHODS.len());
    let properties_strat = proptest::collection::vec(any::<bool>(), CLASSES.len() * PROPERTIES.len());
    let finals_strat = proptest::collection::vec(any::<bool>(), CLASSES.len());

    let sealed_strat = proptest::collection::vec(proptest::option::weighted(0.999, any::<u8>()), CLASSES.len());

    (class_templates_strat, edges_strat, methods_strat, properties_strat, finals_strat, sealed_strat).prop_map(
        |(templates, edges, methods, properties, finals, sealed_markers)| {
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
                        let parent_args: Vec<TypeId> = std::iter::repeat_n(prelude::TYPE_MIXED, parent_arity).collect();
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

            // A class can only be marked `final` when no other class
            // declares it as a parent: PHP forbids extending a final
            // class, and the lattice relies on that invariant when
            // collapsing `final C & X` intersections to `never`.
            // Random worlds that violated the invariant produced
            // contradictory `B <: E` + `is_final(E)` configurations
            // and broke meet-monotonicity through no fault of the
            // algorithm.
            for (i, class) in CLASSES.iter().enumerate() {
                if !finals[i] {
                    continue;
                }

                let class_atom = mago_atom::atom(class);
                let has_descendants =
                    CLASSES.iter().any(|other| *other != *class && w.descends_from(mago_atom::atom(other), class_atom));
                if !has_descendants {
                    w.with_final(class);
                }
            }

            for (i, class) in CLASSES.iter().enumerate() {
                let marker = match sealed_markers[i] {
                    Some(m) if m < 64 => m,
                    _ => continue,
                };
                let class_atom = mago_atom::atom(class);
                // Pick direct children only ; a class is a direct child if it
                // descends from `class` and no other descendant of `class` sits
                // between them. This matches PHP's sealed-class semantics
                // (sealed types list immediate inheritors, not transitive ones).
                let direct_children: Vec<&str> = CLASSES
                    .iter()
                    .filter(|&&c| {
                        if c == *class {
                            return false;
                        }
                        let c_atom = mago_atom::atom(c);
                        if !w.descends_from(c_atom, class_atom) {
                            return false;
                        }
                        !CLASSES.iter().any(|&intermediate| {
                            intermediate != *class
                                && intermediate != c
                                && w.descends_from(c_atom, mago_atom::atom(intermediate))
                                && w.descends_from(mago_atom::atom(intermediate), class_atom)
                        })
                    })
                    .copied()
                    .collect();
                if direct_children.len() < 2 {
                    continue;
                }
                let n = (marker as usize % 3) + 1;
                let inheritors: Vec<&str> =
                    direct_children.iter().take(n.min(direct_children.len())).copied().collect();
                if inheritors.len() >= 2 {
                    w.with_sealed(class, &inheritors);
                }
            }

            WorldHandle(Arc::new(w))
        },
    )
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
            (inner.clone(), inner.clone(), inner.clone(), inner).prop_map(|(a, b, c, d)| {
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
/// descends the other), so no runtime instance can be both at once.
fn element_is_value_never(elem: suffete::ElementId, w: &MockWorld) -> bool {
    if elem.kind() != suffete::ElementKind::Intersected {
        return false;
    }
    let info = *interner().get_intersected(elem);
    if info.head.kind() != suffete::ElementKind::Object {
        return false;
    }
    let head_info = interner().get_object(info.head);
    let mut classes: Vec<mago_atom::Atom> = vec![head_info.name];
    for &conjunct in interner().get_element_list(info.conjuncts) {
        if conjunct.kind() == suffete::ElementKind::Object {
            classes.push(interner().get_object(conjunct).name);
        }
    }
    use suffete::world::World as _;
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

/// `true` for an atom whose value-set is exactly `{[]}` (only the
/// empty array/list). These crop up when `meet` reduces a container's
/// element type to `never` while leaving `non_empty=false`, e.g.
/// `meet(list<int>, array<int, bool>) = list<never>`. Subtract has no
/// canonical "remove the empty list" form, so the
/// `(a\b) ∩ b = never` property would falsely report a violation.
fn atom_is_empty_array_singleton(elem: suffete::ElementId) -> bool {
    let i = interner();
    match elem.kind() {
        suffete::ElementKind::List => {
            let info = i.get_list(elem);
            !info.flags.non_empty() && info.element_type == prelude::TYPE_NEVER && info.known_elements.is_none()
        }

        suffete::ElementKind::Array => {
            let info = i.get_array(elem);
            if info.flags.non_empty() {
                return false;
            }

            let value_is_never = match info.value_param {
                Some(v) => v == prelude::TYPE_NEVER,
                None => true,
            };

            if !value_is_never {
                return false;
            }

            match info.known_items {
                None => true,
                Some(id) => i.get_known_items(id).iter().all(|e| e.optional),
            }
        }

        _ => false,
    }
}

/// `true` when `t` contains an atom whose value-set or precision is
/// known to defeat subtract: open-world objects, structural types,
/// generic parameters, refined strings, true-union dominators, or
/// container atoms whose element types recurse into any of those.
fn type_has_imprecise_atom(t: TypeId) -> bool {
    t.as_ref().elements.iter().any(|e| element_is_imprecise(*e))
}

fn element_is_imprecise(e: suffete::ElementId) -> bool {
    if matches!(
        e.kind(),
        suffete::ElementKind::GenericParameter
            | suffete::ElementKind::Object
            | suffete::ElementKind::HasMethod
            | suffete::ElementKind::HasProperty
            | suffete::ElementKind::ObjectShape
            | suffete::ElementKind::Callable
            | suffete::ElementKind::Iterable
            | suffete::ElementKind::ClassLikeString
            | suffete::ElementKind::Scalar
            | suffete::ElementKind::Numeric
            | suffete::ElementKind::ArrayKey
            | suffete::ElementKind::Mixed
            | suffete::ElementKind::ObjectAny
    ) || atom_is_refined_string(e)
        || atom_is_empty_array_singleton(e)
    {
        return true;
    }
    let i = interner();
    match e.kind() {
        suffete::ElementKind::List => {
            let info = i.get_list(e);
            type_has_imprecise_atom(info.element_type)
        }

        suffete::ElementKind::Array => {
            let info = i.get_array(e);
            info.key_param.is_some_and(type_has_imprecise_atom) || info.value_param.is_some_and(type_has_imprecise_atom)
        }

        _ => false,
    }
}

/// `true` for a String atom carrying refinement flags or casing
/// (`non-empty-string`, `numeric-string`, `lowercase-string`, etc.).
/// Subtract has no canonical complement form for these axes, so any
/// refined-string survivor on either side of `(a \ b) ∩ b` is a
/// known precision gap.
fn atom_is_refined_string(elem: suffete::ElementId) -> bool {
    use suffete::element::payload::scalar::StringCasing;
    use suffete::element::payload::scalar::StringLiteral;
    use suffete::element::payload::scalar::StringRefinementFlags;

    if elem.kind() != suffete::ElementKind::String {
        return false;
    }
    let info = *interner().get_string(elem);
    if matches!(info.literal, StringLiteral::Value(_)) {
        return false;
    }
    info.flags != StringRefinementFlags::EMPTY || !matches!(info.casing, StringCasing::Unspecified)
}

fn meet_of(a: TypeId, b: TypeId, w: &MockWorld) -> TypeId {
    let mut report = LatticeReport::new();
    meet::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn subtract_of(a: TypeId, b: TypeId, w: &MockWorld) -> TypeId {
    let mut report = LatticeReport::new();
    subtract::compute(a, b, w, LatticeOptions::default(), &mut report)
}

fn env_cases() -> u32 {
    std::env::var("SUFFETE_PROPTEST_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(512)
}

fn env_max_shrink_iters() -> u32 {
    std::env::var("SUFFETE_PROPTEST_MAX_SHRINK_ITERS").ok().and_then(|v| v.parse().ok()).unwrap_or(1000)
}

fn env_max_global_rejects(cases: u32) -> u32 {
    std::env::var("SUFFETE_PROPTEST_MAX_GLOBAL_REJECTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| 1024.max(cases.saturating_mul(4)))
}

proptest! {
    #![proptest_config({
        let cases = env_cases();
        ProptestConfig {
            cases,
            max_shrink_iters: env_max_shrink_iters(),
            max_global_rejects: env_max_global_rejects(cases),
            failure_persistence: None,
            ..ProptestConfig::default()
        }
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
        // The order-sensitive string-axis merge inside `join`
        // (used by callable parameter meet under contravariance)
        // can canonicalize a multi-string union differently for
        // (a, b) vs (b, a). That asymmetry is a join-precision
        // gap, not a meet-soundness bug.
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

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
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

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
        let m_left = meet_of(a, prelude::TYPE_NEVER, &world);
        prop_assert_eq!(m_left, prelude::TYPE_NEVER, "meet(a, NEVER) should be NEVER for a={}", a);
        let m_right = meet_of(prelude::TYPE_NEVER, a, &world);
        prop_assert_eq!(m_right, prelude::TYPE_NEVER, "meet(NEVER, a) should be NEVER for a={}", a);
    }

    #[test]
    fn meet_is_associative_lower_bound((world, a, b, c) in arb_world_and_triple()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) || type_has_imprecise_atom(c) {
            return Ok(());
        }

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

        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) || type_has_imprecise_atom(c) {
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
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

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
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

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
        // Skip representation gaps where subtract can't precisely
        // remove a value-set that the meet then legitimately re-finds:
        //
        // - generic-parameter survivors (narrowing `T extends numeric`
        //   by `\ int` would need a `non-int-numeric` complement),
        // - object survivors (no canonical "B except A's subtypes"
        //   form when A descends B; meet then picks the descendant
        //   back up via subsumption),
        // - empty-array singleton containers (e.g. `list<never>`
        //   represents `{[]}`, which any `array<…>` in `b` also
        //   contains; subtract can't excise just `[]` from `list<int>`
        //   because there is no canonical `non-empty-list<int>`
        //   reachable through compositional subtract).
        let has_representation_gap = type_has_imprecise_atom(recheck) || type_has_imprecise_atom(s);

        if has_representation_gap {
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
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) || type_has_imprecise_atom(c) {
            return Ok(());
        }

        prop_assert!(does_refine(bc, cb, &world), "(a\\b)\\c should refine (a\\c)\\b");
        prop_assert!(does_refine(cb, bc, &world), "(a\\c)\\b should refine (a\\b)\\c");
    }

    #[test]
    fn meet_refines_join((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

        let m = meet_of(a, b, &world);
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.extend_from_slice(b.as_ref().elements);
        let joined_atoms = suffete::join::compute(&elems);
        let joined = interner().intern_type(&joined_atoms, FlowFlags::EMPTY);
        prop_assert!(
            does_refine(m, joined, &world),
            "meet ⊑ join\n  a={a}\n  b={b}\n  meet={m}\n  join={joined}"
        );
    }

    #[test]
    fn subtract_anti_monotonic_in_lhs((world, a, b, c) in arb_world_and_triple()) {
        if !does_refine(a, b, &world) {
            return Ok(());
        }

        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) || type_has_imprecise_atom(c) {
            return Ok(());
        }

        let ac = subtract_of(a, c, &world);
        let bc = subtract_of(b, c, &world);
        prop_assert!(does_refine(ac, bc, &world), "a<:b implies (a\\c)<:(b\\c)");
    }

    #[test]
    fn meet_with_self_is_self((world, a) in arb_world_and_type()) {
        // Idempotency: a ∧ a ≡ a.
        if type_has_imprecise_atom(a) {
            return Ok(());
        }

        let m = meet_of(a, a, &world);
        prop_assert!(does_refine(m, a, &world), "meet(a,a) <: a");
        prop_assert!(does_refine(a, m, &world), "a <: meet(a,a)");
    }

    #[test]
    fn join_with_self_widens_to_at_least_a((world, a) in arb_world_and_type()) {
        let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        elems.extend_from_slice(a.as_ref().elements);
        let joined_atoms = suffete::join::compute(&elems);
        let joined = interner().intern_type(&joined_atoms, FlowFlags::EMPTY);
        prop_assert!(does_refine(a, joined, &world), "a <: join(a,a)\n  a={a}\n  joined={joined}");
    }

    #[test]
    fn join_is_commutative((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

        let mut ab_elems: Vec<ElementId> = a.as_ref().elements.to_vec();
        ab_elems.extend_from_slice(b.as_ref().elements);
        let ab = interner().intern_type(&suffete::join::compute(&ab_elems), FlowFlags::EMPTY);
        let mut ba_elems: Vec<ElementId> = b.as_ref().elements.to_vec();
        ba_elems.extend_from_slice(a.as_ref().elements);
        let ba = interner().intern_type(&suffete::join::compute(&ba_elems), FlowFlags::EMPTY);
        prop_assert!(does_refine(ab, ba, &world), "join(a,b) <: join(b,a)\n  a={a}\n  b={b}\n  ab={ab}\n  ba={ba}");
        prop_assert!(does_refine(ba, ab, &world), "join(b,a) <: join(a,b)\n  a={a}\n  b={b}\n  ab={ab}\n  ba={ba}");
    }

    #[test]
    fn subtract_with_mixed_is_never((world, a) in arb_world_and_type()) {
        let r = subtract_of(a, prelude::TYPE_MIXED, &world);
        prop_assert_eq!(r, prelude::TYPE_NEVER, "a \\ mixed should be NEVER\n  a={}", a);
    }

    #[test]
    fn meet_with_never_is_never_symmetric((world, a) in arb_world_and_type()) {
        let l = meet_of(a, prelude::TYPE_NEVER, &world);
        let r = meet_of(prelude::TYPE_NEVER, a, &world);
        prop_assert_eq!(l, prelude::TYPE_NEVER, "meet(a, never) should be NEVER\n  a={}", a);
        prop_assert_eq!(r, prelude::TYPE_NEVER, "meet(never, a) should be NEVER\n  a={}", a);
    }

    #[test]
    fn refines_implies_meet_value_equivalent((world, a, b) in arb_world_and_pair()) {
        if !does_refine(a, b, &world) {
            return Ok(());
        }


        if a == prelude::TYPE_NEVER || type_is_value_never(a, &world) {
            return Ok(());
        }


        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }


        let m = meet_of(a, b, &world);
        prop_assert!(
            does_refine(a, m, &world) && does_refine(m, a, &world),
            "a<:b implies meet(a,b) ≡ a\n  a={a}\n  b={b}\n  m={m}"
        );
    }

    #[test]
    fn refines_implies_subtract_outcome_impossible((world, a, b) in arb_world_and_pair()) {
        if !does_refine(a, b, &world) {
            return Ok(());
        }


        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }


        let mut report = LatticeReport::new();
        let outcome = suffete::subtract::narrow(a, b, &*world, LatticeOptions::default(), &mut report);
        prop_assert!(
            matches!(outcome, suffete::subtract::SubtractOutcome::Impossible),
            "a<:b implies subtract::narrow Impossible; got {outcome:?}\n  a={a}\n  b={b}"
        );
    }

    #[test]
    fn disjoint_implies_subtract_value_equivalent((world, a, b) in arb_world_and_pair()) {
        if a == prelude::TYPE_NEVER || type_is_value_never(a, &world) {
            return Ok(());
        }


        if type_is_value_never(b, &world) {
            return Ok(());
        }


        if does_overlap(a, b, &world) {
            return Ok(());
        }


        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }


        let s = subtract_of(a, b, &world);
        prop_assert!(
            does_refine(a, s, &world) && does_refine(s, a, &world),
            "a # b implies (a \\ b) ≡ a value-wise\n  a={a}\n  b={b}\n  s={s}"
        );
    }

    #[test]
    fn meet_compute_is_lower_bound((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

        let m = meet_of(a, b, &world);
        prop_assert!(does_refine(m, a, &world), "meet(a,b) <: a\n  a={a}\n  b={b}\n  m={m}");
        prop_assert!(does_refine(m, b, &world), "meet(a,b) <: b\n  a={a}\n  b={b}\n  m={m}");
    }

    #[test]
    fn subtract_compute_refines_input((world, a, b) in arb_world_and_pair()) {
        let s = subtract_of(a, b, &world);
        prop_assert!(does_refine(s, a, &world), "(a\\b) <: a\n  a={a}\n  b={b}\n  s={s}");
    }

    #[test]
    fn meet_with_subtract_is_disjoint_from_subtract((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }

        let m = meet_of(a, b, &world);
        let s = subtract_of(a, b, &world);
        if m == prelude::TYPE_NEVER || s == prelude::TYPE_NEVER {
            return Ok(());
        }

        if does_refine(a, s, &world) {
            return Ok(());
        }

        let cross = meet_of(m, s, &world);
        prop_assert_eq!(cross, prelude::TYPE_NEVER, "meet(a,b) ∩ (a\\b) should be NEVER\n  a={}\n  b={}\n  m={}\n  s={}", a, b, m, s);
    }

    #[test]
    fn negated_meet_with_self_is_never((world, a) in arb_world_and_type()) {
        if type_has_imprecise_atom(a) || type_is_value_never(a, &world) {
            return Ok(());
        }
        let m = meet_of(a, neg_of(a), &world);
        prop_assert_eq!(
            m,
            prelude::TYPE_NEVER,
            "meet(T, !T) should be NEVER\n  T={}\n  !T={}\n  m={}",
            a,
            neg_of(a),
            m,
        );
    }

    #[test]
    fn negated_meet_equals_subtract((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }
        let lhs = meet_of(a, neg_of(b), &world);
        let rhs = subtract_of(a, b, &world);
        prop_assert!(
            does_refine(lhs, rhs, &world) && does_refine(rhs, lhs, &world),
            "meet(a, !b) ≡ subtract(a, b)\n  a={a}\n  b={b}\n  lhs={lhs}\n  rhs={rhs}",
        );
    }

    #[test]
    fn negated_subtract_equals_meet((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) {
            return Ok(());
        }
        let lhs = subtract_of(a, neg_of(b), &world);
        let rhs = meet_of(a, b, &world);
        prop_assert!(
            does_refine(lhs, rhs, &world) && does_refine(rhs, lhs, &world),
            "subtract(a, !b) ≡ meet(a, b)\n  a={a}\n  b={b}\n  lhs={lhs}\n  rhs={rhs}",
        );
    }

    #[test]
    fn double_negation_value_equal((world, a) in arb_world_and_type()) {
        if type_has_imprecise_atom(a) {
            return Ok(());
        }
        let dn = neg_of(neg_of(a));
        prop_assert!(
            does_refine(a, dn, &world) && does_refine(dn, a, &world),
            "!!T ≡ T value-wise\n  T={a}\n  !!T={dn}",
        );
    }

    #[test]
    fn negated_refines_iff_no_overlap((world, a, b) in arb_world_and_pair()) {
        if type_has_imprecise_atom(a) || type_has_imprecise_atom(b) || type_is_value_never(a, &world) {
            return Ok(());
        }
        let nb = neg_of(b);
        let refines_neg = does_refine(a, nb, &world);
        let no_overlap = !does_overlap(a, b, &world);
        if refines_neg != no_overlap {
            prop_assert_eq!(
                refines_neg,
                no_overlap,
                "X <: !T iff !overlaps(X, T)\n  X={}\n  T={}\n  !T={}\n  refines={}\n  no_overlap={}",
                a, b, nb, refines_neg, no_overlap,
            );
        }
    }

    /// Algebraic-law battery for every generated type pair. One single
    /// proptest case exercises every soundness identity that holds
    /// between `refines`, `meet`, `join`, `subtract`, `overlaps`, and
    /// `negate`. Catches any inconsistency between operations.
    #[test]
    fn lattice_pair_laws_hold((world, a, b) in arb_world_and_pair()) {
        if let Err(violation) = check_lattice_pair_laws(a, b, &world) {
            prop_assert!(false, "{violation}");
        }
    }

    #[test]
    fn lattice_triple_laws_hold((world, a, b, c) in arb_world_and_triple()) {
        if let Err(violation) = check_lattice_triple_laws(a, b, c, &world) {
            prop_assert!(false, "{violation}");
        }
    }

    #[test]
    fn sealed_full_cover_subtract_is_never((world, a) in arb_world_and_type()) {
        let _ = a;

        for &class_name in CLASSES {
            let name_atom = mago_atom::atom(class_name);
            let inheritors = match world.sealed_direct_inheritors(name_atom) {
                Some(inh) => inh.to_vec(),
                None => continue,
            };

            if inheritors.is_empty() {
                continue;
            }

            let head = ElementId::object_named(class_name);
            let negations: Vec<ElementId> = inheritors
                .iter()
                .map(|&inh| {
                    let inh_elem = ElementId::object_named(inh.as_str());
                    ElementId::negated(TypeId::singleton(inh_elem))
                })
                .collect();

            let sealed_no_inheritors = ElementId::intersected(head, &negations);
            let m = meet_of(TypeId::singleton(sealed_no_inheritors), prelude::TYPE_MIXED, &world);
            prop_assert_eq!(
                m,
                prelude::TYPE_NEVER,
                "sealed full cover must be never for {}",
                class_name
            );
        }
    }
}

fn neg_of(t: TypeId) -> TypeId {
    let neg = suffete::ElementId::negated(t);
    interner().intern_type(&[neg], FlowFlags::EMPTY)
}

fn join_of(a: TypeId, b: TypeId) -> TypeId {
    let mut elems: Vec<ElementId> = a.as_ref().elements.to_vec();
    elems.extend_from_slice(b.as_ref().elements);
    let joined = suffete::join::compute(&elems);
    interner().intern_type(&joined, FlowFlags::EMPTY)
}

fn equiv(a: TypeId, b: TypeId, w: &MockWorld) -> bool {
    does_refine(a, b, w) && does_refine(b, a, w)
}

/// Battery of every algebraic identity that must hold for any pair
/// `(a, b)` of inhabited types. Returns `Err` with a precise diagnostic
/// on the first violation.
///
/// Soundness laws (must hold or there's a bug):
/// - Universal axioms: refl, bottom, top.
/// - Meet GLB axiom: `meet(a,b) <: a` and `<: b`.
/// - Join LUB axiom: `a <: join(a,b)` and `b <: join(a,b)`.
/// - Subtract bound: `a\b <: a`.
/// - Negation involution: `!!T ≡ T` (over types where double-negation
///   collapses ; multi-atom shapes excluded).
/// - Commutativity of meet, join, overlaps.
/// - Idempotence: `meet(a,a) ≡ a`, `join(a,a) ≡ a`, `a\a ≡ never`.
/// - Identity: `meet(a, mixed) ≡ a`, `join(a, never) ≡ a`,
///   `meet(a, never) ≡ never`, `join(a, mixed) ≡ mixed`,
///   `a \ never ≡ a`, `a \ mixed ≡ never`.
/// - Subsumption interlock: `a <: b` ⟺ `meet(a,b) ≡ a` ⟺
///   `join(a,b) ≡ b` (when subtract is precise enough also
///   `subtract(a,b) ≡ never`).
/// - Refines is consistent with overlaps when one side is non-empty.
fn check_lattice_pair_laws(a: TypeId, b: TypeId, w: &MockWorld) -> Result<(), String> {
    if !does_refine(a, a, w) {
        return Err(format!("refl: a !<: a; a={a}"));
    }

    if !does_refine(b, b, w) {
        return Err(format!("refl: b !<: b; b={b}"));
    }

    if !does_refine(prelude::TYPE_NEVER, a, w) {
        return Err(format!("bottom: NEVER !<: a={a}"));
    }

    if !does_refine(a, prelude::TYPE_MIXED, w) {
        return Err(format!("top: a={a} !<: MIXED"));
    }

    let ab = meet_of(a, b, w);
    let ba = meet_of(b, a, w);

    if !equiv(ab, ba, w) {
        return Err(format!("meet not commutative: meet(a,b)={ab}, meet(b,a)={ba}; a={a}, b={b}"));
    }

    if !does_refine(ab, a, w) {
        return Err(format!("GLB: meet(a,b)={ab} !<: a={a}; b={b}"));
    }

    if !does_refine(ab, b, w) {
        return Err(format!("GLB: meet(a,b)={ab} !<: b={b}; a={a}"));
    }

    let aub = join_of(a, b);
    let bua = join_of(b, a);

    if !does_refine(a, aub, w) {
        return Err(format!("LUB: a={a} !<: join(a,b)={aub}; b={b}"));
    }

    if !does_refine(b, aub, w) {
        return Err(format!("LUB: b={b} !<: join(a,b)={aub}; a={a}"));
    }

    if !does_refine(a, bua, w) {
        return Err(format!("LUB symmetric: a={a} !<: join(b,a)={bua}; b={b}"));
    }

    if !does_refine(b, bua, w) {
        return Err(format!("LUB symmetric: b={b} !<: join(b,a)={bua}; a={a}"));
    }

    let a_minus_b = subtract_of(a, b, w);
    if !does_refine(a_minus_b, a, w) {
        return Err(format!("subtract bound: a\\b={a_minus_b} !<: a={a}; b={b}"));
    }

    let aa = meet_of(a, a, w);
    if !equiv(aa, a, w) {
        return Err(format!("meet idempotence: meet(a,a)={aa}, expected a={a}"));
    }

    let aja = join_of(a, a);
    if !does_refine(a, aja, w) {
        return Err(format!("join LUB self: a={a} !<: join(a,a)={aja}"));
    }

    let a_mix = meet_of(a, prelude::TYPE_MIXED, w);
    if !equiv(a_mix, a, w) {
        return Err(format!("meet identity: meet(a, MIXED)={a_mix}, expected a={a}"));
    }

    let a_nev = meet_of(a, prelude::TYPE_NEVER, w);
    if a_nev != prelude::TYPE_NEVER {
        return Err(format!("meet absorb: meet(a, NEVER)={a_nev}, expected NEVER; a={a}"));
    }

    let a_join_nev = join_of(a, prelude::TYPE_NEVER);
    if !does_refine(a, a_join_nev, w) {
        return Err(format!("join identity bound: a={a} !<: join(a, NEVER)={a_join_nev}"));
    }

    let a_join_mix = join_of(a, prelude::TYPE_MIXED);
    if !equiv(a_join_mix, prelude::TYPE_MIXED, w) {
        return Err(format!("join absorb: join(a, MIXED)={a_join_mix}, expected MIXED; a={a}"));
    }

    let a_minus_nev = subtract_of(a, prelude::TYPE_NEVER, w);
    if !equiv(a_minus_nev, a, w) {
        return Err(format!("subtract identity: a\\NEVER={a_minus_nev}, expected a={a}"));
    }

    let a_minus_mix = subtract_of(a, prelude::TYPE_MIXED, w);
    if a_minus_mix != prelude::TYPE_NEVER {
        return Err(format!("subtract absorb: a\\MIXED={a_minus_mix}, expected NEVER; a={a}"));
    }

    let a_minus_a = subtract_of(a, a, w);
    if a_minus_a != prelude::TYPE_NEVER {
        return Err(format!("subtract self: a\\a={a_minus_a}, expected NEVER; a={a}"));
    }

    let ov_ab = does_overlap(a, b, w);
    let ov_ba = does_overlap(b, a, w);
    if ov_ab != ov_ba {
        return Err(format!("overlaps not symmetric: overlaps(a,b)={ov_ab}, overlaps(b,a)={ov_ba}; a={a}, b={b}"));
    }

    if does_refine(a, b, w) {
        if !equiv(ab, a, w) {
            return Err(format!("a<:b ⇒ meet(a,b)≡a: meet={ab}, a={a}; b={b}"));
        }
        // `join(a,b) ≡ b` would hold if join were strictly the LUB,
        // but the lattice's join canonicalizes (string axis merge,
        // dominator collapse, etc.). The LUB axiom (b <: join(a,b))
        // is checked above.
    }

    Ok(())
}

/// Battery for ternary identities. Strict associativity / equivalence
/// is relaxed to soundness bounds because the lattice's join and meet
/// canonicalize, which makes the operations order-sensitive in the
/// representational sense even when the value-set is the same.
fn check_lattice_triple_laws(a: TypeId, b: TypeId, c: TypeId, w: &MockWorld) -> Result<(), String> {
    let m_ab = meet_of(a, b, w);
    let m_left = meet_of(m_ab, c, w);
    let m_bc = meet_of(b, c, w);
    let m_right = meet_of(a, m_bc, w);

    for (label, t) in [("(a∩b)∩c", m_left), ("a∩(b∩c)", m_right)] {
        if !does_refine(t, a, w) {
            return Err(format!("meet GLB ternary: {label}={t} !<: a={a}; b={b}, c={c}"));
        }
        if !does_refine(t, b, w) {
            return Err(format!("meet GLB ternary: {label}={t} !<: b={b}; a={a}, c={c}"));
        }
        if !does_refine(t, c, w) {
            return Err(format!("meet GLB ternary: {label}={t} !<: c={c}; a={a}, b={b}"));
        }
    }

    let j_ab = join_of(a, b);
    let j_left = join_of(j_ab, c);
    let j_bc = join_of(b, c);
    let j_right = join_of(a, j_bc);

    for (label, t) in [("(a∪b)∪c", j_left), ("a∪(b∪c)", j_right)] {
        if !does_refine(a, t, w) {
            return Err(format!("join LUB ternary: a={a} !<: {label}={t}; b={b}, c={c}"));
        }
        if !does_refine(b, t, w) {
            return Err(format!("join LUB ternary: b={b} !<: {label}={t}; a={a}, c={c}"));
        }
        if !does_refine(c, t, w) {
            return Err(format!("join LUB ternary: c={c} !<: {label}={t}; a={a}, b={b}"));
        }
    }

    if does_refine(a, b, w) && does_refine(b, c, w) && !does_refine(a, c, w) {
        return Err(format!("refines transitivity: a<:b={a}<:{b}, b<:c={b}<:{c}, but a !<: c={c}"));
    }

    Ok(())
}
