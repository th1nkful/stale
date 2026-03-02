use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use sha2::{Digest, Sha256};
use xxhash_rust::xxh3::Xxh3;

/// Generate deterministic test data of the given size.
fn make_data(size: usize) -> Vec<u8> {
    (0..size).map(|i| (i % 251) as u8).collect()
}

fn bench_hash_algorithms(c: &mut Criterion) {
    let sizes: &[usize] = &[
        1_024,         // 1 KB  — typical small config file
        64 * 1_024,    // 64 KB — medium source file
        1_024 * 1_024, // 1 MB — large file
    ];

    let mut group = c.benchmark_group("hash_algorithm");

    for &size in sizes {
        let data = make_data(size);
        let label = match size {
            1_024 => "1KB",
            65_536 => "64KB",
            1_048_576 => "1MB",
            _ => unreachable!(),
        };

        group.bench_with_input(BenchmarkId::new("sha256", label), &data, |b, data| {
            b.iter(|| {
                let mut hasher = Sha256::new();
                hasher.update(black_box(data));
                hex::encode(hasher.finalize())
            });
        });

        group.bench_with_input(BenchmarkId::new("xxhash3_128", label), &data, |b, data| {
            b.iter(|| {
                let mut hasher = Xxh3::new();
                hasher.update(black_box(data));
                format!("{:032x}", hasher.digest128())
            });
        });
    }

    group.finish();
}

fn bench_multi_file_simulation(c: &mut Criterion) {
    // Simulate hashing 50 files of ~10 KB each (typical source tree).
    let files: Vec<Vec<u8>> = (0..50).map(|i| make_data(10_000 + i * 100)).collect();
    let paths: Vec<String> = (0..50).map(|i| format!("src/module_{i}/file.rs")).collect();

    let mut group = c.benchmark_group("multi_file");

    group.bench_function("sha256_50_files", |b| {
        b.iter(|| {
            let mut hasher = Sha256::new();
            for (path, data) in paths.iter().zip(files.iter()) {
                hasher.update(path.as_bytes());
                hasher.update(b"\0");
                hasher.update(black_box(data));
                hasher.update(b"\0");
            }
            hex::encode(hasher.finalize())
        });
    });

    group.bench_function("xxhash3_128_50_files", |b| {
        b.iter(|| {
            let mut hasher = Xxh3::new();
            for (path, data) in paths.iter().zip(files.iter()) {
                hasher.update(path.as_bytes());
                hasher.update(b"\0");
                hasher.update(black_box(data));
                hasher.update(b"\0");
            }
            format!("{:032x}", hasher.digest128())
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hash_algorithms, bench_multi_file_simulation);
criterion_main!(benches);
