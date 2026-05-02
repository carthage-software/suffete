//! Benchmarks for [`suffete::template`] public APIs:
//! `substitute` (template-parameter rewrite) and `standin` (call-site
//! inference).

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
use suffete::ElementId;
use suffete::TypeId;
use suffete::element::payload::DefiningEntity;
use suffete::template;
use suffete::template::StandinOptions;
use suffete::template::TemplateState;
use suffete::world::Variance;

mod common;
use common::*;

fn template_t() -> TypeId {
    let entity = DefiningEntity::ClassLike(atom("Foo"));
    ut(ElementId::generic_parameter("T", entity, t_mixed()))
}

fn list_of_t() -> TypeId {
    let t_param = template_t();
    let elem = ut(e_list(t_param, false));
    ut(elem.as_ref().elements[0])
}

fn bench_substitute(c: &mut Criterion) {
    let mut g = c.benchmark_group("template::substitute");

    let no_param = small_union();
    let single_param = template_t();
    let nested = list_of_t();
    let int_replacement = t_int();

    let always_int =
        |_info: &suffete::element::payload::GenericParameterInfo| -> Option<TypeId> { Some(int_replacement) };
    let never = |_info: &suffete::element::payload::GenericParameterInfo| -> Option<TypeId> { None };

    g.bench_function("no_param_identity", |b| {
        b.iter(|| template::substitute(black_box(no_param), &never));
    });

    g.bench_function("singleton_replace", |b| {
        b.iter(|| template::substitute(black_box(single_param), &always_int));
    });

    g.bench_function("nested_replace", |b| {
        b.iter(|| template::substitute(black_box(nested), &always_int));
    });

    g.bench_function("nested_skip", |b| {
        b.iter(|| template::substitute(black_box(nested), &never));
    });

    g.finish();
}

fn bench_standin(c: &mut Criterion) {
    let mut g = c.benchmark_group("template::standin");
    let world = bench_world();
    let opts = StandinOptions { argument_offset: 0, default_variance: Variance::Invariant, max_depth: 8, span: None };

    let parameter = template_t();
    let argument_int = t_int();
    let argument_string = t_string();
    let argument_union = small_union();
    let argument_deep = deep_nested();

    let pairs = [
        ("identity", parameter, parameter),
        ("int", parameter, argument_int),
        ("string", parameter, argument_string),
        ("union", parameter, argument_union),
        ("deep", parameter, argument_deep),
    ];

    for (label, p, a) in pairs {
        g.bench_function(label, |b| {
            b.iter(|| {
                let mut state = TemplateState::default();
                template::standin(black_box(p), black_box(a), &world, &mut state, &opts)
            });
        });
    }

    g.finish();
}

criterion_group!(benches, bench_substitute, bench_standin);
criterion_main!(benches);
