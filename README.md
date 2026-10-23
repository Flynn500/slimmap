# slimmap

**An insertion-ordered map for when you have lots of maps.**

`SlimMap` is a drop-in-style ordered map that is a single pointer wide and
stores its entries in one compact allocation. It is built for programs that
hold many small-to-medium maps at once: JSON and other document trees, element
attributes, span fields, label sets, etc.

```rust
use slimmap::SlimMap;

let mut user = SlimMap::new();
user.insert("name".to_string(), "Ada");
user.insert("role".to_string(), "engineer");
user.insert("team".to_string(), "compilers");

// Look up with a borrowed key.
assert_eq!(user.get("role"), Some(&"engineer"));

// Iteration follows insertion order.
let keys: Vec<_> = user.keys().map(String::as_str).collect();
assert_eq!(keys, ["name", "role", "team"]);

// Removing keeps the remaining entries in order.
user.remove("role");
assert_eq!(user.first(), Some((&"name".to_string(), &"Ada")));
```

## Why

Ordered maps such as `IndexMap` are excellent general-purpose tables, but each map carries a fixed cost: a multi-word handle, a hasher, a separate hash table and spare capacity in two places. With one big map that cost is noise. With ten thousand maps of four entries each, it dominates.

`SlimMap` attacks that overhead directly:

- **One pointer wide.** The handle is 8 bytes on 64-bit targets, and
  `Option<SlimMap<K, V>>` is too. When maps live inside other values — such as a
  JSON `Value` enum, where the largest variant sets the size of every value —
  this shrinks everything around them, not just the maps.
- **Empty maps don't allocate.**
- **One allocation for small maps.** The length and capacity live in a small
  header at the start of the buffer, followed directly by the entries. No hash
  table is built until a map is large enough to need one.
- **Exact sizing.** `collect`, `from`, `clone` and `with_capacity` allocate
  exactly what they need; growth is 1.5× rather than 2×.
- **Still fast when it grows.** Past a size threshold, the map maintains a hash
  index (backed by [hashbrown]), so lookups stay O(1) on large maps.

## Benchmarks

10,000 maps with `u64` keys and values. See [`benchmarks/`](benchmarks/) for the harness, methodology and full results including single large maps and `String` keys.

| Entries per map | | SlimMap | HashMap | BTreeMap | IndexMap |
|---|---|---:|---:|---:|---:|
| 4  | memory (total) | **0.80 MB** | 2.00 MB | 2.16 MB | 3.28 MB |
|    | get (ns)       | **3.9**     | 12.3    | 6.8     | 17.7    |
| 16 | memory (total) | **3.20 MB** | 6.08 MB | 6.96 MB | 10.48 MB |
|    | get (ns)       | **6.6**     | 18.4    | 20.2    | 20.1    |


Memory figures count heap bytes plus the inline map handles. Hash maps here use their default hashers; `SlimMap` uses foldhash by default, which accounts for part of the lookup difference.

## When to use slimmap

**Good fit:**

- Many maps alive at once, mostly with up to a few hundred entries
- Insertion order matters, or is nice to have
- Data that is built once and read often, with few removals
- Memory-constrained environments, including `no_std` + `alloc`

Examples: JSON/YAML/TOML document trees, HTML/XML element attributes, tracing
span fields, metric labels, HTTP headers, interpreter object properties,
per-scope symbol tables.

**Reach for something else when:**

- You have **one large, hot map**: `HashMap` or `IndexMap` will serve you
  well, and `SlimMap`'s memory advantage mostly disappears at scale.
- Your workload is **removal-heavy**: order-preserving `remove` is O(n).
  (`swap_remove` is O(1) but moves the last entry.)
- You need **order-insensitive equality** (see below).
- You need a **concurrent** map.

## Semantics

`SlimMap` mostly mirrors `HashMap`'s API, but a few behaviors differ:

- **Equality, ordering and hashing are order-sensitive.** Two maps with the
  same entries in a different order are not equal. This also lets `SlimMap`
  implement `Ord` and `Hash`. `IndexMap` and `HashMap` compare as sets instead.
- **`remove` preserves order.** Use `swap_remove` for the O(1) version.
- **Inserting an existing key** replaces the value in place, keeping the
  entry's position and original key.
- **Lookups use [`Equivalent`]**, re-exported from the [equivalent] crate, so a
  `SlimMap<String, _>` can be queried with a `&str`. As with `Borrow`,
  equivalent keys must hash identically.
- **Mutating methods require `K: Hash + Eq`**, including `reserve` and
  `shrink_to`: hashes aren't stored, so growing past the indexing threshold
  rehashes keys.
- **Capacity is limited to `u32::MAX` entries.**

## Features

| Feature | Default | Description |
|---|---|---|
| `std` | yes | Seeds the default hasher from OS randomness. Without it, the crate is `no_std` + `alloc` and seeds from address entropy. |

The default hasher, `DefaultHashBuilder`, is zero-sized and uses [foldhash]
with a random per-process seed. Supply your own with `SlimMap::with_hasher`.