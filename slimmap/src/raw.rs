use alloc::alloc::{alloc, dealloc, handle_alloc_error, realloc};
use core::marker::PhantomData;
use core::ptr::{self, NonNull};
use core::slice;

use crate::layout::{
    Header, IndexTable, empty_header, entries_ptr, is_indexed_cap, layout_for, table_offset,
};

// Safety invariant: the tail slot holds a valid `IndexTable` if and only if
// `cap >= INDEX_MIN_CAP`. The table's *contents* are only a logical
// invariant: every access from the table into the entries is bounds-checked,
// so a stale table (e.g. after a panicking `Hash` impl) can cause wrong
// answers or panics, but never undefined behavior.

pub(crate) struct RawBuf<K, V> {
    ptr: NonNull<Header>,
    _marker: PhantomData<(K, V)>,
}

// SAFETY: `RawBuf` owns its entries like `Vec<(K, V)>` does, and has no
// shared mutable state (the empty header is never written). The index table
// holds plain `u32`s.
unsafe impl<K: Send, V: Send> Send for RawBuf<K, V> {}
// SAFETY: as above; `&RawBuf` only hands out shared references.
unsafe impl<K: Sync, V: Sync> Sync for RawBuf<K, V> {}

impl<K, V> RawBuf<K, V> {
    pub(crate) const fn new() -> Self {
        RawBuf {
            ptr: empty_header(),
            _marker: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        // SAFETY: `ptr` always points to a valid header.
        unsafe { (*self.ptr.as_ptr()).len as usize }
    }

    #[inline]
    pub(crate) fn cap(&self) -> usize {
        // SAFETY: `ptr` always points to a valid header.
        unsafe { (*self.ptr.as_ptr()).cap as usize }
    }

    #[inline]
    pub(crate) fn is_indexed(&self) -> bool {
        is_indexed_cap(self.cap())
    }

    #[inline]
    pub(crate) fn data(&self) -> *mut (K, V) {
        // SAFETY: `ptr` is always the empty header or our own allocation.
        unsafe { entries_ptr::<K, V>(self.ptr) }
    }

    #[inline]
    pub(crate) fn entries(&self) -> &[(K, V)] {
        // SAFETY: the first `len` entries are initialized.
        unsafe { slice::from_raw_parts(self.data(), self.len()) }
    }

    #[inline]
    pub(crate) fn entries_mut(&mut self) -> &mut [(K, V)] {
        // SAFETY: the first `len` entries are initialized, and `&mut self`
        // guarantees exclusive access.
        unsafe { slice::from_raw_parts_mut(self.data(), self.len()) }
    }

    /// Pointer to the index table. Only valid when `is_indexed()`.
    #[inline]
    pub(crate) fn table_ptr(&self) -> *mut IndexTable {
        debug_assert!(self.is_indexed());
        // SAFETY: indexed allocations were laid out by `layout_for`, which
        // places the table at `table_offset`.
        unsafe {
            self.ptr
                .as_ptr()
                .cast::<u8>()
                .add(table_offset::<K, V>(self.cap()))
                .cast()
        }
    }

    #[inline]
    pub(crate) fn table(&self) -> Option<&IndexTable> {
        if self.is_indexed() {
            // SAFETY: the table exists when indexed (struct invariant).
            Some(unsafe { &*self.table_ptr() })
        } else {
            None
        }
    }

    /// The entries and the index table, borrowed at the same time. They
    /// occupy disjoint parts of the allocation.
    #[inline]
    pub(crate) fn parts_mut(&mut self) -> (&mut [(K, V)], Option<&mut IndexTable>) {
        let table = if self.is_indexed() {
            // SAFETY: the table exists when indexed, and doesn't overlap the
            // entries.
            Some(unsafe { &mut *self.table_ptr() })
        } else {
            None
        };
        // SAFETY: as in `entries_mut`; disjoint from the table.
        let entries = unsafe { slice::from_raw_parts_mut(self.data(), self.len()) };
        (entries, table)
    }

    /// # Safety
    /// `len <= cap`, and entries `0..len` must be initialized.
    #[inline]
    pub(crate) unsafe fn set_len(&mut self, len: usize) {
        debug_assert!(len <= self.cap());
        if self.cap() == 0 {
            // Never write to the shared empty header.
            return;
        }
        // SAFETY: we own a real allocation; `len <= cap <= u32::MAX`.
        unsafe { (*self.ptr.as_ptr()).len = len as u32 };
    }

    /// Appends an entry. The caller must have reserved room; the index is
    /// not updated.
    #[inline]
    pub(crate) fn push(&mut self, key: K, value: V) {
        let len = self.len();
        assert!(len < self.cap());
        // SAFETY: `len < cap`, so the slot is in bounds and uninitialized.
        unsafe {
            ptr::write(self.data().add(len), (key, value));
            self.set_len(len + 1);
        }
    }

    /// Removes entry `i`, shifting later entries down. Index not updated.
    pub(crate) fn shift_remove(&mut self, i: usize) -> (K, V) {
        let len = self.len();
        assert!(i < len);
        let data = self.data();
        // SAFETY: `i < len`. We move entry `i` out, shift the tail down by
        // one, then shrink `len`. No user code runs in between.
        unsafe {
            let entry = ptr::read(data.add(i));
            ptr::copy(data.add(i + 1), data.add(i), len - i - 1);
            self.set_len(len - 1);
            entry
        }
    }

    /// Removes entry `i`, moving the last entry into its place. Index not
    /// updated.
    pub(crate) fn swap_remove(&mut self, i: usize) -> (K, V) {
        let len = self.len();
        assert!(i < len);
        let data = self.data();
        let last = len - 1;
        // SAFETY: `i < len`. We move entry `i` out, move the last entry into
        // its slot (if different), then shrink `len`.
        unsafe {
            let entry = ptr::read(data.add(i));
            if i != last {
                ptr::copy_nonoverlapping(data.add(last), data.add(i), 1);
            }
            self.set_len(last);
            entry
        }
    }

    /// Removes the last entry. Index not updated.
    pub(crate) fn pop(&mut self) -> Option<(K, V)> {
        let len = self.len();
        if len == 0 {
            return None;
        }
        // SAFETY: entry `len - 1` is initialized; shrinking `len` first
        // transfers ownership to us.
        unsafe {
            self.set_len(len - 1);
            Some(ptr::read(self.data().add(len - 1)))
        }
    }

    /// Drops all entries and empties the index, keeping the allocation.
    pub(crate) fn clear(&mut self) {
        if let (_, Some(table)) = self.parts_mut() {
            table.clear();
        }
        let len = self.len();
        // SAFETY: set len to 0 before dropping so a panicking destructor
        // can't cause a double drop (the rest are leaked instead).
        unsafe {
            self.set_len(0);
            ptr::drop_in_place(ptr::slice_from_raw_parts_mut(self.data(), len));
        }
    }

    /// Keeps entries for which `keep` returns true. Index not updated.
    pub(crate) fn retain<F>(&mut self, mut keep: F)
    where
        F: FnMut(&K, &mut V) -> bool,
    {
        // On unwind (from `keep` or from a destructor), the guard moves the
        // unprocessed tail down and fixes `len`, so nothing is dropped twice
        // or leaked.
        struct Guard<'a, K, V> {
            raw: &'a mut RawBuf<K, V>,
            read: usize,
            write: usize,
            len: usize,
        }
        impl<K, V> Drop for Guard<'_, K, V> {
            fn drop(&mut self) {
                let data = self.raw.data();
                let tail = self.len - self.read;
                // SAFETY: entries `read..len` are initialized and
                // unprocessed; `write <= read`, so `ptr::copy` handles any
                // overlap.
                unsafe {
                    if tail > 0 && self.read != self.write {
                        ptr::copy(data.add(self.read), data.add(self.write), tail);
                    }
                    self.raw.set_len(self.write + tail);
                }
            }
        }

