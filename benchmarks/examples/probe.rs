use slimmap::SlimMap;

#[unsafe(no_mangle)]
#[inline(never)]
pub fn probe_get<'a>(m: &'a SlimMap<u64, u64>, k: &u64) -> Option<&'a u64> {
    m.get(k)
}

fn main() {
    let mut m = SlimMap::new();
    m.insert(1u64, 2u64);
    println!("{:?}", probe_get(&m, &1));
}
