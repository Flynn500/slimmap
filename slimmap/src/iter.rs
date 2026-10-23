use core::iter::FusedIterator;
use core::ptr;
use core::slice;

use crate::map::SlimMap;
use crate::raw::RawBuf;

/// Owning iterator over `(K, V)`, created by `SlimMap::into_iter`.
pub struct IntoIter<K, V> {
    /// Its header `len` is stale while iterating: entries `start..end` are
    /// the ones still owned. `Drop` restores a consistent state.
    raw: RawBuf<K, V>,
    start: usize,
    end: usize,
}

impl<K, V, S> IntoIterator for SlimMap<K, V, S> {
    type Item = (K, V);
    type IntoIter = IntoIter<K, V>;
    fn into_iter(self) -> Self::IntoIter {
        let SlimMap { raw, .. } = self;
        let end = raw.len();
        IntoIter { raw, start: 0, end }
    }
}

impl<K, V> Iterator for IntoIter<K, V> {
    type Item = (K, V);

    #[inline]
    fn next(&mut self) -> Option<(K, V)> {
        if self.start == self.end {
            return None;
        }
        let i = self.start;
        self.start += 1;
        // SAFETY: `i` was in `start..end`, so the entry is initialized and
        // we take ownership of it exactly once.
        Some(unsafe { ptr::read(self.raw.data().add(i)) })
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.end - self.start;
        (n, Some(n))
    }
}

impl<K, V> DoubleEndedIterator for IntoIter<K, V> {
    #[inline]
    fn next_back(&mut self) -> Option<(K, V)> {
        if self.start == self.end {
            return None;
        }
        self.end -= 1;
        // SAFETY: as in `next`.
        Some(unsafe { ptr::read(self.raw.data().add(self.end)) })
    }
}

impl<K, V> ExactSizeIterator for IntoIter<K, V> {}
impl<K, V> FusedIterator for IntoIter<K, V> {}

impl<K, V> Drop for IntoIter<K, V> {
    fn drop(&mut self) {
        // Move the remaining entries to the front and fix `len`; then the
        // `RawBuf` field drops them, the index and the allocation.
        let data = self.raw.data();
        let remaining = self.end - self.start;
        // SAFETY: entries `start..end` are initialized; after moving them to
        // `0..remaining`, setting `len` makes the buffer consistent again.
        unsafe {
            if self.start != 0 && remaining > 0 {
                ptr::copy(data.add(self.start), data, remaining);
            }
            self.raw.set_len(remaining);
        }
    }
}

// The iterator types wrap slice iterators today, but are opaque so the
// storage layout (e.g. separate key and value arrays) can change later.

macro_rules! impl_iterator {
    ($name:ident, $item:ty, |$e:pat_param| $map:expr) => {
        impl<'a, K, V> Iterator for $name<'a, K, V> {
            type Item = $item;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                self.inner.next().map(|$e| $map)
            }

            #[inline]
            fn size_hint(&self) -> (usize, Option<usize>) {
                self.inner.size_hint()
            }

            #[inline]
            fn nth(&mut self, n: usize) -> Option<Self::Item> {
                self.inner.nth(n).map(|$e| $map)
            }
        }

        impl<'a, K, V> DoubleEndedIterator for $name<'a, K, V> {
            #[inline]
            fn next_back(&mut self) -> Option<Self::Item> {
                self.inner.next_back().map(|$e| $map)
            }
        }

        impl<K, V> ExactSizeIterator for $name<'_, K, V> {}
        impl<K, V> FusedIterator for $name<'_, K, V> {}
    };
}

macro_rules! impl_clone_iter {
    ($name:ident) => {
        impl<K, V> Clone for $name<'_, K, V> {
            fn clone(&self) -> Self {
                $name {
                    inner: self.inner.clone(),
                }
            }
        }
    };
}

/// Iterator over `(&K, &V)`, created by [`SlimMap::iter`].
pub struct Iter<'a, K, V> {
    pub(crate) inner: slice::Iter<'a, (K, V)>,
}
impl_iterator!(Iter, (&'a K, &'a V), |(k, v)| (k, v));
impl_clone_iter!(Iter);

/// Iterator over `(&K, &mut V)`, created by [`SlimMap::iter_mut`].
pub struct IterMut<'a, K, V> {
    pub(crate) inner: slice::IterMut<'a, (K, V)>,
}
impl_iterator!(IterMut, (&'a K, &'a mut V), |(k, v)| (&*k, v));

/// Iterator over keys, created by [`SlimMap::keys`].
pub struct Keys<'a, K, V> {
    pub(crate) inner: slice::Iter<'a, (K, V)>,
}
impl_iterator!(Keys, &'a K, |(k, _)| k);
impl_clone_iter!(Keys);

/// Iterator over values, created by [`SlimMap::values`].
pub struct Values<'a, K, V> {
    pub(crate) inner: slice::Iter<'a, (K, V)>,
}
impl_iterator!(Values, &'a V, |(_, v)| v);
impl_clone_iter!(Values);

/// Iterator over mutable values, created by [`SlimMap::values_mut`].
pub struct ValuesMut<'a, K, V> {
    pub(crate) inner: slice::IterMut<'a, (K, V)>,
}
impl_iterator!(ValuesMut, &'a mut V, |(_, v)| v);

impl<'a, K, V, S> IntoIterator for &'a SlimMap<K, V, S> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V, S> IntoIterator for &'a mut SlimMap<K, V, S> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
