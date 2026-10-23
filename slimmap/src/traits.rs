use core::fmt;
use core::hash::{BuildHasher, Hash, Hasher};
use core::ops::Index;

use crate::Equivalent;
use crate::map::SlimMap;

impl<K, V, S: Default> Default for SlimMap<K, V, S> {
    fn default() -> Self {
        Self::with_hasher(S::default())
    }
}

impl<K: Clone, V: Clone, S: Clone> Clone for SlimMap<K, V, S> {
    fn clone(&self) -> Self {
        let mut out = Self::with_capacity_and_hasher(self.len(), self.hash_builder.clone());
        for (k, v) in self.iter() {
            // `len` is bumped per entry, so a panicking `clone` drops
            // exactly the entries copied so far.
            out.raw.push(k.clone(), v.clone());
        }
        // Entries are in the same order, so the index can be copied as-is.
        // `out` is indexed only if `len >= INDEX_MIN_CAP`, and then so is
        // `self`, since its capacity is at least `len`.
        if let (Some(src), (_, Some(dst))) = (self.raw.table(), out.raw.parts_mut()) {
            *dst = src.clone();
        }
        out
    }
}

impl<K: fmt::Debug, V: fmt::Debug, S> fmt::Debug for SlimMap<K, V, S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

/// Order-sensitive: maps are equal only if their entries match in order.
impl<K: PartialEq, V: PartialEq, S> PartialEq for SlimMap<K, V, S> {
    fn eq(&self, other: &Self) -> bool {
        self.raw.entries() == other.raw.entries()
    }
}

impl<K: Eq, V: Eq, S> Eq for SlimMap<K, V, S> {}

/// Lexicographic over entries in order.
impl<K: PartialOrd, V: PartialOrd, S> PartialOrd for SlimMap<K, V, S> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        self.raw.entries().partial_cmp(other.raw.entries())
    }
}

impl<K: Ord, V: Ord, S> Ord for SlimMap<K, V, S> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.raw.entries().cmp(other.raw.entries())
    }
}

/// Order-sensitive, consistent with `PartialEq`.
impl<K: Hash, V: Hash, S> Hash for SlimMap<K, V, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.raw.entries().hash(state);
    }
}

impl<K, V, Q, S> Index<&Q> for SlimMap<K, V, S>
where
    Q: ?Sized + Hash + Equivalent<K>,
    S: BuildHasher,
{
    type Output = V;

    /// # Panics
    /// Panics if `key` is not present.
    fn index(&self, key: &Q) -> &V {
        self.get(key).expect("key not found in SlimMap")
    }
}

impl<K: Hash + Eq, V, S: BuildHasher + Default> FromIterator<(K, V)> for SlimMap<K, V, S> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        let mut map = Self::default();
        map.extend(iter);
        map
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> Extend<(K, V)> for SlimMap<K, V, S> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        let hint = iter.size_hint().0;
        if self.is_empty() {
            // Collecting into a fresh map: size it exactly.
            self.reserve_exact(hint);
        } else {
            self.reserve(hint);
        }
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

impl<'a, K, V, S> Extend<(&'a K, &'a V)> for SlimMap<K, V, S>
where
    K: Hash + Eq + Copy,
    V: Copy,
    S: BuildHasher,
{
    fn extend<I: IntoIterator<Item = (&'a K, &'a V)>>(&mut self, iter: I) {
        self.extend(iter.into_iter().map(|(k, v)| (*k, *v)));
    }
}

impl<K: Hash + Eq, V, const N: usize> From<[(K, V); N]> for SlimMap<K, V> {
    fn from(entries: [(K, V); N]) -> Self {
        Self::from_iter(entries)
    }
}
