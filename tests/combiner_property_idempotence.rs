#![allow(clippy::vec_init_then_push)]

mod combiner_common;

use combiner_common::*;
use suffete::ElementId;
use suffete::ElementKind;

/// A reduced atom zoo covering every element family currently exposed by
/// suffete's combiner. The original mago zoo also carries generic objects,
/// list/keyed-array shapes, and sealed-list variants; those are skipped
/// here pending the corresponding helpers / combiner support.
fn full_atom_zoo() -> Vec<ElementId> {
    let mut atoms = Vec::new();

    atoms.push(t_bool());
    atoms.push(t_true());
    atoms.push(t_false());

    atoms.push(t_int());
    atoms.push(t_int_unspec_lit());
    atoms.push(t_positive_int());
    atoms.push(t_negative_int());
    atoms.push(t_non_negative_int());
    atoms.push(t_non_positive_int());
    for v in [-1000_i64, -100, -10, -1, 0, 1, 10, 100, 1000] {
        atoms.push(t_lit_int(v));
    }
    for from in [-100_i64, -1, 0, 1, 100] {
        atoms.push(t_int_from(from));
    }
    for to in [-100_i64, -1, 0, 1, 100] {
        atoms.push(t_int_to(to));
    }
    for (lo, hi) in [(-50_i64, 50), (0, 100), (-100, 0), (-10, 10)] {
        atoms.push(t_int_range(lo, hi));
    }

    atoms.push(t_float());
    atoms.push(t_unspec_lit_float());
    for v in [-100.0_f64, -1.0, 0.0, 1.0, 1.5, 100.0] {
        atoms.push(t_lit_float(v));
    }

    atoms.push(t_string());
    atoms.push(t_non_empty_string());
    atoms.push(t_numeric_string());
    atoms.push(t_lower_string());
    atoms.push(t_upper_string());
    atoms.push(t_truthy_string());
    atoms.push(t_callable_string());
    atoms.push(t_unspec_lit_string(false));
    atoms.push(t_unspec_lit_string(true));
    for s in ["", "hi", "0", "Hello", "HELLO", "hello world", "123"] {
        atoms.push(t_lit_string(s));
    }

    atoms.push(t_class_string());
    atoms.push(t_interface_string());
    atoms.push(t_enum_string());
    atoms.push(t_trait_string());
    for n in ["Foo", "Bar", "App\\Service"] {
        atoms.push(t_lit_class_string(n));
    }

    atoms.push(t_array_key());
    atoms.push(t_numeric());
    atoms.push(t_scalar());

    atoms.push(null());
    atoms.push(void());
    atoms.push(never());

    atoms.push(mixed());

    atoms.push(t_resource());
    atoms.push(t_open_resource());
    atoms.push(t_closed_resource());

    atoms.push(t_object_any());
    for n in ["Foo", "App\\Bar", "X\\Y\\Z"] {
        atoms.push(t_named(n));
    }
    for n in ["E", "MyEnum", "Status"] {
        atoms.push(t_enum(n));
    }
    for (n, c) in [("E", "A"), ("Status", "Active"), ("Color", "Red")] {
        atoms.push(t_enum_case(n, c));
    }

    atoms.push(t_empty_array());

    atoms
}

fn is_mixed(e: ElementId) -> bool {
    e.kind() == ElementKind::Mixed
}

#[test]
fn singleton_passthrough_for_full_zoo() {
    for atom in full_atom_zoo() {
        let r = combine_default(vec![atom]);
        assert_eq!(r.len(), 1, "singleton broke for {atom:?}");
        assert_eq!(r[0], atom, "singleton id changed for {atom:?}");
    }
}

#[test]
fn self_idempotency_basic() {
    for atom in full_atom_zoo() {
        for n in [2_usize, 3, 5, 10] {
            let r = combine_default(vec![atom; n]);
            assert_eq!(r.len(), 1, "self-idempotency broke for {atom:?} (n={n})");
            assert_eq!(r[0], atom);
        }
    }
}

#[test]
fn double_input_matches_single_for_zoo() {
    for atom in full_atom_zoo() {
        let single = combine_default(vec![atom]);
        let double = combine_default(vec![atom, atom]);
        assert_multiset_eq(&single, &double);
    }
}

#[test]
fn never_is_absorbed_by_every_non_void_zoo_atom() {
    for atom in full_atom_zoo() {
        if atom == never() || atom == void() {
            continue;
        }
        let r1 = combine_default(vec![atom, never()]);
        let r2 = combine_default(vec![never(), atom]);
        assert!(r1.iter().all(|a| *a != never()), "never leaked through with {atom:?}");
        assert!(r2.iter().all(|a| *a != never()), "never leaked through (rev) with {atom:?}");
    }
}

#[test]
fn mixed_dominates_every_zoo_atom() {
    for atom in full_atom_zoo() {
        let r1 = combine_default(vec![atom, mixed()]);
        let r2 = combine_default(vec![mixed(), atom]);
        assert_eq!(r1.len(), 1, "mixed didn't dominate {atom:?} (forward)");
        assert_eq!(r2.len(), 1, "mixed didn't dominate {atom:?} (reverse)");
        assert!(is_mixed(r1[0]), "{atom:?} | mixed didn't yield Mixed");
        assert!(is_mixed(r2[0]), "{atom:?} | mixed didn't yield Mixed (rev)");
    }
}

#[test]
fn lit_int_absorbed_by_int_for_many_values() {
    let values: Vec<i64> = (-50..50).collect();
    for v in values {
        assert_combines_to(vec![t_int(), t_lit_int(v)], vec![t_int()]);
        assert_combines_to(vec![t_lit_int(v), t_int()], vec![t_int()]);
    }
}

#[test]
fn lit_string_absorbed_by_string_for_many_values() {
    let strings: Vec<String> = (0..50).map(|i| format!("test_{i}")).collect();
    for s in &strings {
        assert_combines_to(vec![t_string(), t_lit_string(s)], vec![t_string()]);
        assert_combines_to(vec![t_lit_string(s), t_string()], vec![t_string()]);
    }
}

#[test]
fn lit_float_absorbed_by_float_for_many_values() {
    for i in 0..30 {
        let v = f64::from(i) * 0.5;
        assert_combines_to(vec![t_float(), t_lit_float(v)], vec![t_float()]);
        assert_combines_to(vec![t_lit_float(v), t_float()], vec![t_float()]);
    }
}

#[test]
fn order_independence_for_non_asymmetric_pairs() {
    let stable = vec![
        t_int(),
        t_string(),
        t_float(),
        t_bool(),
        t_named("Foo"),
        t_named("Bar"),
        t_object_any(),
        t_resource(),
        t_open_resource(),
        t_closed_resource(),
        null(),
    ];

    for (i, a) in stable.iter().enumerate() {
        for b in &stable[i..] {
            let ab = combine_default(vec![*a, *b]);
            let ba = combine_default(vec![*b, *a]);
            assert_multiset_eq(&ab, &ba);
        }
    }
}
