//! Benchmarks for [`suffete::predicates`] public APIs.
//!
//! Each predicate is run against the standard input shapes (tiny, small
//! union, wide union, deeply nested, keyed-array payload, named object)
//! so regressions in any one shape surface immediately.

#![allow(
    clippy::significant_drop_tightening,
    clippy::wildcard_imports,
    clippy::missing_const_for_fn,
    clippy::separated_literal_suffix,
    clippy::doc_markdown,
    clippy::redundant_closure,
    clippy::arithmetic_side_effects,
    clippy::missing_docs_in_private_items,
    clippy::absolute_paths,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core
)]

use core::hint::black_box;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;

use suffete::predicates;

mod common;
use common::standard_inputs;

macro_rules! bench_predicate {
    ($g:expr, $name:literal, $func:path, $inputs:expr) => {
        for (label, ty) in $inputs.iter().copied() {
            $g.bench_function(format!("{}_{}", $name, label), |b| {
                b.iter(|| $func(black_box(ty)));
            });
        }
    };
}

fn bench_predicates(c: &mut Criterion) {
    let mut g = c.benchmark_group("predicates");
    let inputs = standard_inputs();

    bench_predicate!(g, "is_never", predicates::is_never, inputs);
    bench_predicate!(g, "is_mixed", predicates::is_mixed, inputs);
    bench_predicate!(g, "is_singleton", predicates::is_singleton, inputs);
    bench_predicate!(g, "is_union", predicates::is_union, inputs);
    bench_predicate!(g, "is_int", predicates::is_int, inputs);
    bench_predicate!(g, "is_float", predicates::is_float, inputs);
    bench_predicate!(g, "is_string", predicates::is_string, inputs);
    bench_predicate!(g, "is_bool", predicates::is_bool, inputs);
    bench_predicate!(g, "is_array", predicates::is_array, inputs);
    bench_predicate!(g, "is_object", predicates::is_object, inputs);
    bench_predicate!(g, "is_truthy", predicates::is_truthy, inputs);
    bench_predicate!(g, "is_falsy", predicates::is_falsy, inputs);
    bench_predicate!(g, "could_be_truthy", predicates::could_be_truthy, inputs);
    bench_predicate!(g, "could_be_falsy", predicates::could_be_falsy, inputs);
    bench_predicate!(g, "is_literal", predicates::is_literal, inputs);
    bench_predicate!(g, "is_constant_foldable", predicates::is_constant_foldable, inputs);
    bench_predicate!(g, "contains_mixed_anywhere", predicates::contains_mixed_anywhere, inputs);
    bench_predicate!(g, "contains_template_anywhere", predicates::contains_template_anywhere, inputs);
    bench_predicate!(g, "contains_unresolved_anywhere", predicates::contains_unresolved_anywhere, inputs);
    bench_predicate!(g, "is_fully_resolved", predicates::is_fully_resolved, inputs);

    g.finish();
}

criterion_group!(benches, bench_predicates);
criterion_main!(benches);
