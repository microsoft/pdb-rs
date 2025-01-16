//! Utilities for sorting

/// Verifies that all items in the slice are in sorted order. Duplicates are permitted.
pub fn assert_is_sorted<T>(items: &[T])
where
    T: Ord + std::fmt::Debug,
{
    for (i, w) in items.windows(2).enumerate() {
        assert!(
            w[0] <= w[1],
            "items are not in right order.  items[{}] = {:?}, items[{}] = {:?}",
            i,
            w[0],
            i + 1,
            w[1]
        );
    }
}

/// Verifies that all items in the slice are in strictly-increasing sorted order.
pub fn assert_is_sorted_and_unique<T>(items: &[T])
where
    T: Ord + std::fmt::Debug,
{
    for (i, w) in items.windows(2).enumerate() {
        assert!(
            w[0] < w[1],
            "items are not in right order.  items[{}] = {:?}, items[{}] = {:?}",
            i,
            w[0],
            i + 1,
            w[1]
        );
    }
}

/// Sorts a sequence of variable-length records. Returns a sorted permutation vector.
///
/// This function does not move the records. It just produces a new vector that describes
/// their order.
///
/// The `starts` vector provides the index within `records` where each record begins.
/// There is also an extra value at the end of `starts` that is equal to `records.len()`.
/// Therefore, the number of records is `starts.len() - 1`.
pub fn sort_records<T: Ord>(records: &[T], starts: &[u32]) -> Vec<u32> {
    assert!(!starts.is_empty());
    assert!(*starts.last().unwrap() as usize == records.len());
    assert!(records.len() <= u32::MAX as usize);
    debug_assert!(
        starts.windows(2).all(|w| w[0] <= w[1]),
        "starts should be non-decreasing"
    );

    let num_records = starts.len() - 1;

    let get_record = |i: u32| -> &[T] {
        let start = starts[i as usize] as usize;
        let end = starts[i as usize + 1] as usize;
        &records[start..end]
    };

    let mut order: Vec<u32> = (0..num_records as u32).collect();
    order.sort_unstable_by(|&a, &b| {
        let record_a = get_record(a);
        let record_b = get_record(b);
        record_a.cmp(record_b)
    });
    order
}

/// Reads a sequence of variable-length records and writes them to a new destination,
/// using a given order vector (a permutation).
///
/// The `starts` vector provides the index within `src_records` where each record begins.
pub fn reorder_records<T: Copy>(
    src_records: &[T],
    dst_records: &mut [T],
    src_starts: &[u32],
    order: &[u32],
) {
    assert!(!src_starts.is_empty());
    assert_eq!(*src_starts.last().unwrap() as usize, src_records.len());
    assert_eq!(src_records.len(), dst_records.len());
    assert_eq!(src_starts.len(), order.len() + 1);
    debug_assert!(
        src_starts.windows(2).all(|w| w[0] <= w[1]),
        "starts should be non-decreasing"
    );

    let get_record = |i: u32| -> &[T] {
        let start = src_starts[i as usize] as usize;
        let end = src_starts[i as usize + 1] as usize;
        &src_records[start..end]
    };

    let mut dst_iter = dst_records;
    for &i in order.iter() {
        let src_record = get_record(i);
        let (dst_record, dst_next) = dst_iter.split_at_mut(src_record.len());
        dst_record.copy_from_slice(src_record);
        dst_iter = dst_next;
    }
    assert!(dst_iter.is_empty());
}

/// Reorders a sequence of variable-length records in-place, using a given order vector
/// (a permutation).
///
/// This allocates temporary storage for all of the records and copies them to it.
pub fn reorder_records_inplace<T: Copy>(records: &mut [T], starts: &[u32], order: &[u32]) {
    let old_records = records.to_vec();
    reorder_records(&old_records, records, starts, order);
}

/// Checks that `p` is a permutation vector.
pub fn assert_is_permutation_u32(p: &[u32]) {
    let mut found = vec![false; p.len()];

    for &i in p.iter() {
        assert!(!found[i as usize]);
        found[i as usize] = true;
    }

    assert!(found.iter().all(|&f| f));
}

/// Debug-only variant of `assert_is_permutation_u32`.
pub fn debug_assert_is_permutation_u32(p: &[u32]) {
    if cfg!(debug_assertions) {
        assert_is_permutation_u32(p);
    }
}

/// Inverts a permutation vector, stored in `u32`.
pub fn invert_permutation_u32(p: &[u32]) -> Vec<u32> {
    const INVALID_INDEX: u32 = u32::MAX;
    assert!(p.len() < INVALID_INDEX as usize);

    let mut inv = vec![INVALID_INDEX; p.len()];
    for (i, &j) in p.iter().enumerate() {
        assert_eq!(inv[j as usize], INVALID_INDEX, "i = {i}, j = {j}");
        inv[j as usize] = i as u32;
    }

    for &j in p.iter() {
        assert_ne!(j, INVALID_INDEX);
    }

    inv
}

/// Creates a permutation vector, stored in `u32`.
pub fn identity_permutation_u32(n: usize) -> Vec<u32> {
    (0..n as u32).collect()
}

/// Reorders a set of copyable items, using a permutation `p`. The permutation gives the order to
/// read input items.
pub fn reorder_copy_u32<T: Copy>(p: &[u32], items: &[T]) -> Vec<T> {
    assert_eq!(p.len(), items.len());
    let mut output = Vec::with_capacity(p.len());
    for &i in p.iter() {
        output.push(items[i as usize]);
    }
    output
}

/// Helper method that reads a HashMap and copies references to a vector, then sorts the vector
/// by the keys.
pub fn sort_map<K, V>(map: &std::collections::HashMap<K, V>) -> Vec<(&K, &V)>
where
    K: Ord,
{
    let mut ordered: Vec<(&K, &V)> = map.iter().collect();
    ordered.sort_unstable_by_key(|i| i.0);
    ordered
}
