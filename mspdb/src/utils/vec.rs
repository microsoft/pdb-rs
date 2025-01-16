//! Utilities for `Vec`

// use std::ops::Range;
use std::cmp::Ordering;

/// Replace a range of values in a vector with a new range. The old and new ranges can be
/// different sizes.
pub fn replace_range_copy<T: Copy>(v: &mut Vec<T>, start: usize, old_len: usize, values: &[T]) {
    assert!(start <= v.len());
    assert!(old_len <= v.len() - start);

    match values.len().cmp(&old_len) {
        Ordering::Equal => {
            v[start..start + values.len()].copy_from_slice(values);
        }

        Ordering::Less => {
            // The new values are shorter than the old values.
            // Copy the overlap, then drain the remainder.
            v[start..start + values.len()].copy_from_slice(values);
            v.drain(start + values.len()..start + old_len);
        }

        Ordering::Greater => {
            // Copy the overlapping values.
            // Then append the other values.
            // Then rotate them into position.
            let (lo, hi) = values.split_at(old_len);
            v.extend_from_slice(hi);
            v[start..start + old_len].copy_from_slice(lo);
            v[start + old_len..].rotate_right(lo.len());
        }
    }
}
