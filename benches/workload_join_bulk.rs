//! Workload: bulk union construction.
//!
//! Models the union-construction phase of an analyzer: thousands of
//! `join::compute` calls with varying-width input slices. Stresses
//! `canonicalize` (the densest cluster of SIMD `contains` calls),
//! `apply_same_kind_dominator`, the per-family payload merges
//! (range merging, string-axis collapse, scalar synthesis, list /
//! keyed-array element-type unions), and the SIMD `count_of_kind`
//! threshold checks in the literal-collapse rules.

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

use suffete::ElementId;
use suffete::join;

mod common;
use common::*;

/// Tuned so one iteration runs ~500ms.
const JOINS_PER_ITER: usize = 500_000;
const SEED: u64 = 0xF00D_BABE;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let mut rng = Rng::new(SEED);
    let mut inputs: Vec<Vec<ElementId>> = Vec::with_capacity(JOINS_PER_ITER);
    for _ in 0..JOINS_PER_ITER {
        let n = 1 + (rng.next_u32() % 8) as usize;
        let mut elems: Vec<ElementId> = Vec::with_capacity(n * 2);
        for _ in 0..n {
            let t = pool.pick(&mut rng);
            elems.extend(t.as_ref().elements);
        }

        inputs.push(elems);
    }

    c.bench_function("workload::join_bulk", |b| {
        b.iter(|| {
            let mut acc: usize = 0;
            for input in &inputs {
                let result = join::compute(black_box(input));
                acc = acc.wrapping_add(result.len());
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
