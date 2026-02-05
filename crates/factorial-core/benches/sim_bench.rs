use criterion::{criterion_group, criterion_main};

fn placeholder(_c: &mut criterion::Criterion) {
    // Benchmarks will be added as simulation components are implemented.
}

criterion_group!(benches, placeholder);
criterion_main!(benches);
