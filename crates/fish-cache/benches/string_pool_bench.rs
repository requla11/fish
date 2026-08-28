use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use fish_cache::{ScopedString, StringPool};
use std::hint::black_box;
use std::sync::Arc;

fn bench_string_allocations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_allocations");

    for size in [16, 64, 256, 1024, 4096].iter() {
        // Baseline: direct String allocation
        group.bench_with_input(BenchmarkId::new("direct", size), size, |b, &size| {
            b.iter(|| {
                let _s = String::with_capacity(size);
            })
        });

        // With StringPool
        let pool = Arc::new(StringPool::new());
        group.bench_with_input(BenchmarkId::new("pooled", size), size, |b, &size| {
            b.iter(|| {
                let s = pool.get_string(size);
                pool.return_string(s);
            })
        });
    }

    group.finish();
}

fn bench_realistic_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("realistic_reuse");

    // Simulate realistic scenario: repeated allocation/deallocation
    // This is where pooling shines - reduces memory fragmentation
    group.bench_function("direct_reuse", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let mut s = String::with_capacity(100);
                s.push_str("test");
                black_box(&s);
            }
        })
    });

    let pool = Arc::new(StringPool::new());
    group.bench_function("pooled_reuse", |b| {
        b.iter(|| {
            for _ in 0..100 {
                let mut s = pool.get_string(100);
                s.push_str("test");
                black_box(&s);
                pool.return_string(s);
            }
        })
    });

    group.finish();
}

fn bench_string_with_content(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_with_content");

    // Baseline: create and fill string
    group.bench_function("direct_fill", |b| {
        b.iter(|| {
            let mut s = String::with_capacity(100);
            s.push_str("hello world, this is a test string for benchmarking");
            black_box(s);
        })
    });

    // With StringPool
    let pool = Arc::new(StringPool::new());
    group.bench_function("pooled_fill", |b| {
        b.iter(|| {
            let mut s = pool.get_string(100);
            s.push_str("hello world, this is a test string for benchmarking");
            pool.return_string(s);
        })
    });

    group.finish();
}

fn bench_scoped_string(c: &mut Criterion) {
    let pool = Arc::new(StringPool::new());

    c.bench_function("scoped_string", |b| {
        b.iter(|| {
            let mut scoped = ScopedString::new(100, pool.clone());
            scoped.as_mut().push_str("test data");
            black_box(scoped.as_ref().clone());
        })
    });
}

fn bench_string_pool_stats(c: &mut Criterion) {
    let pool = StringPool::new();

    c.bench_function("pool_stats", |b| {
        b.iter(|| {
            black_box(pool.stats());
        })
    });
}

criterion_group!(
    benches,
    bench_string_allocations,
    bench_realistic_reuse,
    bench_string_with_content,
    bench_scoped_string,
    bench_string_pool_stats
);
criterion_main!(benches);
