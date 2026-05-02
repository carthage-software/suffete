//! Benchmarks for [`suffete::hierarchy`] public APIs:
//! `HierarchyBuilder::new`, `HierarchyBuilder::add_edge`,
//! `HierarchyBuilder::build`, `Hierarchy::args`, `Hierarchy::arg`,
//! `Hierarchy::iter`.

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

use mago_atom::atom;
use suffete::TypeId;
use suffete::hierarchy::Hierarchy;
use suffete::hierarchy::HierarchyBuilder;

mod common;
use common::*;

/// Build a linear hierarchy `C0 -> C1 -> ... -> C(n-1)`.
fn linear_builder(n: usize) -> HierarchyBuilder {
    let mut bldr = HierarchyBuilder::new();
    for i in 0..(n - 1) {
        let child = atom(&format!("C{i}"));
        let parent = atom(&format!("C{}", i + 1));
        bldr.add_edge(child, parent, vec![t_int()]);
    }
    bldr
}

/// Build a fan-out hierarchy: `C0` extends `P0`..`P(n-1)`.
fn fanout_builder(n: usize) -> HierarchyBuilder {
    let mut bldr = HierarchyBuilder::new();
    let child = atom("C0");
    for i in 0..n {
        let parent = atom(&format!("P{i}"));
        bldr.add_edge(child, parent, vec![t_int()]);
    }
    bldr
}

fn bench_builder(c: &mut Criterion) {
    let mut g = c.benchmark_group("HierarchyBuilder");
    let world = bench_world();

    g.bench_function("new", |b| {
        b.iter(HierarchyBuilder::new);
    });

    g.bench_function("add_edge", |b| {
        let child = atom("C");
        let parent = atom("P");
        let args = vec![t_int()];
        b.iter_with_setup(HierarchyBuilder::new, |mut bldr| {
            bldr.add_edge(black_box(child), black_box(parent), args.clone());
        });
    });

    for n in [4, 16, 64] {
        g.bench_function(format!("build_linear_{n}"), |b| {
            b.iter_with_setup(|| linear_builder(n), |bldr| bldr.build(&world));
        });
        g.bench_function(format!("build_fanout_{n}"), |b| {
            b.iter_with_setup(|| fanout_builder(n), |bldr| bldr.build(&world));
        });
    }

    g.finish();
}

fn bench_lookups(c: &mut Criterion) {
    let mut g = c.benchmark_group("Hierarchy::lookup");
    let world = bench_world();

    let h: Hierarchy = linear_builder(16).build(&world);
    let leaf = atom("C0");
    let root = atom("C15");
    let absent = atom("Nope");

    g.bench_function("args_present", |b| {
        b.iter(|| black_box(&h).args(black_box(leaf), black_box(root)));
    });

    g.bench_function("args_absent", |b| {
        b.iter(|| black_box(&h).args(black_box(leaf), black_box(absent)));
    });

    g.bench_function("arg_present_pos0", |b| {
        b.iter(|| black_box(&h).arg(black_box(leaf), black_box(root), 0));
    });

    g.bench_function("iter_collect", |b| {
        b.iter(|| {
            let pairs: Vec<((mago_atom::Atom, mago_atom::Atom), Vec<TypeId>)> =
                black_box(&h).iter().map(|(k, v)| (k, v.to_vec())).collect();
            pairs
        });
    });

    g.finish();
}

criterion_group!(benches, bench_builder, bench_lookups);
criterion_main!(benches);
