//! Conversion benchmarks

use criterion::{criterion_group, criterion_main, Criterion};

fn conversion_benchmark(c: &mut Criterion) {
    c.bench_function("noop", |b| {
        b.iter(|| {
            // TODO: Add actual benchmarks
            std::hint::black_box(1 + 1)
        })
    });
}

criterion_group!(benches, conversion_benchmark);
criterion_main!(benches);