        let len = self.len();
        let data = self.data();
        // SAFETY: setting len to 0 first means a panic can never expose
        // half-processed entries; the guard restores the correct len.
        unsafe { self.set_len(0) };
        let mut g = Guard {
            raw: self,
            read: 0,
            write: 0,
            len,
        };
        while g.read < len {
            // SAFETY: `read < len`, so the entry is initialized and not yet
            // moved or dropped.
            unsafe {
                let cur = data.add(g.read);
                let (k, v) = &mut *cur;
                if keep(k, v) {
                    if g.read != g.write {
                        ptr::copy_nonoverlapping(cur, data.add(g.write), 1);
                    }
                    g.write += 1;
                    g.read += 1;
                } else {
                    // Advance first: if the destructor panics, the entry
                    // counts as gone.
                    g.read += 1;
                    ptr::drop_in_place(cur);
                }
            }
        }
    }

    /// Reallocates to exactly `new_cap` entries (non-zero, at least `len`),
    /// creating, keeping or dropping the index table as the capacity
    /// requires. `hash` is used to build a new index from existing keys.
    pub(crate) fn realloc_to(&mut self, new_cap: usize, hash: impl Fn(&K) -> u64) {
        let len = self.len();
        debug_assert!(new_cap >= len && new_cap > 0);
        assert!(new_cap <= u32::MAX as usize, "capacity overflow");
        let old_cap = self.cap();
        let old_indexed = is_indexed_cap(old_cap);
        let new_indexed = is_indexed_cap(new_cap);
        let new_layout = layout_for::<K, V>(new_cap);

        // Build a fresh index *before* touching the allocation: hashing runs
        // user code that may panic, and the map must be intact if it does.
        let fresh = if new_indexed && !old_indexed {
            let entries = self.entries();
            let mut table = IndexTable::with_capacity(new_cap);
            for (i, (k, _)) in entries.iter().enumerate() {
                table.insert_unique(hash(k), i as u32, |&j| hash(&entries[j as usize].0));
            }
            Some(table)
        } else {
            None
        };

        // Move the existing index out of the tail slot, which is about to
        // move or disappear.
        let moved = if old_indexed {
            // SAFETY: the table exists when indexed; we restore or replace
            // it below, so it is never duplicated.
            Some(unsafe { ptr::read(self.table_ptr()) })
        } else {
            None
        };

        // SAFETY: the layout is non-zero-sized (it contains the header). For
        // realloc, the old pointer and layout match the existing allocation,
        // and both layouts share one alignment (see `layout_for`).
        let raw = unsafe {
            if old_cap == 0 {
                alloc(new_layout)
            } else {
                realloc(
                    self.ptr.as_ptr().cast(),
                    layout_for::<K, V>(old_cap),
                    new_layout.size(),
                )
            }
        };
        let Some(new_ptr) = NonNull::new(raw.cast::<Header>()) else {
            // The old allocation is untouched. Put its index back so the map
            // stays valid if the error handler unwinds.
            if let Some(table) = moved {
                // SAFETY: the slot we read from is still ours and still
                // holds the bytes we moved out.
                unsafe { ptr::write(self.table_ptr(), table) };
            }
            drop(fresh);
            handle_alloc_error(new_layout)
        };

        // SAFETY: `new_ptr` is a live allocation laid out for `new_cap`.
        unsafe {
            if old_cap == 0 {
                new_ptr.as_ptr().write(Header {
                    len: 0,
                    cap: new_cap as u32,
                });
            } else {
                (*new_ptr.as_ptr()).cap = new_cap as u32;
            }
        }
        self.ptr = new_ptr;

        if new_indexed {
            let table = moved.or(fresh).expect("index table for indexed capacity");
            // SAFETY: the tail slot exists for this capacity and holds no
            // live table (its bytes, if any, were moved out above).
            unsafe { ptr::write(self.table_ptr(), table) };
        }
        // If the map stopped being indexed, `moved` is dropped here.
    }

    /// Frees the allocation (and index) of an empty map, returning it to the
    /// shared empty header.
    pub(crate) fn free(&mut self) {
        debug_assert_eq!(self.len(), 0);
        if self.cap() == 0 {
            return;
        }
        if self.is_indexed() {
            // SAFETY: the table exists when indexed and is dropped once.
            unsafe { ptr::drop_in_place(self.table_ptr()) };
        }
        // SAFETY: cap != 0, no live entries, table dropped.
        unsafe { self.dealloc_buffer() };
        self.ptr = empty_header();
    }

    /// Frees the allocation without dropping entries or the index.
    ///
    /// # Safety
    /// `cap() != 0`; entries and the index must already be dropped or moved
    /// out. `self.ptr` is dangling afterwards.
    pub(crate) unsafe fn dealloc_buffer(&mut self) {
        let cap = self.cap();
        debug_assert!(cap != 0);
        // SAFETY: caller guarantees this is our allocation with this cap.
        unsafe { dealloc(self.ptr.as_ptr().cast(), layout_for::<K, V>(cap)) };
    }
}

impl<K, V> Drop for RawBuf<K, V> {
    fn drop(&mut self) {
        if self.cap() == 0 {
            return;
        }
        if self.is_indexed() {
            // SAFETY: the table exists when indexed and is dropped once.
            // Dropping it runs no user code.
            unsafe { ptr::drop_in_place(self.table_ptr()) };
        }
        // Free the buffer even if an entry's destructor panics.
        struct DeallocOnDrop<'a, K, V>(&'a mut RawBuf<K, V>);
        impl<K, V> Drop for DeallocOnDrop<'_, K, V> {
            fn drop(&mut self) {
                // SAFETY: cap != 0; the table is dropped and entries are
                // dropped (or leaked by a panic) before this runs.
                unsafe { self.0.dealloc_buffer() };
            }
        }
        let len = self.len();
        let data = self.data();
        let _guard = DeallocOnDrop(self);
        // SAFETY: entries `0..len` are initialized and dropped exactly once.
        unsafe { ptr::drop_in_place(ptr::slice_from_raw_parts_mut(data, len)) };
    }
}
