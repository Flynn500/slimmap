use alloc::alloc::Layout;
use core::hash::{BuildHasher, Hash};
use core::mem::{align_of, size_of};
use core::ptr::NonNull;

use hashbrown::HashTable;

use crate::INDEX_MIN_CAP;

// [ Header | (K, V) x cap | IndexTable (only if cap >= INDEX_MIN_CAP) ]
//
// Whether the index exists is a pure function of `cap`, so it needs no flag.
// The index table holds `u32` positions into the entries. It has its own
// allocation (hashbrown's), but its 32-byte handle lives in our tail slot.

pub(crate) type IndexTable = HashTable<u32>;

#[repr(C)]
pub(crate) struct Header {
    pub(crate) len: u32,
    pub(crate) cap: u32,
}

/// Shared by every empty map, so that empty maps never allocate.
/// It is never written to: all writes go through a check that `cap != 0`.
pub(crate) static EMPTY_HEADER: Header = Header { len: 0, cap: 0 };

pub(crate) const fn empty_header() -> NonNull<Header> {
    // SAFETY: a reference to a static is never null. The pointer is only
    // read through, never written (see `EMPTY_HEADER`).
    unsafe { NonNull::new_unchecked(&EMPTY_HEADER as *const Header as *mut Header) }
}

#[inline]
pub(crate) fn is_indexed_cap(cap: usize) -> bool {
    cap >= INDEX_MIN_CAP
}

/// Byte offset from the start of the allocation to the first entry.
#[inline]
pub(crate) fn entries_offset<K, V>() -> usize {
    // Infallible: a single element always fits in a layout.
    let (_, offset) = Layout::new::<Header>()
        .extend(Layout::new::<(K, V)>())
        .expect("layout of a single entry");
    offset
}

/// Byte offset of the index table for an allocation with `cap` entries.
/// Only meaningful when `is_indexed_cap(cap)`.
#[inline]
pub(crate) fn table_offset<K, V>(cap: usize) -> usize {
    // Cannot overflow: an allocation with this capacity already exists, so
    // `layout_for` computed this without overflowing.
    let end = entries_offset::<K, V>() + cap * size_of::<(K, V)>();
    let align = align_of::<IndexTable>();
    (end + align - 1) & !(align - 1)
}

/// Layout of an allocation holding `cap` entries. Panics on overflow.
pub(crate) fn layout_for<K, V>(cap: usize) -> Layout {
    let entries = Layout::array::<(K, V)>(cap).expect("capacity overflow");
    let (mut layout, _) = Layout::new::<Header>()
        .extend(entries)
        .expect("capacity overflow");
    if is_indexed_cap(cap) {
        let (with_table, offset) = layout
            .extend(Layout::new::<IndexTable>())
            .expect("capacity overflow");
        debug_assert_eq!(offset, table_offset::<K, V>(cap));
        layout = with_table;
    }
    // `realloc` cannot change alignment, so every capacity must share one
    // alignment, whether or not it includes the index table.
    layout
        .align_to(align_of::<IndexTable>())
        .expect("capacity overflow")
        .pad_to_align()
}

/// Pointer to the first entry of the allocation behind `header`.
///
/// # Safety
/// `header` must point either to `EMPTY_HEADER` or to an allocation made
/// with `layout_for::<K, V>(cap)` whose header holds that `cap`.
#[inline]
pub(crate) unsafe fn entries_ptr<K, V>(header: NonNull<Header>) -> *mut (K, V) {
    // SAFETY: caller guarantees `header` is valid to read.
    let cap = unsafe { (*header.as_ptr()).cap };
    if cap == 0 {
        // The empty header has no room for entries. A dangling pointer is
        // the correct base for a zero-length slice.
        NonNull::dangling().as_ptr()
    } else {
        // SAFETY: the offset is within (or one past the end of) the
        // allocation, which was laid out by `layout_for`.
        unsafe {
            header
                .as_ptr()
                .cast::<u8>()
                .add(entries_offset::<K, V>())
                .cast()
        }
    }
}

/// Closure for hashbrown to rehash an index slot, by hashing the entry's key.
#[inline]
pub(crate) fn rehasher<'a, K: Hash, V, S: BuildHasher>(
    hash_builder: &'a S,
    entries: &'a [(K, V)],
) -> impl Fn(&u32) -> u64 + 'a {
    move |&i| hash_builder.hash_one(&entries[i as usize].0)
}
