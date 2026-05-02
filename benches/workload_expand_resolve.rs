//! Workload: alias / reference expansion.
//!
//! Models the unresolved-type-resolution phase of an analyzer: a forest
//! of types containing `Reference`, `MemberReference`, and nested
//! generic objects is expanded under a populated World. Stresses
//! `expand_with`, the structural walk in `expand`, and the inner
//! lookups it performs (`World::class_property_type`, etc.).
//!
//! Uses the default `ExpansionContext` plus a "full" variant that
//! enables every toggleable expansion stage.

#![allow(
    clippy::arithmetic_side_effects,
    clippy::integer_division_remainder_used,
    clippy::integer_division,
    clippy::map_with_unused_argument_over_ranges,
    clippy::verbose_bit_mask,
    clippy::missing_docs_in_private_items,
    clippy::absolute_paths,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::wildcard_imports,
    clippy::cast_possible_truncation,
    clippy::missing_const_for_fn,
    clippy::significant_drop_tightening,
    clippy::doc_markdown
)]

use core::hint::black_box;

use core::time::Duration;

use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;

use suffete::expand;
use suffete::expand::ExpansionContext;

mod common;
use common::*;

const OPS_PER_ITER: usize = 6_000_000;
const SEED: u64 = 0xDEAD_BEEF;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let world = bench_world();
    let ctx_default = ExpansionContext::default();
    let ctx_full = ExpansionContext::default()
        .with_eval_conditional(true)
        .with_fill_template_defaults(true)
        .with_substitute_template_constraints(true)
        .with_function_is_final(true);

    let mut rng = Rng::new(SEED);
    let mut targets: Vec<suffete::TypeId> = Vec::with_capacity(OPS_PER_ITER);
    for _ in 0..OPS_PER_ITER {
        targets.push(pool.pick(&mut rng));
    }

    c.bench_function("workload::expand_resolve::default", |b| {
        b.iter(|| {
            let mut acc: u32 = 0;
            for ty in &targets {
                let result = expand::expand_with(black_box(*ty), &world, &ctx_default);
                acc = acc.wrapping_add(u32::from(result.meta()));
            }
            black_box(acc)
        });
    });

    c.bench_function("workload::expand_resolve::full", |b| {
        b.iter(|| {
            let mut acc: u32 = 0;
            for ty in &targets {
                let result = expand::expand_with(black_box(*ty), &world, &ctx_full);
                acc = acc.wrapping_add(u32::from(result.meta()));
            }
            black_box(acc)
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(8));
    targets = workload
);
criterion_main!(benches);
