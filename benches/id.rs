use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use std::hint::black_box;
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
