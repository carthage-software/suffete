//! Workload: narrowing funnel.
//!
//! Models assertion-driven narrowing on a wide union: take a
//! many-element type and run a long sequence of `meet::narrow` and
//! `subtract::narrow` calls against it, mimicking what an analyzer does
//! across a chain of `if ($x is Foo) ... elseif (...)` guards.
//!
//! Stresses: the per-input-atom × per-narrowing-atom inner loops in
//! `meet::narrow` and `subtract::narrow`, the scratch-buffer reuse +
//! `bs_type` hoist in `subtract_all`, the singleton cache (every
//! atom-pair interns two singleton TypeIds), and `negated_atom_meet*`
//! when narrowings overlap.

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

use suffete::lattice::LatticeOptions;
use suffete::lattice::LatticeReport;
use suffete::meet;
use suffete::subtract;

mod common;
use common::*;

/// Tuned so each meet/subtract bench runs ~500ms per iter.
const NARROWS_PER_ITER: usize = 100_000;
const SEED: u64 = 0xC0DE_FEED;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let world = bench_world();
    let opts = LatticeOptions::default();

    let mut rng = Rng::new(SEED);
    let mut inputs: Vec<suffete::TypeId> = Vec::with_capacity(64);
    for _ in 0..32 {
        inputs.push(rng.pick_from(&pool.wide_unions));
    }

    for _ in 0..16 {
        inputs.push(rng.pick_from(&pool.deep_nested));
    }

    for _ in 0..16 {
        inputs.push(rng.pick_from(&pool.small_unions));
    }

    let narrowings: Vec<suffete::TypeId> = (0..NARROWS_PER_ITER).map(|_| pool.pick(&mut rng)).collect();
    let pairs: Vec<(usize, usize)> = (0..NARROWS_PER_ITER).map(|i| (i % inputs.len(), i)).collect();

    c.bench_function("workload::narrow_funnel::meet", |b| {
        b.iter(|| {
            let mut report = LatticeReport::default();
            let mut acc: u32 = 0;
            for &(in_idx, n_idx) in &pairs {
                let outcome =
                    meet::narrow(black_box(inputs[in_idx]), black_box(narrowings[n_idx]), &world, opts, &mut report);
                acc = acc.wrapping_add(u32::from(outcome.into_type().meta()));
            }

            black_box(acc)
        });
    });

    c.bench_function("workload::narrow_funnel::subtract", |b| {
        b.iter(|| {
            let mut report = LatticeReport::default();
            let mut acc: u32 = 0;
            for &(in_idx, n_idx) in &pairs {
                let outcome = subtract::narrow(
                    black_box(inputs[in_idx]),
                    black_box(narrowings[n_idx]),
                    &world,
                    opts,
                    &mut report,
                );

                acc = acc.wrapping_add(u32::from(outcome.into_type().meta()));
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
