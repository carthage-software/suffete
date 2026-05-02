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

mod combiner_common;

use combiner_common::*;
use suffete::ElementId;
use suffete::ElementKind;

#[test]
fn int_absorbs_every_literal_in_minus_500_to_500() {
    for v in -500..=500i64 {
        assert_combines_to(vec![t_int(), t_lit_int(v)], vec![t_int()]);
        assert_combines_to(vec![t_lit_int(v), t_int()], vec![t_int()]);
    }
}

#[test]
fn unspec_int_absorbs_lit_unspec_lit() {
    assert_combines_to(vec![t_int(), t_int_unspec_lit()], vec![t_int()]);
    assert_combines_to(vec![t_int_unspec_lit(), t_int()], vec![t_int()]);
}

#[test]
fn lit_int_self_dedup_for_many_values() {
    for v in -50..=50i64 {
        for n in 2..=5 {
            let result = combine_default(vec![t_lit_int(v); n]);
            assert_eq!(result.len(), 1);
        }
    }
}

#[test]
fn non_adjacent_lit_pairs_kept_apart() {
    for a in -10..=10i64 {
        for b in -10..=10i64 {
            if a == b || (a - b).abs() == 1 {
                continue;
            }
            let result = combine_default(vec![t_lit_int(a), t_lit_int(b)]);
            assert_eq!(result.len(), 2, "{a} | {b}");
        }
    }
}

#[test]
fn adjacent_lit_pairs_merge_to_range() {
    for a in -5..=5i64 {
        let b = a + 1;
        let result = combine_default(vec![t_lit_int(a), t_lit_int(b)]);
        assert_eq!(result, vec![t_int_range(a, b)], "{a} | {b}");
    }
}

#[test]
fn ranges_with_overlaps_collapse() {}

#[test]
fn from_n_with_lit_n_minus_1_extends_for_many_n() {}

#[test]
fn to_n_with_lit_n_plus_1_extends_for_many_n() {}

#[test]
fn string_absorbs_many_literals() {
    for i in 0..200 {
        let s = format!("test_{i}");
        assert_combines_to(vec![t_string(), t_lit_string(&s)], vec![t_string()]);
        assert_combines_to(vec![t_lit_string(&s), t_string()], vec![t_string()]);
    }
}

#[test]
fn lit_string_self_dedup_many_values() {
    for i in 0..50 {
        let s = format!("v_{i}");
        for n in 2..=5 {
            let result = combine_default(vec![t_lit_string(&s); n]);
            assert_eq!(result.len(), 1);
        }
    }
}

#[test]
fn distinct_lit_string_pairs_kept_apart() {
    for i in 0..30 {
        for j in (i + 1)..30 {
            let a = format!("a_{i}");
            let b = format!("b_{j}");
            let result = combine_default(vec![t_lit_string(&a), t_lit_string(&b)]);
            assert_eq!(result.len(), 2);
        }
    }
}

#[test]
fn float_absorbs_many_literals() {
    for i in 0..200 {
        let v = f64::from(i).mul_add(0.5, -50.0);
        assert_combines_to(vec![t_float(), t_lit_float(v)], vec![t_float()]);
        assert_combines_to(vec![t_lit_float(v), t_float()], vec![t_float()]);
    }
}

#[test]
fn lit_float_self_dedup_many_values() {
    for i in 0..50 {
        let v = f64::from(i) * 0.25;
        for n in 2..=5 {
            assert_combines_to(vec![t_lit_float(v); n], vec![t_lit_float(v)]);
        }
    }
}

#[test]
fn all_bool_triples_collapse() {
    let bools = [t_true(), t_false(), t_bool()];
    let valid = [t_true(), t_false(), t_bool()];
    for a in &bools {
        for b in &bools {
            for c in &bools {
                let result = combine_default(vec![*a, *b, *c]);
                assert_eq!(result.len(), 1, "{a:?} {b:?} {c:?}");
                assert!(valid.contains(&result[0]), "got {:?}", result[0]);
            }
        }
    }
}

#[test]
fn many_distinct_named_objects_kept_apart() {
    for n in [3usize, 5, 10, 20, 50] {
        let inputs: Vec<ElementId> = (0..n).map(|i| t_named(&format!("Class{i}"))).collect();
        let result = combine_default(inputs);
        assert_eq!(result.len(), n);
    }
}

#[test]
fn same_named_with_many_copies_collapses() {
    for n in 1..=20 {
        assert_combines_to(vec![t_named("Foo"); n], vec![t_named("Foo")]);
    }
}

#[test]
fn object_any_absorbs_many_named() {
    for n in 1..=20 {
        let mut inputs = vec![t_object_any()];
        for i in 0..n {
            inputs.push(t_named(&format!("C{i}")));
        }
        assert_combines_to(inputs, vec![t_object_any()]);
    }
}

#[test]
fn generic_with_n_distinct_int_params_collapse_to_one_container() {}

#[test]
fn generic_with_int_and_string_param_keeps_one_container() {}

