//! Workload: template substitution + structural walk.
//!
//! Models the generic-instantiation phase of an analyzer: a forest of
//! template-bearing types is walked end-to-end via `transform::map`,
//! `transform::flat_map`, and `template::substitute` to specialise
//! `T -> concrete` across thousands of call sites.
//!
//! Stresses: the post-order walker in `transform`, every
//! payload-bearing arm of `walk_nested` (object args, list element type,
//! keyed-array key/value, iterable, callable signature, intersections),
//! and `template::substitute`'s per-element decision under
//! `transform::flat_map`.

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

use mago_atom::atom;
use suffete::ElementId;
use suffete::ElementKind;
use suffete::TypeId;
use suffete::element::payload::DefiningEntity;
use suffete::element::payload::GenericParameterInfo;
use suffete::template;
use suffete::transform;

mod common;
use common::*;

const SUBSTITUTE_OPS: usize = 200_000;
const MAP_OPS: usize = 5_000_000;
const SEED: u64 = 0x1337_DEAD;

fn workload(c: &mut Criterion) {
    let pool = TypePool::new(SEED);
    let mut rng = Rng::new(SEED);

    let entity = DefiningEntity::ClassLike(atom("Foo"));
    let t_param = ut(ElementId::generic_parameter("T", entity, suffete::prelude::TYPE_MIXED));

    let mut templated_tree: Vec<TypeId> = Vec::with_capacity(64);
    for _ in 0..64 {
        let depth = 1 + (rng.next_u32() % 3) as usize;
        let mut current = t_param;
        for _ in 0..depth {
            current = ut(ElementId::list(current, false));
        }
        templated_tree.push(current);
    }

    let replacements: Vec<TypeId> = (0..256).map(|_| rng.pick_from(&pool.singletons)).collect();
    let substitute_work: Vec<(usize, usize)> =
        (0..SUBSTITUTE_OPS).map(|i| (i % templated_tree.len(), i % replacements.len())).collect();
    let map_work: Vec<usize> = (0..MAP_OPS).map(|i| i % templated_tree.len()).collect();

    c.bench_function("workload::template_walk::substitute", |b| {
        b.iter(|| {
            let mut acc: u32 = 0;
            for &(t_idx, r_idx) in &substitute_work {
                let target = templated_tree[t_idx];
                let replacement = replacements[r_idx];
                let result = template::substitute(black_box(target), &|info: &GenericParameterInfo| {
                    let _ = info;
                    Some(replacement)
                });

                acc = acc.wrapping_add(u32::from(result.meta()));
            }

            black_box(acc)
        });
    });

    c.bench_function("workload::template_walk::transform_map", |b| {
        let identity_target = ElementId::int_literal(0);
        b.iter(|| {
            let mut acc: u32 = 0;
            for &t_idx in &map_work {
                let target = templated_tree[t_idx];
                let result =
                    transform::map(
                        black_box(target),
                        |e| {
                            if e.kind() == ElementKind::Int { identity_target } else { e }
                        },
                    );

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
