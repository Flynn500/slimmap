mod capacity;
mod index;
mod lookup;
mod mutate;

use crate::hasher::DefaultHashBuilder;
use crate::iter::{Iter, IterMut, Keys, Values, ValuesMut};
use crate::raw::RawBuf;

/// An insertion-ordered map that is one pointer wide.
///
/// Small maps are searched linearly; once the capacity reaches a threshold,
/// the map also keeps a hash index so lookups stay fast as it grows.
pub struct SlimMap<K, V, S = DefaultHashBuilder> {
    pub(crate) raw: RawBuf<K, V>,
    pub(crate) hash_builder: S,
}

impl<K, V> SlimMap<K, V> {
    /// Creates an empty map. This does not allocate.
    pub const fn new() -> Self {
        Self::with_hasher(DefaultHashBuilder)
    }

    /// Creates an empty map with room for exactly `capacity` entries.
    ///
    /// # Panics
    /// Panics if `capacity` exceeds `u32::MAX` or the allocation size
    /// overflows.
    pub fn with_capacity(capacity: usize) -> Self {
        Self::with_capacity_and_hasher(capacity, DefaultHashBuilder)
    }
}

impl<K, V, S> SlimMap<K, V, S> {
    /// Creates an empty map that will use `hash_builder` once it is large
    /// enough to need a hash index. This does not allocate.
    pub const fn with_hasher(hash_builder: S) -> Self {
        SlimMap {
            raw: RawBuf::new(),
            hash_builder,
        }
    }

    /// Creates an empty map with room for exactly `capacity` entries, using
    /// `hash_builder` for its index.
    ///
    /// # Panics
    /// Panics if `capacity` exceeds `u32::MAX` or the allocation size
    /// overflows.
    pub fn with_capacity_and_hasher(capacity: usize, hash_builder: S) -> Self {
        let mut map = Self::with_hasher(hash_builder);
        if capacity > 0 {
            // The map is empty, so there are no keys to hash.
            map.raw
                .realloc_to(capacity, |_| unreachable!("no keys to hash"));
        }
        map
    }

    /// Returns a reference to the map's hasher builder.
    pub fn hasher(&self) -> &S {
        &self.hash_builder
    }

    /// Returns the number of entries.
    #[inline]
    pub fn len(&self) -> usize {
        self.raw.len()
    }

    /// Returns `true` if the map has no entries.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of entries the map can hold without reallocating.
    #[inline]
    pub fn capacity(&self) -> usize {
        self.raw.cap()
    }

    /// Returns the entry at position `index` in insertion order.
    pub fn get_index(&self, index: usize) -> Option<(&K, &V)> {
        self.raw.entries().get(index).map(|(k, v)| (k, v))
    }

    /// Returns the first entry in insertion order.
    pub fn first(&self) -> Option<(&K, &V)> {
        self.raw.entries().first().map(|(k, v)| (k, v))
    }

    /// Returns the last entry in insertion order.
    pub fn last(&self) -> Option<(&K, &V)> {
        self.raw.entries().last().map(|(k, v)| (k, v))
    }

    /// Removes all entries, keeping the allocation.
    pub fn clear(&mut self) {
        self.raw.clear();
    }

    /// Iterates over `(key, value)` pairs in order.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            inner: self.raw.entries().iter(),
        }
    }

    /// Iterates over `(key, &mut value)` pairs in order.
    pub fn iter_mut(&mut self) -> IterMut<'_, K, V> {
        IterMut {
            inner: self.raw.entries_mut().iter_mut(),
        }
    }

    /// Iterates over keys in order.
    pub fn keys(&self) -> Keys<'_, K, V> {
        Keys {
            inner: self.raw.entries().iter(),
        }
    }

    /// Iterates over values in order.
    pub fn values(&self) -> Values<'_, K, V> {
        Values {
            inner: self.raw.entries().iter(),
        }
    }

    /// Iterates mutably over values in order.
    pub fn values_mut(&mut self) -> ValuesMut<'_, K, V> {
        ValuesMut {
            inner: self.raw.entries_mut().iter_mut(),
        }
    }
}
