//! Benchmarks for [`suffete::meet`] public APIs: `compute` and `narrow`.

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
use suffete::meet;

mod common;
use common::*;

fn pairs() -> Vec<(&'static str, suffete::TypeId, suffete::TypeId)> {
    vec![
        ("identity", t_int(), t_int()),
        ("disjoint", t_int(), t_string()),
        ("singleton_vs_mixed", tiny_singleton(), t_mixed()),
        ("small_vs_wide", small_union(), wide_union()),
        ("wide_vs_singleton", wide_union(), tiny_singleton()),
        ("deep_vs_deep", deep_nested(), deep_nested()),
        ("subsumed", t_int(), t_scalar()),
    ]
}

fn bench_compute(c: &mut Criterion) {
    let mut g = c.benchmark_group("meet::compute");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                meet::compute(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

fn bench_narrow(c: &mut Criterion) {
    let mut g = c.benchmark_group("meet::narrow");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                meet::narrow(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

criterion_group!(benches, bench_compute, bench_narrow);
criterion_main!(benches);
