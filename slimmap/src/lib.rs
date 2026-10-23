#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlimMap<K, V> {
    entries: Vec<(K, V)>,
}

impl<K: PartialEq, V> SlimMap<K, V> {
    /// Creates an empty map.
    pub fn new() -> Self {
        SlimMap {
            entries: Vec::new(),
        }
    }

    /// Returns the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn position(&self, key: &K) -> Option<usize> {
        self.entries.iter().position(|(k, _)| k == key)
    }

    /// Inserts a key-value pair. If the key was already present, its value is
    /// replaced and the old value is returned.
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.position(&key) {
            Some(i) => Some(std::mem::replace(&mut self.entries[i].1, value)),
            None => {
                self.entries.push((key, value));
                None
            }
        }
    }

    /// Returns a reference to the value for `key`, if present.
    pub fn get(&self, key: &K) -> Option<&V> {
        self.position(key).map(|i| &self.entries[i].1)
    }

    /// Returns a mutable reference to the value for `key`, if present.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.position(key).map(move |i| &mut self.entries[i].1)
    }

    /// Returns `true` if the map contains `key`.
    pub fn contains_key(&self, key: &K) -> bool {
        self.position(key).is_some()
    }

    /// Removes `key` and returns its value, if present.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let i = self.position(key)?;
        let (_, value) = self.entries.swap_remove(i);
        Some(value)
    }

    /// Removes all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Iterates over `(key, value)` pairs in insertion order (until a removal
    /// reorders them).
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(k, v)| (k, v))
    }
}
