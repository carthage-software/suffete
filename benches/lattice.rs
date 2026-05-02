//! Benchmarks for [`suffete::lattice`] public APIs:
//! `refines`, `generalizes`, `overlaps`.
//!
//! Coverage matrix: combinations of standard input shapes against each
//! other so we track every relevant pair (singleton-vs-singleton through
//! wide-vs-wide).

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
use suffete::lattice::generalizes;
use suffete::lattice::overlaps;
use suffete::lattice::refines;

mod common;
use common::*;

fn pairs() -> Vec<(&'static str, suffete::TypeId, suffete::TypeId)> {
    vec![
        ("singleton_vs_singleton", tiny_singleton(), tiny_singleton()),
        ("singleton_vs_mixed", tiny_singleton(), t_mixed()),
        ("mixed_vs_singleton", t_mixed(), tiny_singleton()),
        ("singleton_vs_small_union", tiny_singleton(), small_union()),
        ("small_vs_wide", small_union(), wide_union()),
        ("wide_vs_wide", wide_union(), wide_union()),
        ("deep_vs_deep", deep_nested(), deep_nested()),
        ("disjoint", t_int(), t_string()),
        ("subsumed", t_int(), t_scalar()),
    ]
}

fn bench_refines(c: &mut Criterion) {
    let mut g = c.benchmark_group("lattice::refines");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                refines(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

fn bench_generalizes(c: &mut Criterion) {
    let mut g = c.benchmark_group("lattice::generalizes");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                generalizes(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

fn bench_overlaps(c: &mut Criterion) {
    let mut g = c.benchmark_group("lattice::overlaps");
    let world = bench_world();
    let opts = LatticeOptions::default();

    for (label, a, b) in pairs() {
        g.bench_function(label, |bencher| {
            bencher.iter(|| {
                let mut report = LatticeReport::default();
                overlaps(black_box(a), black_box(b), &world, opts, &mut report)
            });
        });
    }

    g.finish();
}

criterion_group!(benches, bench_refines, bench_generalizes, bench_overlaps);
criterion_main!(benches);
