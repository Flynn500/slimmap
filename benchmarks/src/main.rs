use std::alloc::{GlobalAlloc, Layout, System};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{self, File};
use std::hash::Hash;
use std::hint::black_box;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use hashlink::LinkedHashMap;
use indexmap::IndexMap;
use litemap::LiteMap;
use ordermap::OrderMap;
use slimmap::SlimMap;
use smallmap::Map as SmallMap;

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
const STRING_SIZES: &[usize] = &[10, 100, 1_000, 10_000];
const REPEATS: usize = 5;
const MIN_OPS_PER_PASS: usize = 100_000;

/// The whole suite is run this many times and the results averaged.
const RUNS: usize = 10;

/// Seeds for the deterministic key, shuffle and miss streams.
const KEY_SEED: u64 = 0x9E37_79B9_7F4A_7C15;
const SHUFFLE_SEED: u64 = 0x2545_F491_4F6C_DD1D;
const MISS_SEED: u64 = 0xD1B5_4A32_D192_ED03;

/// "Many small maps" scenario: this many maps, each holding `entries` entries.
const MANY_MAPS: usize = 10_000;
const MANY_ENTRIES: &[usize] = &[4, 16, 64];

/// The common interface the benchmarks drive. Values are always `u64`; the key
/// type is an associated type so the same code runs with `u64` and `String`.
trait Map {
    type Key;
    const NAME: &'static str;
    fn new() -> Self;
    fn insert(&mut self, k: Self::Key, v: u64);
    fn get(&self, k: &Self::Key) -> Option<&u64>;
}

