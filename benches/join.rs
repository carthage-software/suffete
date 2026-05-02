//! Benchmarks for [`suffete::join`] public APIs:
//! `compute` (canonical preset) and `compute_with` (caller-controlled
//! options).

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
use suffete::join;
use suffete::join::JoinOptions;

mod common;
use common::*;

fn workloads() -> Vec<(&'static str, Vec<ElementId>)> {
    vec![
        ("empty", vec![]),
        ("singleton", vec![e_int_lit(0)]),
        ("3_distinct_ints", vec![e_int_lit(0), e_int_lit(1), e_int_lit(2)]),
        ("32_distinct_ints", wide_int_literals()),
        ("32_distinct_strings", wide_string_literals()),
        ("dup_singleton_x32", std::iter::repeat_n(e_int_lit(0), 32).collect()),
        ("mixed_dominators", vec![e_int_lit(0), e_int_lit(1), e_str_lit("foo"), e_str_lit("bar")]),
    ]
}

fn bench_compute(c: &mut Criterion) {
    let mut g = c.benchmark_group("join::compute");

    for (label, elems) in workloads() {
        g.bench_function(label, |b| {
            b.iter(|| join::compute(black_box(&elems)));
        });
    }

    g.finish();
}

fn bench_compute_with_default(c: &mut Criterion) {
    let mut g = c.benchmark_group("join::compute_with(default)");
    let opts = JoinOptions::default();

    for (label, elems) in workloads() {
        g.bench_function(label, |b| {
            b.iter(|| join::compute_with(black_box(&elems), &opts));
        });
    }

    g.finish();
}

fn bench_compute_with_structural(c: &mut Criterion) {
    let mut g = c.benchmark_group("join::compute_with(structural)");
    let opts = JoinOptions::structural();

    for (label, elems) in workloads() {
        g.bench_function(label, |b| {
            b.iter(|| join::compute_with(black_box(&elems), &opts));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_compute, bench_compute_with_default, bench_compute_with_structural);
criterion_main!(benches);
