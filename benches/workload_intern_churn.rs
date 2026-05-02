//! Workload: steady-state interner churn.
//!
//! Models the type-construction phase of an analyzer: a long sequence of
//! `intern_type` calls drawn from a realistic mix of singletons, small
//! unions, wide unions, and deeply nested shapes. This is the primary
//! stress for the singleton TypeId cache, the slice-arena `intern`
//! path, and the `types` arena hash + dedup.
//!
//! Each iteration runs `OPS_PER_ITER` distinct `intern_type` calls. Most
//! hit the singleton cache; some miss and exercise the slow path; some
//! intern fresh wide / deep types each iteration to keep the dashmap
//! shards warm.

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
use suffete::FlowFlags;
use suffete::interner::interner;

mod common;
use common::*;

/// Tuned so one iteration runs ~500ms on a modern desktop core.
/// Codspeed measures cycles per iteration; the target keeps each
/// measurement well above the simulator's noise floor.
const OPS_PER_ITER: usize = 8_000_000;
const SEED: u64 = 0xCAFE_F00D;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let mut element_buffers: Vec<Vec<ElementId>> = Vec::with_capacity(512);
    let mut rng = Rng::new(SEED);
    for _ in 0..512 {
        let ty = pool.pick(&mut rng);
        element_buffers.push(ty.as_ref().elements.to_vec());
    }

    let mut fresh_seed = Rng::new(SEED ^ 0xFFFF);

    c.bench_function("workload::intern_churn", |b| {
        b.iter(|| {
            let i = interner();
            let mut idx: usize = 0;
            for _ in 0..OPS_PER_ITER {
                if idx & 0x1F == 0 {
                    let v = fresh_seed.next_u64() as i64;
                    let _ = black_box(ElementId::int_literal(v));
                } else {
                    let buf = &element_buffers[idx % element_buffers.len()];
                    let _ = black_box(i.intern_type(buf, FlowFlags::EMPTY));
                }

                idx = idx.wrapping_add(1);
            }
        });
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().sample_size(10).measurement_time(Duration::from_secs(8));
    targets = workload
);

criterion_main!(benches);
