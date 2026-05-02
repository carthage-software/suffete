//! Benchmarks for [`suffete::transform`] public APIs:
//! `map`, `flat_map`, `filter_map`, `filter`.
//!
//! Three closure shapes per operation:
//! - **identity** (no change ; tests the no-op short-circuit).
//! - **rewrite** (uniform per-element replacement; rebuild path).
//! - **conditional** (branch on element kind; mixed rewrite/preserve).

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

use suffete::ElementId;
use suffete::ElementKind;
use suffete::transform;

mod common;
use common::*;

fn bench_map(c: &mut Criterion) {
    let mut g = c.benchmark_group("transform::map");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("identity_{label}"), |b| {
            b.iter(|| transform::map(black_box(ty), |e| e));
        });
        g.bench_function(format!("rewrite_{label}"), |b| {
            let int_lit = e_int_lit(0);
            b.iter(|| transform::map(black_box(ty), |e| if e.kind() == ElementKind::Int { int_lit } else { e }));
        });
    }

    g.finish();
}

fn bench_flat_map(c: &mut Criterion) {
    let mut g = c.benchmark_group("transform::flat_map");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("identity_{label}"), |b| {
            b.iter(|| transform::flat_map(black_box(ty), |e| vec![e]));
        });
        g.bench_function(format!("expand_int_{label}"), |b| {
            let alts: Vec<ElementId> = (0_i64..3).map(e_int_lit).collect();
            b.iter(|| {
                transform::flat_map(
                    black_box(ty),
                    |e| {
                        if e.kind() == ElementKind::Int { alts.clone() } else { vec![e] }
                    },
                )
            });
        });
        g.bench_function(format!("drop_int_{label}"), |b| {
            b.iter(|| {
                transform::flat_map(black_box(ty), |e| if e.kind() == ElementKind::Int { vec![] } else { vec![e] })
            });
        });
    }

    g.finish();
}

fn bench_filter_map(c: &mut Criterion) {
    let mut g = c.benchmark_group("transform::filter_map");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("identity_{label}"), |b| {
            b.iter(|| transform::filter_map(black_box(ty), |e| Some(e)));
        });
        g.bench_function(format!("drop_int_{label}"), |b| {
            b.iter(|| {
                transform::filter_map(black_box(ty), |e| if e.kind() == ElementKind::Int { None } else { Some(e) })
            });
        });
    }

    g.finish();
}

fn bench_filter(c: &mut Criterion) {
    let mut g = c.benchmark_group("transform::filter");

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("keep_all_{label}"), |b| {
            b.iter(|| transform::filter(black_box(ty), |_| true));
        });
        g.bench_function(format!("drop_int_{label}"), |b| {
            b.iter(|| transform::filter(black_box(ty), |e| e.kind() != ElementKind::Int));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_map, bench_flat_map, bench_filter_map, bench_filter);
criterion_main!(benches);
