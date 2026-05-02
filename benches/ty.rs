//! Benchmarks for [`suffete::ty`] public APIs:
//! - `TypeId` constructors (`singleton`, `union`, `int_literal`,
//!   `int_range`, `float_literal`, `string_literal`).
//! - `TypeId` accessors (`flags`, `meta`, `slot` indirectly via `as_ref`,
//!   `with_flags`, `with_meta`, `content_eq`, `as_ref`).

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
use suffete::TypeId;

mod common;
use common::*;

fn bench_constructors(c: &mut Criterion) {
    let mut g = c.benchmark_group("TypeId::ctor");

    g.bench_function("singleton", |b| {
        let elem = ElementId::int_literal(42);
        b.iter(|| TypeId::singleton(black_box(elem)));
    });

    g.bench_function("union_3", |b| {
        let elems = [ElementId::int_literal(0), ElementId::int_literal(1), ElementId::int_literal(2)];
        b.iter(|| TypeId::union(black_box(&elems)));
    });

    g.bench_function("union_32_dedup", |b| {
        let elems: Vec<ElementId> = (0..32).map(ElementId::int_literal).collect();
        b.iter(|| TypeId::union(black_box(&elems)));
    });

    g.bench_function("union_32_unsorted", |b| {
        let mut elems: Vec<ElementId> = (0_i64..32).rev().map(ElementId::int_literal).collect();
        elems.swap(0, 16);
        b.iter(|| TypeId::union(black_box(&elems)));
    });

    g.bench_function("int_literal", |b| {
        b.iter(|| TypeId::int_literal(black_box(42)));
    });

    g.bench_function("int_range_open", |b| {
        b.iter(|| TypeId::int_range(black_box(None), black_box(None)));
    });

    g.bench_function("int_range_closed", |b| {
        b.iter(|| TypeId::int_range(black_box(Some(0)), black_box(Some(100))));
    });

    g.bench_function("float_literal", |b| {
        b.iter(|| TypeId::float_literal(black_box(1.5)));
    });

    g.bench_function("string_literal", |b| {
        b.iter(|| TypeId::string_literal(black_box("hello")));
    });

    g.finish();
}

fn bench_accessors(c: &mut Criterion) {
    let mut g = c.benchmark_group("TypeId::accessor");

    let singleton = tiny_singleton();
    let small = small_union();
    let wide = wide_union();
    let deep = deep_nested();

    for (name, ty) in [("singleton", singleton), ("small", small), ("wide", wide), ("deep", deep)] {
        g.bench_function(format!("flags_{name}"), |b| {
            b.iter(|| black_box(ty).flags());
        });

        g.bench_function(format!("meta_{name}"), |b| {
            b.iter(|| black_box(ty).meta());
        });

        g.bench_function(format!("as_ref_{name}"), |b| {
            b.iter(|| black_box(ty).as_ref());
        });
    }

    g.bench_function("with_flags", |b| {
        let ty = singleton;
        let flags = FlowFlags::EMPTY;
        b.iter(|| black_box(ty).with_flags(black_box(flags)));
    });

    g.bench_function("with_meta", |b| {
        let ty = singleton;
        b.iter(|| black_box(ty).with_meta(black_box(7)));
    });

    g.bench_function("content_eq_same", |b| {
        let a = singleton;
        let b_ty = singleton.with_meta(3);
        b.iter(|| black_box(a).content_eq(black_box(b_ty)));
    });

    g.bench_function("content_eq_different", |b| {
        let a = singleton;
        let b_ty = small;
        b.iter(|| black_box(a).content_eq(black_box(b_ty)));
    });

    g.finish();
}

fn bench_flow_flags(c: &mut Criterion) {
    let mut g = c.benchmark_group("FlowFlags");

    g.bench_function("from_bits", |b| {
        b.iter(|| FlowFlags::from_bits(black_box(0xff)));
    });

    g.bench_function("bits", |b| {
        let f = FlowFlags::EMPTY;
        b.iter(|| black_box(f).bits());
    });

    g.finish();
}

criterion_group!(benches, bench_constructors, bench_accessors, bench_flow_flags);
criterion_main!(benches);
