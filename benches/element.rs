//! Benchmarks for [`suffete::element`] public APIs:
//! - Every `ElementId::*` constructor (interning hot paths).
//! - Every `ElementId::*` accessor (`view`, `intersection_types`,
//!   `has_intersection_types`, `can_be_intersected`).

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
use suffete::ElementKind;
use suffete::element::payload::ArrayKey;
use suffete::element::payload::DefiningEntity;
use suffete::element::payload::KnownElementEntry;
use suffete::element::payload::KnownItemEntry;
use suffete::prelude;

mod common;
use common::*;

fn bench_constructors(c: &mut Criterion) {
    let mut g = c.benchmark_group("ElementId::ctor");

    g.bench_function("new", |b| {
        b.iter(|| ElementId::new(black_box(ElementKind::Int), black_box(42)));
    });

    g.bench_function("int_literal_fresh", |b| {
        let mut v = 0_i64;
        b.iter(|| {
            v = v.wrapping_add(1);
            ElementId::int_literal(black_box(v))
        });
    });

    g.bench_function("int_literal_dedup", |b| {
        b.iter(|| ElementId::int_literal(black_box(42)));
    });

    g.bench_function("int_range_open", |b| {
        b.iter(|| ElementId::int_range(black_box(None), black_box(None)));
    });

    g.bench_function("int_range_closed", |b| {
        b.iter(|| ElementId::int_range(black_box(Some(0)), black_box(Some(100))));
    });

    g.bench_function("float_literal", |b| {
        b.iter(|| ElementId::float_literal(black_box(1.5)));
    });

    g.bench_function("string_literal_short", |b| {
        b.iter(|| ElementId::string_literal(black_box("foo")));
    });

    g.bench_function("string_literal_long", |b| {
        let s = "the quick brown fox jumps over the lazy dog".repeat(4);
        b.iter(|| ElementId::string_literal(black_box(&s)));
    });

    g.bench_function("string_literal_numeric", |b| {
        b.iter(|| ElementId::string_literal(black_box("12345")));
    });

    g.bench_function("object_named", |b| {
        b.iter(|| ElementId::object_named(black_box("Foo")));
    });

    g.bench_function("enum_any", |b| {
        b.iter(|| ElementId::enum_any(black_box("Status")));
    });

    g.bench_function("enum_case", |b| {
        b.iter(|| ElementId::enum_case(black_box("Status"), black_box("Active")));
    });

    g.bench_function("class_string_literal", |b| {
        b.iter(|| ElementId::class_string_literal(black_box("App\\Foo")));
    });

    g.bench_function("negated", |b| {
        let inner = t_int();
        b.iter(|| ElementId::negated(black_box(inner)));
    });

    g.bench_function("iterable", |b| {
        let key = t_string();
        let value = t_int();
        b.iter(|| ElementId::iterable(black_box(key), black_box(value)));
    });

    g.bench_function("list_unsealed", |b| {
        let elem = t_int();
        b.iter(|| ElementId::list(black_box(elem), black_box(false)));
    });

    g.bench_function("list_non_empty", |b| {
        let elem = t_int();
        b.iter(|| ElementId::list(black_box(elem), black_box(true)));
    });

    g.bench_function("sealed_list_3", |b| {
        let entries = [
            KnownElementEntry { index: 0, value: t_int(), optional: false },
            KnownElementEntry { index: 1, value: t_string(), optional: false },
            KnownElementEntry { index: 2, value: t_bool(), optional: true },
        ];
        b.iter(|| ElementId::sealed_list(black_box(&entries), black_box(true)));
    });

    g.bench_function("keyed_unsealed", |b| {
        let k = t_string();
        let v = t_int();
        b.iter(|| ElementId::keyed_unsealed(black_box(k), black_box(v), black_box(false)));
    });

    g.bench_function("keyed_sealed_3", |b| {
        let entries = [
            KnownItemEntry { key: ArrayKey::String(mago_atom::atom("a")), value: t_int(), optional: false },
            KnownItemEntry { key: ArrayKey::String(mago_atom::atom("b")), value: t_string(), optional: false },
            KnownItemEntry { key: ArrayKey::Int(0), value: t_bool(), optional: true },
        ];
        b.iter(|| ElementId::keyed_sealed(black_box(&entries), black_box(true)));
    });

    g.bench_function("callable_any", |b| {
        b.iter(ElementId::callable_any);
    });

    g.bench_function("callable_mixed", |b| {
        b.iter(ElementId::callable_mixed);
    });

    g.bench_function("closure_mixed", |b| {
        b.iter(ElementId::closure_mixed);
    });

    g.bench_function("generic_parameter", |b| {
        let entity = DefiningEntity::ClassLike(mago_atom::atom("Foo"));
        b.iter(|| ElementId::generic_parameter(black_box("T"), black_box(entity), black_box(t_int())));
    });

    g.finish();
}

fn bench_accessors(c: &mut Criterion) {
    let mut g = c.benchmark_group("ElementId::accessor");

    let int_elem = e_int_lit(42);
    let object_elem = e_named("Foo");
    let trivial_elem = prelude::NULL;

    g.bench_function("kind", |b| {
        let elem = int_elem;
        b.iter(|| black_box(elem).kind());
    });

    g.bench_function("slot", |b| {
        let elem = int_elem;
        b.iter(|| black_box(elem).slot());
    });

    g.bench_function("view_int", |b| {
        let elem = int_elem;
        b.iter(|| black_box(elem).view());
    });

    g.bench_function("view_object", |b| {
        let elem = object_elem;
        b.iter(|| black_box(elem).view());
    });

    g.bench_function("view_trivial", |b| {
        let elem = trivial_elem;
        b.iter(|| black_box(elem).view());
    });

    g.bench_function("intersection_types_object", |b| {
        let elem = object_elem;
        b.iter(|| black_box(elem).intersection_types());
    });

    g.bench_function("has_intersection_types", |b| {
        let elem = object_elem;
        b.iter(|| black_box(elem).has_intersection_types());
    });

    g.bench_function("can_be_intersected_object", |b| {
        let elem = object_elem;
        b.iter(|| black_box(elem).can_be_intersected());
    });

    g.bench_function("can_be_intersected_int", |b| {
        let elem = int_elem;
        b.iter(|| black_box(elem).can_be_intersected());
    });

    g.finish();
}

criterion_group!(benches, bench_constructors, bench_accessors);
criterion_main!(benches);
