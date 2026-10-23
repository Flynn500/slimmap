use core::hash::{BuildHasher, Hash};

use super::SlimMap;
use crate::Equivalent;
use crate::layout::rehasher;

impl<K: Hash + Eq, V, S: BuildHasher> SlimMap<K, V, S> {
    /// Finds `key` and, if the map is indexed, removes its index slot.
    /// Returns the entry's position; the entry itself is left in place.
    pub(super) fn take_index<Q>(&mut self, key: &Q) -> Option<usize>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let (entries, table) = self.raw.parts_mut();
        match table {
            Some(table) => {
                let hash = self.hash_builder.hash_one(key);
                let slot = table
                    .find_entry(hash, |&i| key.equivalent(&entries[i as usize].0))
                    .ok()?;
                let (i, _) = slot.remove();
                Some(i as usize)
            }
            None => entries.iter().position(|(k, _)| key.equivalent(k)),
        }
    }

    /// After entry `i` was shift-removed, decrement every index slot that
    /// pointed past it.
    pub(super) fn decrement_indices_after(&mut self, i: usize) {
        let (entries, Some(table)) = self.raw.parts_mut() else {
            return;
        };
        let shifted = entries.len() - i;
        if shifted == 0 {
            return;
        }
        if shifted <= table.len() / 2 {
            // Few entries moved: look up each one. Entry `j` used to be at
            // `j + 1`. Each slot is updated before the next is searched, so
            // the value being searched for is always unique.
            for (j, (k, _)) in entries.iter().enumerate().skip(i) {
                let hash = self.hash_builder.hash_one(k);
                if let Some(slot) = table.find_mut(hash, |&x| x as usize == j + 1) {
                    *slot = j as u32;
                }
            }
        } else {
            // Most entries moved: one pass over the whole table is cheaper.
            for slot in table.iter_mut() {
                if *slot as usize > i {
                    *slot -= 1;
                }
            }
        }
    }

    /// Rebuilds the index from scratch to match the entries.
    pub(super) fn rebuild_index(&mut self) {
        let (entries, Some(table)) = self.raw.parts_mut() else {
            return;
        };
        let entries = &*entries;
        table.clear();
        for (i, (k, _)) in entries.iter().enumerate() {
            let hash = self.hash_builder.hash_one(k);
            table.insert_unique(hash, i as u32, rehasher(&self.hash_builder, entries));
        }
    }

    #[cfg(test)]
    pub(super) fn is_indexed(&self) -> bool {
        self.raw.is_indexed()
    }
}
