use core::fmt;
use core::hash::{BuildHasher, Hasher};

/// Zero-sized hasher builder used by default, so it adds nothing to the size
/// of a [`SlimMap`](crate::SlimMap).
///
/// It uses foldhash with a random seed chosen once per process. Iteration
/// order never depends on hashes, so sharing one seed across maps doesn't
/// expose the quadratic-merge problems that per-map seeds guard against in
/// unordered hash maps.
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultHashBuilder;

/// Hasher created by [`DefaultHashBuilder`].
#[derive(Clone)]
pub struct DefaultHasher(foldhash::fast::FoldHasher<'static>);

impl BuildHasher for DefaultHashBuilder {
    type Hasher = DefaultHasher;

    #[inline]
    fn build_hasher(&self) -> DefaultHasher {
        // Arbitrary constant; the randomness comes from the shared seed.
        const PER_HASHER_SEED: u64 = 0x243f_6a88_85a3_08d3;
        DefaultHasher(foldhash::fast::FoldHasher::with_seed(
            PER_HASHER_SEED,
            foldhash::SharedSeed::global_random(),
        ))
    }
}

impl Hasher for DefaultHasher {
    #[inline]
    fn finish(&self) -> u64 {
        self.0.finish()
    }
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        self.0.write(bytes)
    }
    #[inline]
    fn write_u8(&mut self, i: u8) {
        self.0.write_u8(i)
    }
    #[inline]
    fn write_u16(&mut self, i: u16) {
        self.0.write_u16(i)
    }
    #[inline]
    fn write_u32(&mut self, i: u32) {
        self.0.write_u32(i)
    }
    #[inline]
    fn write_u64(&mut self, i: u64) {
        self.0.write_u64(i)
    }
    #[inline]
    fn write_u128(&mut self, i: u128) {
        self.0.write_u128(i)
    }
    #[inline]
    fn write_usize(&mut self, i: usize) {
        self.0.write_usize(i)
    }
}

impl fmt::Debug for DefaultHasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DefaultHasher").finish_non_exhaustive()
    }
}