impl<K: Eq + Hash + Ord> Map for SlimMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "SlimMap";
    fn new() -> Self {
        SlimMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        SlimMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        SlimMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for IndexMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "IndexMap";
    fn new() -> Self {
        IndexMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        IndexMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        IndexMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for BTreeMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "BTreeMap";
    fn new() -> Self {
        BTreeMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        BTreeMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        BTreeMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for HashMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "HashMap";
    fn new() -> Self {
        HashMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        HashMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        HashMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for OrderMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "OrderMap";
    fn new() -> Self {
        OrderMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        OrderMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        OrderMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for LinkedHashMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "LinkedHashMap";
    fn new() -> Self {
        LinkedHashMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        LinkedHashMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        LinkedHashMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for LiteMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "LiteMap";
    fn new() -> Self {
        LiteMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        LiteMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        LiteMap::get(self, k)
    }
}

impl<K: Eq + Hash + Ord> Map for SmallMap<K, u64> {
    type Key = K;
    const NAME: &'static str = "SmallMap";
    fn new() -> Self {
        SmallMap::new()
    }
    fn insert(&mut self, k: K, v: u64) {
        SmallMap::insert(self, k, v);
    }
    fn get(&self, k: &K) -> Option<&u64> {
        SmallMap::get(self, k)
    }
}

struct Row {
    map: &'static str,
    n: usize,
    insert_ns_per_op: f64,
    get_ns_per_op: f64,
    mem_bytes: usize,
}

struct ManyRow {
    map: &'static str,
    maps: usize,
    entries_per_map: usize,
    insert_ns_per_op: f64,
    get_ns_per_op: f64,
    mem_bytes: usize,
}

/// Lookup-only result, used by the shuffled and 50%-miss scenarios.
struct GetRow {
    map: &'static str,
    n: usize,
    get_ns_per_op: f64,
}

/// Splitmix64 stream: deterministic, cheap, and good enough for benchmark keys.
struct SplitMix(u64);

impl SplitMix {
    fn new(seed: u64) -> Self {
        SplitMix(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(KEY_SEED);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// Deterministic pseudo-random keys.
fn random_keys(n: usize) -> Vec<u64> {
    let mut rng = SplitMix::new(KEY_SEED);
    (0..n).map(|_| rng.next_u64()).collect()
}

/// Deterministic string keys like `user:00a1b2c3d4e5f607` (21 bytes each).
fn random_string_keys(n: usize) -> Vec<String> {
    random_keys(n)
        .into_iter()
        .map(|k| format!("user:{k:016x}"))
        .collect()
}

/// A permutation of `keys` (Fisher-Yates driven by a fixed seed).
fn shuffled(keys: &[u64]) -> Vec<u64> {
    let mut v = keys.to_vec();
    let mut rng = SplitMix::new(SHUFFLE_SEED);
    for i in (1..v.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
    v
}

/// Lookup keys where exactly half the queries are absent from `keys`. Hits
/// and misses alternate, with hits taken in insertion order.
fn half_miss_queries(keys: &[u64]) -> Vec<u64> {
    let present: HashSet<u64> = keys.iter().copied().collect();
    let mut rng = SplitMix::new(MISS_SEED);
    let mut misses = Vec::with_capacity(keys.len() / 2 + 1);
    while misses.len() < keys.len() / 2 + 1 {
        let k = rng.next_u64();
        if !present.contains(&k) {
            misses.push(k);
        }
    }

    let mut misses = misses.into_iter();
    keys.iter()
        .enumerate()
        .map(|(i, &k)| {
            if i % 2 == 0 {
                k
            } else {
                misses.next().unwrap()
            }
        })
        .collect()
}

fn median(mut xs: Vec<f64>) -> f64 {
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    xs[xs.len() / 2]
}

/// Builds a map from `keys`. Each key is cloned into the map, so for `String`
/// keys the timed insert includes that allocation (as an owning map would).
fn build<M: Map>(keys: &[M::Key]) -> M
where
    M::Key: Clone,
{
    let mut m = M::new();
    for (i, k) in keys.iter().enumerate() {
        m.insert(k.clone(), i as u64);
    }
    m
}

fn measure<M: Map>(keys: &[M::Key]) -> Row
where
    M::Key: Clone,
{
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

    // Memory: heap held by the map, including owned keys for `String` maps.
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

/// Builds one map per `entries`-sized chunk of `keys`, collected into a `Vec`.
fn build_many<M: Map<Key = u64>>(keys: &[u64], entries: usize) -> Vec<M> {
    keys.chunks(entries).map(build::<M>).collect()
}

fn measure_many<M: Map<Key = u64>>(entries: usize) -> ManyRow {
    let keys = random_keys(MANY_MAPS * entries);
    let ops = keys.len() as f64;
    let mut insert_times = Vec::with_capacity(REPEATS);
    let mut get_times = Vec::with_capacity(REPEATS);

    for _ in 0..REPEATS {
        let start = Instant::now();
        black_box(build_many::<M>(&keys, entries));
        insert_times.push(start.elapsed().as_nanos() as f64 / ops);

        let maps = build_many::<M>(&keys, entries);
        let start = Instant::now();
        let mut sum = 0u64;
        for (map, chunk) in maps.iter().zip(keys.chunks(entries)) {
            for k in chunk {
                if let Some(v) = map.get(k) {
                    sum = sum.wrapping_add(*v);
                }
            }
        }
        get_times.push(start.elapsed().as_nanos() as f64 / ops);
        black_box(sum);
    }

    // Memory: heap held by the whole Vec of maps, including the Vec itself.
    let before = LIVE_BYTES.load(Ordering::Relaxed);
    let maps = black_box(build_many::<M>(&keys, entries));
    let mem_bytes = LIVE_BYTES.load(Ordering::Relaxed).saturating_sub(before);
    drop(maps);

    ManyRow {
        map: M::NAME,
        maps: MANY_MAPS,
        entries_per_map: entries,
        insert_ns_per_op: median(insert_times),
        get_ns_per_op: median(get_times),
        mem_bytes,
    }
}

/// Times lookups of `queries` against a map built from `keys`.
fn measure_gets<M: Map<Key = u64>>(keys: &[u64], queries: &[u64]) -> GetRow {
    let m = build::<M>(keys);
    let inner = (MIN_OPS_PER_PASS / queries.len()).max(1);
    let ops = (inner * queries.len()) as f64;
    let mut get_times = Vec::with_capacity(REPEATS);

    for _ in 0..REPEATS {
        let start = Instant::now();
        let mut sum = 0u64;
        for _ in 0..inner {
            for k in queries {
                if let Some(v) = m.get(k) {
                    sum = sum.wrapping_add(*v);
                }
            }
        }
        get_times.push(start.elapsed().as_nanos() as f64 / ops);
        black_box(sum);
    }

    GetRow {
        map: M::NAME,
        n: keys.len(),
        get_ns_per_op: median(get_times),
    }
}

/// Runs the size scenario for one map type, returning a row per size.
fn size_rows<M: Map<Key = u64>>(rows: &mut Vec<Row>) {
    for &size in SIZES {
        rows.push(measure::<M>(&random_keys(size)));
    }
}

fn string_rows<M: Map<Key = String>>(rows: &mut Vec<Row>) {
    for &size in STRING_SIZES {
        rows.push(measure::<M>(&random_string_keys(size)));
    }
}

fn many_rows<M: Map<Key = u64>>(rows: &mut Vec<ManyRow>) {
    for &entries in MANY_ENTRIES {
        rows.push(measure_many::<M>(entries));
    }
}

fn shuffled_rows<M: Map<Key = u64>>(rows: &mut Vec<GetRow>) {
    for &size in SIZES {
        let keys = random_keys(size);
        let queries = shuffled(&keys);
        rows.push(measure_gets::<M>(&keys, &queries));
    }
}

fn half_miss_rows<M: Map<Key = u64>>(rows: &mut Vec<GetRow>) {
    for &size in SIZES {
        let keys = random_keys(size);
        let queries = half_miss_queries(&keys);
        rows.push(measure_gets::<M>(&keys, &queries));
    }
}

/// Everything one pass of the suite produces.
struct RunResults {
    size: Vec<Row>,
    string: Vec<Row>,
    many: Vec<ManyRow>,
    shuffled: Vec<GetRow>,
    half_miss: Vec<GetRow>,
}

fn run_suite() -> RunResults {
    let mut size = Vec::new();
    size_rows::<SlimMap<u64, u64>>(&mut size);
    size_rows::<IndexMap<u64, u64>>(&mut size);
    size_rows::<BTreeMap<u64, u64>>(&mut size);
    size_rows::<HashMap<u64, u64>>(&mut size);
    size_rows::<OrderMap<u64, u64>>(&mut size);
    size_rows::<LinkedHashMap<u64, u64>>(&mut size);
    size_rows::<LiteMap<u64, u64>>(&mut size);
    size_rows::<SmallMap<u64, u64>>(&mut size);

    let mut string = Vec::new();
    string_rows::<SlimMap<String, u64>>(&mut string);
    string_rows::<IndexMap<String, u64>>(&mut string);
    string_rows::<BTreeMap<String, u64>>(&mut string);
    string_rows::<HashMap<String, u64>>(&mut string);
    string_rows::<OrderMap<String, u64>>(&mut string);
    string_rows::<LinkedHashMap<String, u64>>(&mut string);
    string_rows::<LiteMap<String, u64>>(&mut string);
    string_rows::<SmallMap<String, u64>>(&mut string);

    let mut many = Vec::new();
    many_rows::<SlimMap<u64, u64>>(&mut many);
    many_rows::<IndexMap<u64, u64>>(&mut many);
    many_rows::<BTreeMap<u64, u64>>(&mut many);
    many_rows::<HashMap<u64, u64>>(&mut many);
    many_rows::<OrderMap<u64, u64>>(&mut many);
    many_rows::<LinkedHashMap<u64, u64>>(&mut many);
    many_rows::<LiteMap<u64, u64>>(&mut many);
    many_rows::<SmallMap<u64, u64>>(&mut many);

    let mut shuffled = Vec::new();
    shuffled_rows::<SlimMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<IndexMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<BTreeMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<HashMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<OrderMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<LinkedHashMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<LiteMap<u64, u64>>(&mut shuffled);
    shuffled_rows::<SmallMap<u64, u64>>(&mut shuffled);

    let mut half_miss = Vec::new();
    half_miss_rows::<SlimMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<IndexMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<BTreeMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<HashMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<OrderMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<LinkedHashMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<LiteMap<u64, u64>>(&mut half_miss);
    half_miss_rows::<SmallMap<u64, u64>>(&mut half_miss);

    RunResults {
        size,
        string,
        many,
        shuffled,
        half_miss,
    }
}

/// Combines the `i`-th row across all runs. Every run produces the same rows
/// in the same order, so the rows line up by index.
fn average<T>(runs: &[&[T]], f: impl Fn(&[&T]) -> T) -> Vec<T> {
    (0..runs[0].len())
        .map(|i| {
            let column: Vec<&T> = runs.iter().map(|r| &r[i]).collect();
            f(&column)
        })
        .collect()
}

fn mean<T>(column: &[&T], f: impl Fn(&T) -> f64) -> f64 {
    column.iter().map(|r| f(r)).sum::<f64>() / column.len() as f64
}

fn print_row(r: &Row) {
    println!(
        "{:<10} n={:<7} insert {:>10.1} ns/op  get {:>10.1} ns/op  mem {:>12} B",
        r.map, r.n, r.insert_ns_per_op, r.get_ns_per_op, r.mem_bytes
    );
}

fn print_many_row(r: &ManyRow) {
    println!(
        "{:<10} maps={} x {:<3} entries  insert {:>10.1} ns/entry  get {:>10.1} ns/lookup  mem {:>12} B",
        r.map, r.maps, r.entries_per_map, r.insert_ns_per_op, r.get_ns_per_op, r.mem_bytes
    );
}

fn print_get_row(r: &GetRow) {
    println!(
        "{:<10} n={:<7} get {:>10.1} ns/op",
        r.map, r.n, r.get_ns_per_op
    );
}

/// Writes the size-scenario rows as CSV, one line per map and size.
fn write_size_csv(path: &Path, rows: &[Row]) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        "map,n,insert_ns_per_op,get_ns_per_op,mem_bytes,bytes_per_entry"
    )?;
    for r in rows {
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
    println!("Wrote {}", path.display());
    Ok(())
}

fn write_many_csv(path: &Path, rows: &[ManyRow]) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(
        file,
        "map,maps,entries_per_map,insert_ns_per_op,get_ns_per_op,mem_bytes,bytes_per_map"
    )?;
    for r in rows {
        writeln!(
            file,
            "{},{},{},{:.2},{:.2},{},{:.2}",
            r.map,
            r.maps,
            r.entries_per_map,
            r.insert_ns_per_op,
            r.get_ns_per_op,
            r.mem_bytes,
            r.mem_bytes as f64 / r.maps as f64
        )?;
    }
    println!("Wrote {}", path.display());
    Ok(())
}

fn write_get_csv(path: &Path, rows: &[GetRow]) -> io::Result<()> {
    let mut file = File::create(path)?;
    writeln!(file, "map,n,get_ns_per_op")?;
    for r in rows {
        writeln!(file, "{},{},{:.2}", r.map, r.n, r.get_ns_per_op)?;
    }
    println!("Wrote {}", path.display());
    Ok(())
}

fn main() -> io::Result<()> {
    let out_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("results");
    fs::create_dir_all(&out_dir)?;

    let mut runs = Vec::with_capacity(RUNS);
    for i in 1..=RUNS {
        println!("Run {i}/{RUNS}");
        runs.push(run_suite());
    }

    // Each run already reports a median per scenario, so the final number is
    // the mean of those per-run medians. Memory is averaged the same way.
    let size = average(
        &runs.iter().map(|r| r.size.as_slice()).collect::<Vec<_>>(),
        |c| Row {
            map: c[0].map,
            n: c[0].n,
            insert_ns_per_op: mean(c, |r| r.insert_ns_per_op),
            get_ns_per_op: mean(c, |r| r.get_ns_per_op),
            mem_bytes: mean(c, |r| r.mem_bytes as f64).round() as usize,
        },
    );
    let string = average(
        &runs.iter().map(|r| r.string.as_slice()).collect::<Vec<_>>(),
        |c| Row {
            map: c[0].map,
            n: c[0].n,
            insert_ns_per_op: mean(c, |r| r.insert_ns_per_op),
            get_ns_per_op: mean(c, |r| r.get_ns_per_op),
            mem_bytes: mean(c, |r| r.mem_bytes as f64).round() as usize,
        },
    );
    let many = average(
        &runs.iter().map(|r| r.many.as_slice()).collect::<Vec<_>>(),
        |c| ManyRow {
            map: c[0].map,
            maps: c[0].maps,
            entries_per_map: c[0].entries_per_map,
            insert_ns_per_op: mean(c, |r| r.insert_ns_per_op),
            get_ns_per_op: mean(c, |r| r.get_ns_per_op),
            mem_bytes: mean(c, |r| r.mem_bytes as f64).round() as usize,
        },
    );
    let shuffled = average(
        &runs.iter().map(|r| r.shuffled.as_slice()).collect::<Vec<_>>(),
        |c| GetRow {
            map: c[0].map,
            n: c[0].n,
            get_ns_per_op: mean(c, |r| r.get_ns_per_op),
        },
    );
    let half_miss = average(
        &runs.iter().map(|r| r.half_miss.as_slice()).collect::<Vec<_>>(),
        |c| GetRow {
            map: c[0].map,
            n: c[0].n,
            get_ns_per_op: mean(c, |r| r.get_ns_per_op),
        },
    );

    println!("\nAverage of {RUNS} runs\n\nSize scenario (u64 keys):");
    size.iter().for_each(print_row);
    println!("\nString keys:");
    string.iter().for_each(print_row);
    println!("\nMany small maps:");
    many.iter().for_each(print_many_row);
    println!("\nShuffled lookups (u64 keys, lookups in shuffled order):");
    shuffled.iter().for_each(print_get_row);
    println!("\n50% misses (u64 keys, half of lookups absent):");
    half_miss.iter().for_each(print_get_row);
    println!();

    write_size_csv(&out_dir.join("results.csv"), &size)?;
    write_size_csv(&out_dir.join("string_keys.csv"), &string)?;
    write_many_csv(&out_dir.join("many_small.csv"), &many)?;
    write_get_csv(&out_dir.join("shuffled.csv"), &shuffled)?;
    write_get_csv(&out_dir.join("half_miss.csv"), &half_miss)?;
    Ok(())
}
