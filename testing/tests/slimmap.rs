//! Integration tests for the public `SlimMap` API.

use std::collections::HashMap;

use slimmap::SlimMap;

#[test]
fn new_map_is_empty() {
    let map: SlimMap<u32, u32> = SlimMap::new();
    assert!(map.is_empty());
    assert_eq!(map.len(), 0);
    assert_eq!(map.get(&1), None);
}

#[test]
fn insert_new_key_returns_none() {
    let mut map = SlimMap::new();
    assert_eq!(map.insert("a", 1), None);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&"a"), Some(&1));
}

#[test]
fn insert_existing_key_returns_old_value_and_keeps_len() {
    let mut map = SlimMap::new();
    map.insert("a", 1);
    assert_eq!(map.insert("a", 2), Some(1));
    assert_eq!(map.len(), 1);
    assert_eq!(map.get(&"a"), Some(&2));
}

#[test]
fn get_mut_modifies_value_in_place() {
    let mut map = SlimMap::new();
    map.insert("a", 1);
    *map.get_mut(&"a").unwrap() += 41;
    assert_eq!(map.get(&"a"), Some(&42));
    assert!(map.get_mut(&"missing").is_none());
}

#[test]
fn contains_key_reflects_membership() {
    let mut map = SlimMap::new();
    map.insert(7, ());
    assert!(map.contains_key(&7));
    assert!(!map.contains_key(&8));
}

#[test]
fn remove_returns_value_and_absent_key_returns_none() {
    let mut map = SlimMap::new();
    map.insert("a", 1);
    assert_eq!(map.remove(&"a"), Some(1));
    assert_eq!(map.remove(&"a"), None);
    assert!(map.is_empty());
}

#[test]
fn remove_keeps_other_entries_reachable() {
    let mut map = SlimMap::new();
    for i in 0..10 {
        map.insert(i, i * 10);
    }
    assert_eq!(map.remove(&0), Some(0));
    assert_eq!(map.len(), 9);
    for i in 1..10 {
        assert_eq!(map.get(&i), Some(&(i * 10)), "lost key {i} after remove");
    }
}

#[test]
fn clear_removes_everything() {
    let mut map = SlimMap::new();
    map.insert(1, "one");
    map.insert(2, "two");
    map.clear();
    assert!(map.is_empty());
    assert!(!map.contains_key(&1));
}

#[test]
fn iter_yields_each_pair_once() {
    let mut map = SlimMap::new();
    for i in 0..5 {
        map.insert(i, i + 100);
    }
    let mut pairs: Vec<(i32, i32)> = map.iter().map(|(k, v)| (*k, *v)).collect();
    pairs.sort();
    assert_eq!(
        pairs,
        vec![(0, 100), (1, 101), (2, 102), (3, 103), (4, 104)]
    );
}

#[test]
fn owned_non_copy_keys_and_values_work() {
    let mut map: SlimMap<String, Vec<u8>> = SlimMap::new();
    map.insert("alpha".to_string(), vec![1, 2]);
    map.insert("beta".to_string(), vec![3]);
    assert_eq!(map.get(&"alpha".to_string()), Some(&vec![1, 2]));
    assert_eq!(map.remove(&"beta".to_string()), Some(vec![3]));
}

#[test]
fn clone_and_eq_compare_contents() {
    let mut a = SlimMap::new();
    a.insert(1, 'x');
    let b = a.clone();
    assert_eq!(a, b);
    a.insert(2, 'y');
    assert_ne!(a, b);
}

#[test]
fn matches_std_hashmap_over_random_operations() {
    let mut slim: SlimMap<u32, u32> = SlimMap::new();
    let mut reference: HashMap<u32, u32> = HashMap::new();

    let mut state: u32 = 12345;
    let mut next = || {
        state = state.wrapping_mul(1_103_515_245).wrapping_add(12345);
        state >> 8
    };

    for step in 0..2_000u32 {
        let key = next() % 64;
        let value = step;
        match next() % 3 {
            0 => assert_eq!(slim.insert(key, value), reference.insert(key, value)),
            1 => assert_eq!(slim.remove(&key), reference.remove(&key)),
            _ => assert_eq!(slim.get(&key), reference.get(&key)),
        }
        assert_eq!(slim.len(), reference.len(), "len diverged at step {step}");
    }

    for (k, v) in &reference {
        assert_eq!(slim.get(k), Some(v));
    }
}
