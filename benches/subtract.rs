//! Benchmarks for [`suffete::subtract`] public APIs: `compute` and `narrow`.

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

use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::subtract;

mod common;
use common::*;

fn pairs() -> Vec<(&'static str, suffete::TypeId, suffete::TypeId)> {
    vec![
        ("identity", t_int(), t_int()),
        ("disjoint", t_int(), t_string()),
        ("complete", t_mixed(), t_int()),
        ("partial_range_split", suffete::TypeId::int_range(Some(0), Some(100)), suffete::TypeId::int_literal(50)),
        ("singleton_minus_union", tiny_singleton(), small_union()),
        ("wide_minus_singleton", wide_union(), tiny_singleton()),
    ]
}

fn bench_compute(c: &mut Criterion) {
    let mut g = c.benchmark_group("subtract::compute");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                subtract::compute(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

fn bench_narrow(c: &mut Criterion) {
    let mut g = c.benchmark_group("subtract::narrow");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                subtract::narrow(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

criterion_group!(benches, bench_compute, bench_narrow);
criterion_main!(benches);
