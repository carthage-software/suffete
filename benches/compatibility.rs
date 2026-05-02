//! Benchmarks for [`suffete::compatibility`] public APIs:
//! `statically_compatible` and `runtime_compatible`.

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

use suffete::compatibility;
use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;

mod common;
use common::*;

fn pairs() -> Vec<(&'static str, suffete::TypeId, suffete::TypeId)> {
    vec![
        ("identical", t_int(), t_int()),
        ("disjoint", t_int(), t_string()),
        ("partial_overlap", small_union(), wide_union()),
        ("singleton_vs_mixed", tiny_singleton(), t_mixed()),
        ("nested_lists", deep_nested(), deep_nested()),
    ]
}

fn bench_statically(c: &mut Criterion) {
    let mut g = c.benchmark_group("compatibility::statically_compatible");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                compatibility::statically_compatible(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

fn bench_runtime(c: &mut Criterion) {
    let mut g = c.benchmark_group("compatibility::runtime_compatible");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                compatibility::runtime_compatible(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

criterion_group!(benches, bench_statically, bench_runtime);
criterion_main!(benches);
