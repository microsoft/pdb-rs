//! Iterator utilities

use std::collections::BTreeMap;

/// Reads a slice of items and groups them using a function over the items.
pub fn group_by<'a, T, F, K>(s: &'a [T], f: F) -> BTreeMap<K, Vec<&'a T>>
where
    F: Fn(&T) -> K,
    K: Ord + Eq,
{
    let mut out: BTreeMap<K, Vec<&'a T>> = BTreeMap::new();

    for item in s.iter() {
        let key = f(item);
        if let Some(list) = out.get_mut(&key) {
            list.push(item);
        } else {
            out.insert(key, vec![item]);
        }
    }

    out
}

/// Reads a sequence of items and groups them using a function over the items.
pub fn group_by_iter_ref<'a, T, F, I, K>(iter: I, f: F) -> BTreeMap<K, Vec<&'a T>>
where
    I: Iterator<Item = &'a T>,
    F: Fn(&T) -> K,
    K: Ord + Eq,
{
    let mut out: BTreeMap<K, Vec<&'a T>> = BTreeMap::new();

    for item in iter {
        let key = f(item);
        if let Some(list) = out.get_mut(&key) {
            list.push(item);
        } else {
            out.insert(key, vec![item]);
        }
    }

    out
}

/// Reads a sequence of items and groups them using a function over the items.
pub fn group_by_iter<I, F, K>(iter: I, f: F) -> BTreeMap<K, Vec<I::Item>>
where
    I: Iterator,
    F: Fn(&I::Item) -> K,
    K: Ord + Eq,
{
    let mut out: BTreeMap<K, Vec<I::Item>> = BTreeMap::new();

    for item in iter {
        let key = f(&item);
        if let Some(list) = out.get_mut(&key) {
            list.push(item);
        } else {
            out.insert(key, vec![item]);
        }
    }

    out
}
