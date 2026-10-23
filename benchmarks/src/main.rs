use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::hint::black_box;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use indexmap::IndexMap;
use slimmap::SlimMap;

/// Global allocator wrapper that tracks live heap bytes, so each map's
/// footprint can be measured as a delta.
struct CountingAlloc;

static LIVE_BYTES: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            LIVE_BYTES.fetch_add(layout.size(), Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
        if !new_ptr.is_null() {
            LIVE_BYTES.fetch_add(new_size, Ordering::Relaxed);
            LIVE_BYTES.fetch_sub(layout.size(), Ordering::Relaxed);
        }
        new_ptr
    }
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

const SIZES: &[usize] = &[10, 100, 1_000, 10_000, 100_000];
const REPEATS: usize = 5;
const MIN_OPS_PER_PASS: usize = 100_000;

trait Map {
    const NAME: &'static str;
    fn new() -> Self;
    fn insert(&mut self, k: u64, v: u64);
    fn get(&self, k: &u64) -> Option<&u64>;
}

impl Map for SlimMap<u64, u64> {
    const NAME: &'static str = "SlimMap";
    fn new() -> Self {
        SlimMap::new()
    }
    fn insert(&mut self, k: u64, v: u64) {
        SlimMap::insert(self, k, v);
    }
    fn get(&self, k: &u64) -> Option<&u64> {
        SlimMap::get(self, k)
    }
}

impl Map for IndexMap<u64, u64> {
    const NAME: &'static str = "IndexMap";
    fn new() -> Self {
        IndexMap::new()
    }
    fn insert(&mut self, k: u64, v: u64) {
        IndexMap::insert(self, k, v);
    }
    fn get(&self, k: &u64) -> Option<&u64> {
        IndexMap::get(self, k)
    }
}

impl Map for BTreeMap<u64, u64> {
    const NAME: &'static str = "BTreeMap";
    fn new() -> Self {
        BTreeMap::new()
    }
    fn insert(&mut self, k: u64, v: u64) {
        BTreeMap::insert(self, k, v);
    }
    fn get(&self, k: &u64) -> Option<&u64> {
        BTreeMap::get(self, k)
    }
}

impl Map for HashMap<u64, u64> {
    const NAME: &'static str = "HashMap";
    fn new() -> Self {
        HashMap::new()
    }
    fn insert(&mut self, k: u64, v: u64) {
        HashMap::insert(self, k, v);
    }
    fn get(&self, k: &u64) -> Option<&u64> {
        HashMap::get(self, k)
    }
}

struct Row {
    map: &'static str,
    n: usize,
    insert_ns_per_op: f64,
    get_ns_per_op: f64,
    mem_bytes: usize,
}

/// Deterministic pseudo-random keys (splitmix64)
fn random_keys(n: usize) -> Vec<u64> {
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

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

fn build<M: Map>(keys: &[u64]) -> M {
    let mut m = M::new();
    for &k in keys {
        m.insert(k, k);
    }
    m
}

fn measure<M: Map>(keys: &[u64]) -> Row {
    let inner = (MIN_OPS_PER_PASS / keys.len()).max(1);
    let ops = (inner * keys.len()) as f64;
    let mut insert_times = Vec::with_capacity(REPEATS);
    let mut get_times = Vec::with_capacity(REPEATS);

    for _ in 0..REPEATS {
        let start = Instant::now();
        for _ in 0..inner {
            black_box(build::<M>(keys));
        }
        insert_times.push(start.elapsed().as_nanos() as f64 / ops);

        let m = build::<M>(keys);
        let start = Instant::now();
        let mut sum = 0u64;
        for _ in 0..inner {
            for k in keys {
                if let Some(v) = m.get(k) {
                    sum = sum.wrapping_add(*v);
                }
            }
        }
        get_times.push(start.elapsed().as_nanos() as f64 / ops);
        black_box(sum);
    }

    let before = LIVE_BYTES.load(Ordering::Relaxed);
    let m = black_box(build::<M>(keys));
    let mem_bytes = LIVE_BYTES.load(Ordering::Relaxed).saturating_sub(before);
    drop(m);

    Row {
        map: M::NAME,
        n: keys.len(),
        insert_ns_per_op: median(insert_times),
        get_ns_per_op: median(get_times),
        mem_bytes,
    }
}

fn run<M: Map>(rows: &mut Vec<Row>, size: usize, keys: &[u64]) {
    let row = measure::<M>(keys);
    println!(
        "{:<10} n={:<7} insert {:>10.1} ns/op  get {:>10.1} ns/op  mem {:>12} B",
        row.map, row.n, row.insert_ns_per_op, row.get_ns_per_op, row.mem_bytes
    );
    rows.push(row);
}

fn main() -> std::io::Result<()> {
    let mut rows = Vec::new();

    for &size in SIZES {
        let keys = random_keys(size);
        run::<SlimMap<u64, u64>>(&mut rows, size, &keys);
        run::<IndexMap<u64, u64>>(&mut rows, size, &keys);
        run::<BTreeMap<u64, u64>>(&mut rows, size, &keys);
        run::<HashMap<u64, u64>>(&mut rows, size, &keys);
    }

    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("results");
    fs::create_dir_all(&out_dir)?;
    let out_path = out_dir.join("results.csv");
    let mut file = File::create(&out_path)?;
    writeln!(
        file,
        "map,n,insert_ns_per_op,get_ns_per_op,mem_bytes,bytes_per_entry"
    )?;
    for r in &rows {
        writeln!(
            file,
            "{},{},{:.2},{:.2},{},{:.2}",
            r.map,
            r.n,
            r.insert_ns_per_op,
            r.get_ns_per_op,
            r.mem_bytes,
            r.mem_bytes as f64 / r.n as f64
        )?;
    }
    println!("\nWrote {}", out_path.display());
    Ok(())
}