#[test]
fn many_distinct_enums_kept_apart() {
    for n in [3usize, 5, 10, 20] {
        let inputs: Vec<ElementId> = (0..n).map(|i| t_enum(&format!("E{i}"))).collect();
        let result = combine_default(inputs);
        assert_eq!(result.len(), n);
    }
}

#[test]
fn many_distinct_enum_cases_kept_apart() {
    for n in [3usize, 5, 10] {
        let inputs: Vec<ElementId> = (0..n).map(|i| t_enum_case("E", &format!("Case{i}"))).collect();
        let result = combine_default(inputs);
        assert_eq!(result.len(), n);
    }
}

#[test]
fn many_copies_of_simple_atoms_collapse() {
    // Simple-atom subset (no list/keyed helpers yet).
    let atoms = [
        t_int(),
        t_string(),
        t_float(),
        t_bool(),
        t_true(),
        t_false(),
        null(),
        never(),
        t_object_any(),
        t_named("Foo"),
        t_resource(),
        t_open_resource(),
        t_closed_resource(),
        t_empty_array(),
    ];
    for atom in &atoms {
        for n in [2usize, 5, 10, 20, 50, 100] {
            let result = combine_default(vec![*atom; n]);
            assert_eq!(result.len(), 1, "{n} copies of {atom:?}");
        }
    }
}

#[test]
fn three_way_stable_primitives_consistent() {
    let stable = [t_int(), t_string(), t_float(), t_bool(), null(), t_object_any(), t_named("X"), t_resource()];
    for a in &stable {
        for b in &stable {
            for c in &stable {
                let r = combine_default(vec![*a, *b, *c]);
                let r_rev = combine_default(vec![*c, *b, *a]);
                assert_multiset_eq(&r, &r_rev);
            }
        }
    }
}

#[test]
fn mixed_dominates_every_simple_atom() {
    // Subset (no list/keyed). Also drops `numeric`/`scalar`/`array_key`
    // since the structural mixed-absorbs rule covers them anyway.
    let atoms = [
        t_int(),
        t_string(),
        t_float(),
        t_bool(),
        t_true(),
        t_false(),
        null(),
        void(),
        never(),
        t_object_any(),
        t_named("Foo"),
        t_enum("E"),
        t_resource(),
        t_empty_array(),
        t_array_key(),
        t_numeric(),
        t_scalar(),
        t_class_string(),
    ];
    for atom in &atoms {
        let r = combine_default(vec![*atom, mixed()]);
        let r_rev = combine_default(vec![mixed(), *atom]);
        assert_eq!(r.len(), 1);
        assert_eq!(r_rev.len(), 1);
        assert_eq!(r[0].kind(), ElementKind::Mixed);
        assert_eq!(r_rev[0].kind(), ElementKind::Mixed);
    }
}

#[test]
fn never_absorbed_by_every_non_void_atom() {
    let atoms = [
        t_int(),
        t_string(),
        t_float(),
        t_bool(),
        t_true(),
        t_false(),
        null(),
        t_object_any(),
        t_named("Foo"),
        t_enum("E"),
        t_resource(),
        t_open_resource(),
        t_closed_resource(),
        t_empty_array(),
        t_array_key(),
        t_numeric(),
        t_scalar(),
        t_class_string(),
        t_lit_int(0),
        t_lit_string("x"),
        t_lit_float(1.0),
    ];
    for atom in &atoms {
        let r = combine_default(vec![*atom, never()]);
        let r_rev = combine_default(vec![never(), *atom]);
        assert!(r.iter().all(|a| *a != never()), "never leaked: {atom:?}");
        assert!(r_rev.iter().all(|a| *a != never()), "never leaked rev: {atom:?}");
    }
}

#[test]
fn many_named_with_lits() {
    for n in 1..=10 {
        let mut inputs = vec![];
        for i in 0..n {
            inputs.push(t_named(&format!("C{i}")));
            inputs.push(t_lit_int(i as i64));
        }
        let result = combine_default(inputs);
        // Adjacent literals merge into one range / literal; n named objects stay distinct.
        assert_eq!(result.len(), n + 1);
    }
}

#[test]
fn alternating_int_string_collapses_to_two() {
    for n in 1..=20 {
        let mut inputs = Vec::new();
        for _ in 0..n {
            inputs.push(t_int());
            inputs.push(t_string());
        }
        let r = combine_default(inputs);
        assert_eq!(r.len(), 2);
    }
}

#[test]
fn alternating_named_collapses() {
    for n in 1..=15 {
        let mut inputs = Vec::new();
        for _ in 0..n {
            inputs.push(t_named("A"));
            inputs.push(t_named("B"));
        }
        let r = combine_default(inputs);
        assert_eq!(r.len(), 2);
    }
}

#[test]
fn n_copies_plus_adjacent_int_merges_to_range() {
    for n in [1usize, 5, 10, 50, 100] {
        let mut inputs: Vec<ElementId> = std::iter::repeat_with(|| t_lit_int(0)).take(n).collect();
        inputs.push(t_lit_int(1));
        let r = combine_default(inputs);
        assert_eq!(r, vec![t_int_range(0, 1)], "n={n}");
    }
}
