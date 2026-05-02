//! Benchmarks for [`suffete::cast::cast`] across every [`CastTarget`].

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

use suffete::cast;
use suffete::cast::CastTarget;

mod common;
use common::*;

fn bench_cast(c: &mut Criterion) {
    let mut g = c.benchmark_group("cast::cast");
    let world = bench_world();

    let inputs = standard_inputs();
    let targets = [
        ("int", CastTarget::Int),
        ("float", CastTarget::Float),
        ("string", CastTarget::String),
        ("bool", CastTarget::Bool),
        ("array", CastTarget::Array),
        ("object", CastTarget::Object),
    ];

    for (input_label, ty) in &inputs {
        for (target_label, target) in &targets {
            g.bench_function(format!("{input_label}_to_{target_label}"), |b| {
                b.iter(|| cast::cast(black_box(*ty), black_box(*target), &world));
            });
        }
    }

    g.finish();
}

criterion_group!(benches, bench_cast);
criterion_main!(benches);
