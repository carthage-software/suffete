//! Benchmarks for [`suffete::inspect::any`] and [`suffete::inspect::all`].
//!
//! Coverage:
//! - Predicate that hits early (first element).
//! - Predicate that hits late (last element).
//! - Predicate that never hits.
//! - Across the standard input shapes so deep-traversal regressions
//!   surface alongside flat-traversal ones.

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

use suffete::ElementKind;
use suffete::inspect;

mod common;
use common::standard_inputs;

fn bench_any(c: &mut Criterion) {
    let mut g = c.benchmark_group("inspect::any");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("hit_int_{label}"), |b| {
            b.iter(|| inspect::any(black_box(ty), |e| e.kind() == ElementKind::Int));
        });
        g.bench_function(format!("miss_{label}"), |b| {
            b.iter(|| inspect::any(black_box(ty), |e| e.kind() == ElementKind::Resource));
        });
    }

    g.finish();
}

fn bench_all(c: &mut Criterion) {
    let mut g = c.benchmark_group("inspect::all");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("all_int_{label}"), |b| {
            b.iter(|| inspect::all(black_box(ty), |e| e.kind() == ElementKind::Int));
        });
        g.bench_function(format!("all_truthy_{label}"), |b| {
            b.iter(|| inspect::all(black_box(ty), |e| e.kind() != ElementKind::Null));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_any, bench_all);
criterion_main!(benches);
