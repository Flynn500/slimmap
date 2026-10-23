//! An insertion-ordered map optimized for memory use.
//!
//! [`SlimMap`] is a single pointer wide (8 bytes on 64-bit targets) with the
//! default hasher, and `Option<SlimMap<K, V>>` is the same size. An empty map
//! does not allocate. The length and capacity live in a small header at the
//! start of the heap allocation, followed directly by the entries.
//!
//! Small maps are searched linearly, which is fast and needs no extra memory.
//! Once a map's capacity reaches a threshold, it also maintains a hash index
//! so lookups stay O(1) as it grows.
//!
//! Iteration order is insertion order. [`SlimMap::remove`] preserves the
//! order of the remaining entries; [`SlimMap::swap_remove`] is a faster
//! alternative that does not.
//!
//! Equality, ordering and hashing all take entry order into account: two maps
//! with the same entries in a different order are *not* equal.
//!
//! # Lookup keys
//!
//! Lookups accept any `Q` implementing [`Equivalent<K>`] and [`Hash`], so a
//! `SlimMap<String, _>` can be queried with a `&str`. As with `HashMap` and
//! `Borrow`, equivalent keys must produce identical hashes.

#![no_std]
#![warn(missing_docs)]

extern crate alloc;

mod hasher;
mod iter;
mod layout;
mod map;
mod raw;
mod traits;

pub use equivalent::Equivalent;
pub use hasher::{DefaultHashBuilder, DefaultHasher};
pub use iter::{IntoIter, Iter, IterMut, Keys, Values, ValuesMut};
pub use map::SlimMap;

// ---------------------------------------------------------------------------
// Tuning
// ---------------------------------------------------------------------------

/// Maps with at least this capacity maintain a hash index; smaller maps are
/// searched linearly. Benchmarks put the linear-scan crossover at roughly
/// 40–100 entries depending on key type and cache state, so this errs low.
const INDEX_MIN_CAP: usize = 64;

/// Smallest non-zero capacity when growing, matching `Vec` for typical
/// element sizes. Exact-size constructors (`with_capacity`, `collect`,
/// `shrink_to_fit`) can go below this.
const MIN_NON_ZERO_CAP: usize = 4;
