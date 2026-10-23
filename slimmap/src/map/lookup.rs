use core::hash::{BuildHasher, Hash};

use super::SlimMap;
use crate::Equivalent;

impl<K, V, S: BuildHasher> SlimMap<K, V, S> {
    /// Returns a reference to the value for `key`, if present.
    pub fn get<Q>(&self, key: &Q) -> Option<&V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.find_entry(key).map(|e| &e.1)
    }

    /// Returns the stored key and a reference to the value for `key`, if
    /// present.
    pub fn get_key_value<Q>(&self, key: &Q) -> Option<(&K, &V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.find_entry(key).map(|e| (&e.0, &e.1))
    }

    /// Returns a mutable reference to the value for `key`, if present.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.get_index_of(key)
            .map(move |i| &mut self.raw.entries_mut()[i].1)
    }

    /// Returns `true` if the map contains `key`.
    pub fn contains_key<Q>(&self, key: &Q) -> bool
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        self.find_entry(key).is_some()
    }

    /// Returns the position of `key` in insertion order, if present.
    #[inline]
    pub fn get_index_of<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        if self.raw.is_indexed() {
            self.get_index_of_hashed(key)
        } else {
            self.raw
                .entries()
                .iter()
                .position(|(k, _)| key.equivalent(k))
        }
    }

    /// Finds the entry for `key`. Small maps scan inline; the hashed path
    /// lives in its own function so its register pressure and stack frame
    /// don't burden the scan.
    #[inline]
    fn find_entry<Q>(&self, key: &Q) -> Option<&(K, V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        if self.raw.is_indexed() {
            self.find_entry_hashed(key)
        } else {
            let entries = self.raw.entries();
            // `position` + index compiles to a tighter scan loop than `find`.
            entries
                .iter()
                .position(|(k, _)| key.equivalent(k))
                .map(|i| &entries[i])
        }
    }

    #[inline(never)]
    fn get_index_of_hashed<Q>(&self, key: &Q) -> Option<usize>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let entries = self.raw.entries();
        let table = self.raw.table()?;
        let hash = self.hash_builder.hash_one(key);
        table
            .find(hash, |&i| key.equivalent(&entries[i as usize].0))
            .map(|&i| i as usize)
    }

    #[inline(never)]
    fn find_entry_hashed<Q>(&self, key: &Q) -> Option<&(K, V)>
    where
        Q: ?Sized + Hash + Equivalent<K>,
    {
        let entries = self.raw.entries();
        let table = self.raw.table()?;
        let hash = self.hash_builder.hash_one(key);
        table
            .find(hash, |&i| key.equivalent(&entries[i as usize].0))
            .map(|&i| &entries[i as usize])
    }
}
