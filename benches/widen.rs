//! Benchmarks for [`suffete::widen`] public APIs:
//! `scalars` and `literals`.

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

use suffete::widen;

mod common;
use common::*;

fn bench_scalars(c: &mut Criterion) {
    let mut g = c.benchmark_group("widen::scalars");

    for (label, ty) in standard_inputs() {
        g.bench_function(label, |b| {
            b.iter(|| widen::scalars(black_box(ty)));
        });
    }

    g.finish();
}

fn bench_literals(c: &mut Criterion) {
    let mut g = c.benchmark_group("widen::literals");

    for (label, ty) in standard_inputs() {
        g.bench_function(label, |b| {
            b.iter(|| widen::literals(black_box(ty)));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_scalars, bench_literals);
criterion_main!(benches);
