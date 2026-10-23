use core::hash::{BuildHasher, Hash};

use super::SlimMap;
use crate::Equivalent;
use crate::layout::rehasher;

impl<K: Hash + Eq, V, S: BuildHasher> SlimMap<K, V, S> {
    /// Inserts a key-value pair. If the key was already present, its value is
    /// replaced (keeping its position and the original key) and the old value
    /// is returned. Otherwise the entry is appended at the end.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let mut hash = None;
        let found = {
            let entries = self.raw.entries();
            match self.raw.table() {
                Some(table) => {
                    let h = self.hash_builder.hash_one(&key);
                    hash = Some(h);
                    table
                        .find(h, |&i| key.equivalent(&entries[i as usize].0))
                        .map(|&i| i as usize)
                }
                None => entries.iter().position(|(k, _)| key.equivalent(k)),
            }
        };
        if let Some(i) = found {
            return Some(core::mem::replace(&mut self.raw.entries_mut()[i].1, value));
        }

        // May grow the allocation and build the index for the first time.
        self.reserve(1);

        if !self.raw.is_indexed() {
            self.raw.push(key, value);
            return None;
        }

        let h = hash.unwrap_or_else(|| self.hash_builder.hash_one(&key));
        let i = self.len();
        // Grow the index first: this may rehash (running user code), and
        // nothing has changed yet if it panics.
        if let (entries, Some(table)) = self.raw.parts_mut() {
            table.reserve(1, rehasher(&self.hash_builder, entries));
        }
        self.raw.push(key, value);
        if let (entries, Some(table)) = self.raw.parts_mut() {
            // Room was reserved, so this does not rehash.
            table.insert_unique(h, i as u32, rehasher(&self.hash_builder, entries));
        }
        None
    }

    /// Removes `key` and returns its value, preserving the order of the
    /// remaining entries. This is O(n); see [`swap_remove`](Self::swap_remove)
    /// for an O(1) alternative that reorders.
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.remove_entry(key).map(|(_, v)| v)
    }

    /// Removes `key` and returns the stored key and value, preserving the
    /// order of the remaining entries.
    pub fn remove_entry<Q>(&mut self, key: &Q) -> Option<(K, V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let i = self.take_index(key)?;
        let entry = self.raw.shift_remove(i);
        self.decrement_indices_after(i);
        Some(entry)
    }

    /// Removes `key` by moving the last entry into its place. This is O(1)
    /// but changes the position of the last entry.
    pub fn swap_remove<Q>(&mut self, key: &Q) -> Option<V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let i = self.take_index(key)?;
        let last = self.len() - 1;
        if i != last {
            // Repoint the last entry's index slot at its new position.
            if let (entries, Some(table)) = self.raw.parts_mut() {
                let hash = self.hash_builder.hash_one(&entries[last].0);
                if let Some(slot) = table.find_mut(hash, |&x| x as usize == last) {
                    *slot = i as u32;
                }
            }
        }
        Some(self.raw.swap_remove(i).1)
    }

    /// Removes and returns the last entry.
    pub fn pop(&mut self) -> Option<(K, V)> {
        let last = self.len().checked_sub(1)?;
        if let (entries, Some(table)) = self.raw.parts_mut() {
            let hash = self.hash_builder.hash_one(&entries[last].0);
            if let Ok(slot) = table.find_entry(hash, |&x| x as usize == last) {
                slot.remove();
            }
        }
        self.raw.pop()
    }

    /// Keeps only the entries for which `keep` returns `true`, preserving
    /// their order.
    pub fn retain<F>(&mut self, keep: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        // Rebuild the index afterwards, even if `keep` panics partway.
        struct Rebuild<'a, K: Hash + Eq, V, S: BuildHasher> {
            map: &'a mut SlimMap<K, V, S>,
            len_before: usize,
        }
        impl<K: Hash + Eq, V, S: BuildHasher> Drop for Rebuild<'_, K, V, S> {
            fn drop(&mut self) {
                if self.map.len() != self.len_before {
                    self.map.rebuild_index();
                }
            }
        }
        let len_before = self.len();
        let guard = Rebuild {
            map: self,
            len_before,
        };
        guard.map.raw.retain(keep);
    }
}
