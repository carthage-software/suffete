//! Benchmarks for [`suffete::expand`] public APIs.

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

use suffete::expand;
use suffete::expand::ExpansionContext;

mod common;
use common::*;

fn bench_expand(c: &mut Criterion) {
    let mut g = c.benchmark_group("expand::expand");
    let world = bench_world();

    for (label, ty) in standard_inputs() {
        g.bench_function(label, |b| {
            b.iter(|| expand::expand(black_box(ty), &world));
        });
    }

    g.finish();
}

fn bench_expand_with(c: &mut Criterion) {
    let mut g = c.benchmark_group("expand::expand_with");
    let world = bench_world();
    let ctx_default = ExpansionContext::default();
    let ctx_full = ExpansionContext::default()
        .with_eval_conditional(true)
        .with_fill_template_defaults(true)
        .with_substitute_template_constraints(true)
        .with_function_is_final(true);

    for (label, ty) in standard_inputs() {
        g.bench_function(format!("default_{label}"), |b| {
            b.iter(|| expand::expand_with(black_box(ty), &world, &ctx_default));
        });
        g.bench_function(format!("full_{label}"), |b| {
            b.iter(|| expand::expand_with(black_box(ty), &world, &ctx_full));
        });
    }

    g.finish();
}

criterion_group!(benches, bench_expand, bench_expand_with);
criterion_main!(benches);
