//! Workload: lattice storm.
//!
//! Models a flow-typing pass: a large pool of types compared pairwise
//! via every public lattice predicate (`refines`, `generalizes`,
//! `overlaps`). Each iteration runs `PAIRS_PER_ITER` random pairings,
//! exercising the pairwise-fold inside `refines`/`overlaps`, the SIMD
//! `union_covers` family (`int_union_covers`, `string_union_covers`,
//! `bool_union_covers`, `mixed_union_covers`), and the singleton cache
//! along the way (`atom_meet` / `atom_minus` intern singleton TypeIds
//! per pair).

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
use suffete::lattice::generalizes;
use suffete::lattice::overlaps;
use suffete::lattice::refines;

mod common;
use common::*;

/// Tuned so one iteration runs ~500ms (each pair fans out to three
/// `refines/generalizes/overlaps` calls).
const PAIRS_PER_ITER: usize = 2_000_000;
const SEED: u64 = 0xBEEF_BABE;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let world = bench_world();
    let opts = LatticeOptions::default();

    let mut rng = Rng::new(SEED);
    let pairs: Vec<(usize, usize)> =
        (0..PAIRS_PER_ITER).map(|_| (rng.pick(pool.weighted.len()), rng.pick(pool.weighted.len()))).collect();

    c.bench_function("workload::lattice_storm", |b| {
        b.iter(|| {
            let mut report = LatticeReport::default();
            let mut hits: u32 = 0;
            for &(a_idx, b_idx) in &pairs {
                let a = pool.weighted[a_idx];
                let b_ty = pool.weighted[b_idx];
                // One of each, round-robin, to exercise all three predicates.
                if refines(black_box(a), black_box(b_ty), &world, opts, &mut report) {
                    hits = hits.wrapping_add(1);
                }
                if generalizes(black_box(a), black_box(b_ty), &world, opts, &mut report) {
                    hits = hits.wrapping_add(1);
                }
                if overlaps(black_box(a), black_box(b_ty), &world, opts, &mut report) {
                    hits = hits.wrapping_add(1);
                }
            }

            black_box(hits)
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(8));
    targets = workload
);

criterion_main!(benches);
