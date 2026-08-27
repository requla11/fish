use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fish_cas::artifact::{ArtifactHash, ArtifactMetadata};
use fish_cas::compression::{CompressionAlgorithm, compress, decompress};
use fish_cas::mmap::MmapArtifact;
use std::hint::black_box;
use tempfile::tempdir;

fn bench_artifact_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("cas_blake3_hashing");
    for size in [1024, 64 * 1024, 1024 * 1024] {
        let data = vec![0xABu8; size];
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &data, |b, d| {
            b.iter(|| {
                let hash = ArtifactHash::from_bytes(black_box(d)).unwrap();
                black_box(hash);
            });
        });
    }
    group.finish();
}

fn bench_zstd_compression(c: &mut Criterion) {
    let mut group = c.benchmark_group("cas_zstd_compression");
    let size = 64 * 1024;
    let data = vec![0x42u8; size];
    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("compress_zstd_default", |b| {
        b.iter(|| {
            let compressed = compress(black_box(&data), CompressionAlgorithm::Zstd).unwrap();
            black_box(compressed);
        });
    });

    group.bench_function("compress_zstd_fast", |b| {
        b.iter(|| {
            let compressed = compress(black_box(&data), CompressionAlgorithm::ZstdFast).unwrap();
            black_box(compressed);
        });
    });

    let compressed = compress(&data, CompressionAlgorithm::Zstd).unwrap();
    group.bench_function("decompress_zstd_default", |b| {
        b.iter(|| {
            let decompressed =
                decompress(black_box(&compressed), CompressionAlgorithm::Zstd).unwrap();
            black_box(decompressed);
        });
    });

    group.finish();
}

fn bench_zero_copy_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("cas_zero_copy_reads");
    let size = 1024 * 1024;
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("payload.bin");
    let data = vec![0xA5u8; size];
    std::fs::write(&file_path, &data).unwrap();

    let hash = ArtifactHash::from_bytes(&data).unwrap();
    let metadata = ArtifactMetadata::new(hash, size as u64, "bin".to_string(), "bench".to_string());

    group.throughput(Throughput::Bytes(size as u64));

    group.bench_function("fs_read_copy", |b| {
        b.iter(|| {
            let bytes = std::fs::read(black_box(&file_path)).unwrap();
            let sum: u64 = black_box(&bytes).iter().map(|&x| x as u64).sum();
            black_box(sum);
        });
    });

    group.bench_function("mmap_zero_copy", |b| {
        b.iter(|| {
            let mmap =
                MmapArtifact::open(black_box(&file_path), black_box(metadata.clone())).unwrap();
            let sum: u64 = black_box(mmap.as_slice()).iter().map(|&x| x as u64).sum();
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_artifact_hashing,
    bench_zstd_compression,
    bench_zero_copy_reads
);
criterion_main!(benches);
