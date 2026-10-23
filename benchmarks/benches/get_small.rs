//! Focused `get` benchmarks for `SlimMap<u64, u64>`: small unindexed maps
//! (hot cache) plus one indexed map to guard the hashed path.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use slimmap::SlimMap;

fn keys(n: usize) -> Vec<u64> {
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    (0..n)
        .map(|_| {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^ (z >> 31)
        })
        .collect()
}

fn build(keys: &[u64]) -> SlimMap<u64, u64> {
    let mut m = SlimMap::new();
    for (i, &k) in keys.iter().enumerate() {
        m.insert(k, i as u64);
    }
    m
}

fn bench_get(c: &mut Criterion) {
    let mut g = c.benchmark_group("get_hot");
    for n in [4usize, 10, 16, 1000] {
        let ks = keys(n);
        let m = build(&ks);
        g.bench_function(format!("n={n}"), |b| {
            b.iter(|| {
                let mut sum = 0u64;
                for k in &ks {
                    if let Some(v) = black_box(&m).get(black_box(k)) {
                        sum = sum.wrapping_add(*v);
                    }
                }
                sum
            })
        });
    }
    g.finish();

    // Many small maps: each lookup touches a different allocation.
    let mut g = c.benchmark_group("get_many");
    for n in [4usize, 16] {
        let ks = keys(10_000 * n);
        let maps: Vec<_> = ks.chunks(n).map(build).collect();
        g.bench_function(format!("10000x{n}"), |b| {
            b.iter(|| {
                let mut sum = 0u64;
                for (m, chunk) in maps.iter().zip(ks.chunks(n)) {
                    for k in chunk {
                        if let Some(v) = black_box(m).get(black_box(k)) {
                            sum = sum.wrapping_add(*v);
                        }
                    }
                }
                sum
            })
        });
    }
    g.finish();
}

criterion_group!(benches, bench_get);
criterion_main!(benches);
