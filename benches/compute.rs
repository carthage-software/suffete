use criterion::Criterion;
use criterion::criterion_group;
use criterion::criterion_main;
use std::hint::black_box;
use suffete::compute;

fn bench_compute(c: &mut Criterion) {
    c.bench_function("compute", |b| b.iter(|| compute(black_box(123_456), black_box(789_012))));
}

criterion_group!(benches, bench_compute);
criterion_main!(benches);
