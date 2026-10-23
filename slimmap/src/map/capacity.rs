use core::hash::{BuildHasher, Hash};

use super::SlimMap;
use crate::MIN_NON_ZERO_CAP;
use crate::layout::is_indexed_cap;
use crate::layout::rehasher;

impl<K: Hash + Eq, V, S: BuildHasher> SlimMap<K, V, S> {
    /// Reserves room for at least `additional` more entries. Capacity grows
    /// by at least 1.5× to keep repeated insertion amortized O(1).
    ///
    /// # Panics
    /// Panics if the new capacity exceeds `u32::MAX` or the allocation size
    /// overflows.
    pub fn reserve(&mut self, additional: usize) {
        let required = self
            .len()
            .checked_add(additional)
            .expect("capacity overflow");
        let cap = self.capacity();
        if required <= cap {
            return;
        }
        let grown = cap + cap / 2;
        let new_cap = required
            .max(grown)
            .max(MIN_NON_ZERO_CAP)
            // Don't let growth push us past the u32 limit if `required` fits.
            .min(u32::MAX as usize)
            .max(required);
        self.realloc_to(new_cap);
    }

    /// Reserves room for exactly `additional` more entries, without
    /// over-allocating.
    ///
    /// # Panics
    /// As for [`reserve`](Self::reserve).
    pub fn reserve_exact(&mut self, additional: usize) {
        let required = self
            .len()
            .checked_add(additional)
            .expect("capacity overflow");
        if required > self.capacity() {
            self.realloc_to(required);
        }
    }

    /// Shrinks the capacity to `max(len, min_capacity)`, if that is smaller
    /// than the current capacity. Maps that shrink below the indexing
    /// threshold drop their hash index.
    pub fn shrink_to(&mut self, min_capacity: usize) {
        let target = self.len().max(min_capacity);
        if target >= self.capacity() {
            return;
        }
        if target == 0 {
            self.raw.free();
            return;
        }
        if is_indexed_cap(target) {
            if let (entries, Some(table)) = self.raw.parts_mut() {
                table.shrink_to(target, rehasher(&self.hash_builder, entries));
            }
        }
        self.realloc_to(target);
    }

    /// Shrinks the capacity to exactly the number of entries. An empty map
    /// releases its allocation entirely.
    pub fn shrink_to_fit(&mut self) {
        self.shrink_to(0);
    }
    // -- private helpers ----------------------------------------------------

    fn realloc_to(&mut self, new_cap: usize) {
        let hash_builder = &self.hash_builder;
        self.raw.realloc_to(new_cap, |k| hash_builder.hash_one(k));
    }
}
