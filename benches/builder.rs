//! Benchmarks for [`suffete::builder::TypeBuilder`].
//!
//! Coverage: every constructor, every mutator, both terminal builds
//! (`build` plain vs. `build_canonical`), with input shapes ranging
//! from empty to wide.

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
use suffete::TypeBuilder;

mod common;
use common::*;

fn bench_constructors(c: &mut Criterion) {
    let mut g = c.benchmark_group("TypeBuilder::ctor");

    g.bench_function("new", |b| {
        b.iter(TypeBuilder::new);
    });

    g.bench_function("from_type_singleton", |b| {
        let ty = tiny_singleton();
        b.iter(|| TypeBuilder::from_type(black_box(ty)));
    });

    g.bench_function("from_type_wide", |b| {
        let ty = wide_union();
        b.iter(|| TypeBuilder::from_type(black_box(ty)));
    });

    g.finish();
}

fn bench_mutators(c: &mut Criterion) {
    let mut g = c.benchmark_group("TypeBuilder::mutator");
    let int_lit = e_int_lit(42);

    g.bench_function("push_one", |b| {
        b.iter(|| {
            let mut bldr = TypeBuilder::new();
            bldr.push(black_box(int_lit));
        });
    });

    g.bench_function("push_32", |b| {
        let elems: Vec<ElementId> = (0_i64..32).map(e_int_lit).collect();
        b.iter(|| {
            let mut bldr = TypeBuilder::new();
            for &e in &elems {
                bldr.push(black_box(e));
            }
        });
    });

    g.bench_function("extend_32", |b| {
        let elems: Vec<ElementId> = (0_i64..32).map(e_int_lit).collect();
        b.iter(|| {
            let mut bldr = TypeBuilder::new();
            bldr.extend(elems.iter().copied());
        });
    });

    g.bench_function("remove_present", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.remove(black_box(e_int_lit(15)));
            },
        );
    });

    g.bench_function("remove_all_absent", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.remove_all(black_box(e_int_lit(999)));
            },
        );
    });

    g.bench_function("retain_keep_all", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.retain(|_| black_box(true));
            },
        );
    });

    g.bench_function("replace", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.replace(black_box(e_int_lit(15)), black_box(e_int_lit(100)));
            },
        );
    });

    g.bench_function("map_identity", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.map(|e| e);
            },
        );
    });

    g.bench_function("flat_map_identity", |b| {
        b.iter_with_setup(
            || TypeBuilder::from_type(wide_union()),
            |mut bldr| {
                bldr.flat_map(|e| [e]);
            },
        );
    });

    g.bench_function("set_flags", |b| {
        b.iter_with_setup(TypeBuilder::new, |mut bldr| {
            bldr.set_flags(black_box(FlowFlags::EMPTY));
        });
    });

    g.bench_function("modify_flags", |b| {
        b.iter_with_setup(TypeBuilder::new, |mut bldr| {
            bldr.modify_flags(|f| f);
        });
    });

    g.finish();
}

fn bench_build(c: &mut Criterion) {
    let mut g = c.benchmark_group("TypeBuilder::build");

    g.bench_function("build_short_circuit", |b| {
        let ty = wide_union();
        b.iter(|| {
            let bldr = TypeBuilder::from_type(black_box(ty));
            bldr.build()
        });
    });

    g.bench_function("build_after_push", |b| {
        let ty = wide_union();
        let extra = e_int_lit(99);
        b.iter(|| {
            let mut bldr = TypeBuilder::from_type(black_box(ty));
            bldr.push(extra);
            bldr.build()
        });
    });

    g.bench_function("build_canonical_after_push", |b| {
        let ty = wide_union();
        let extra = e_int_lit(99);
        b.iter(|| {
            let mut bldr = TypeBuilder::from_type(black_box(ty));
            bldr.push(extra);
            bldr.build_canonical()
        });
    });

    g.bench_function("build_canonical_short_circuit", |b| {
        let ty = wide_union();
        b.iter(|| {
            let bldr = TypeBuilder::from_type(black_box(ty));
            bldr.build_canonical()
        });
    });

    g.finish();
}

criterion_group!(benches, bench_constructors, bench_mutators, bench_build);
criterion_main!(benches);
