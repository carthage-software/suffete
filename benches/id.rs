#![allow(
    clippy::significant_drop_tightening,
    clippy::wildcard_imports,
    clippy::missing_const_for_fn,
    clippy::separated_literal_suffix,
    clippy::doc_markdown,
    clippy::redundant_closure,
    clippy::arithmetic_side_effects,
    clippy::absolute_paths,
    clippy::missing_docs_in_private_items
)]

use core::hint::black_box;
use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use suffete::ElementId;
use suffete::ElementKind;

fn bench_element_id_construction(c: &mut Criterion) {
    c.bench_function("ElementId::new + decode", |b| {
        b.iter(|| {
            let id = ElementId::new(black_box(ElementKind::Int), black_box(42));
            (id.kind(), id.slot())
        });
    });
}

criterion_group!(benches, bench_element_id_construction);
criterion_main!(benches);
