//! Benchmarks for [`suffete::serialize`] round-trip:
//! `TypeId::to_serializable` then `SerializableType::intern`.

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

mod common;
use common::*;

fn bench_to_serializable(c: &mut Criterion) {
    let mut g = c.benchmark_group("serialize::to_serializable");

    for (label, ty) in standard_inputs() {
        g.bench_function(label, |b| {
            b.iter(|| black_box(ty).to_serializable());
        });
    }

    g.finish();
}

fn bench_intern(c: &mut Criterion) {
    let mut g = c.benchmark_group("serialize::intern");

    let inputs: Vec<(&'static str, suffete::serialize::SerializableType)> =
        standard_inputs().into_iter().map(|(l, t)| (l, t.to_serializable())).collect();

    for (label, ser) in inputs {
        g.bench_function(label, |b| {
            b.iter(|| black_box(&ser).intern());
        });
    }

    g.finish();
}

fn bench_roundtrip(c: &mut Criterion) {
    let mut g = c.benchmark_group("serialize::roundtrip");

    for (label, ty) in standard_inputs() {
        g.bench_function(label, |b| {
            b.iter(|| black_box(ty).to_serializable().intern());
        });
    }

    g.finish();
}

criterion_group!(benches, bench_to_serializable, bench_intern, bench_roundtrip);
criterion_main!(benches);
