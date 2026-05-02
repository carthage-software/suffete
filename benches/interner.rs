//! Benchmarks for [`suffete::interner::Interner`] public surface.
//!
//! Targets the hot intern + lookup loops the rest of the crate sits on
//! top of: type interning (intern_type), element-list interning, and
//! get_type lookup. Per-payload `intern_*` / `get_*` are exercised
//! transitively through the `element` and `ty` benches.

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
use suffete::FlowFlags;
use suffete::interner::interner;

mod common;
use common::*;

fn bench_intern_type(c: &mut Criterion) {
    let mut g = c.benchmark_group("Interner::intern_type");
    let i = interner();

    let single = vec![e_int_lit(0)];
    let three = vec![e_int_lit(0), e_int_lit(1), e_int_lit(2)];
    let wide: Vec<ElementId> = (0_i64..32).map(e_int_lit).collect();

    g.bench_function("single", |b| {
        b.iter(|| i.intern_type(black_box(&single), FlowFlags::EMPTY));
    });
    g.bench_function("three", |b| {
        b.iter(|| i.intern_type(black_box(&three), FlowFlags::EMPTY));
    });
    g.bench_function("wide_dedup", |b| {
        b.iter(|| i.intern_type(black_box(&wide), FlowFlags::EMPTY));
    });

    g.finish();
}

fn bench_get_type(c: &mut Criterion) {
    let mut g = c.benchmark_group("Interner::get_type");
    let i = interner();

    let singleton = tiny_singleton();
    let small = small_union();
    let wide = wide_union();

    g.bench_function("singleton", |b| {
        b.iter(|| i.get_type(black_box(singleton)));
    });
    g.bench_function("small_union", |b| {
        b.iter(|| i.get_type(black_box(small)));
    });
    g.bench_function("wide_union", |b| {
        b.iter(|| i.get_type(black_box(wide)));
    });

    g.finish();
}

fn bench_intern_element_list(c: &mut Criterion) {
    let mut g = c.benchmark_group("Interner::intern_element_list");
    let i = interner();

    let small = vec![e_int_lit(0), e_int_lit(1)];
    let wide: Vec<ElementId> = (0_i64..16).map(e_int_lit).collect();

    g.bench_function("small", |b| {
        b.iter(|| i.intern_element_list(black_box(&small)));
    });
    g.bench_function("wide", |b| {
        b.iter(|| i.intern_element_list(black_box(&wide)));
    });

    g.finish();
}

criterion_group!(benches, bench_intern_type, bench_get_type, bench_intern_element_list);
criterion_main!(benches);
