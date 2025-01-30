//! Misc utilities

pub mod align;
pub mod io;
pub mod iter;
pub mod path;
pub mod swizzle;
pub mod vec;

use std::ops::Range;
use zerocopy::{FromBytes, Immutable, IntoBytes};

/// Copies a value that implements `FromBytes`, by simply copying its byte representation.
pub fn copy_from_bytes<T>(t: &T) -> T
where
    T: IntoBytes + FromBytes + Immutable,
{
    FromBytes::read_from_bytes(t.as_bytes()).unwrap()
}

/// Helps decode records that are indexed using "starts" arrays.
pub struct StartsOf<'a, T> {
    /// The "starts" array
    pub starts: &'a [u32],
    /// The items that are being indexed.
    pub items: &'a [T],
}

impl<'a, T> StartsOf<'a, T> {
    /// Initializes a new starts-based array accessor.
    pub fn new(starts: &'a [u32], items: &'a [T]) -> Self {
        debug_assert!(!starts.is_empty());
        debug_assert_eq!(starts[0], 0);
        debug_assert_eq!(*starts.last().unwrap() as usize, items.len());
        debug_assert!(starts.windows(2).all(|w| w[0] <= w[1]));

        Self { starts, items }
    }
}

impl<'a, T> std::ops::Index<usize> for StartsOf<'a, T> {
    type Output = [T];

    fn index(&self, i: usize) -> &[T] {
        let start = self.starts[i] as usize;
        let end = self.starts[i + 1] as usize;
        &self.items[start..end]
    }
}

/// True if `n` is a multiple of 4.
pub fn is_aligned_4(n: usize) -> bool {
    (n & 3) == 0
}

/// Align n up to the next multiple of 4, if it is not already a multiple of 4.
pub fn align_4(n: usize) -> usize {
    (n + 3) & !3
}

/// Iterates ranges of items within a slice that share a common property.
pub fn iter_similar_ranges<'a, T, F>(items: &'a [T], is_eq: F) -> IterSimilarRanges<'a, T, F> {
    IterSimilarRanges {
        items,
        is_eq,
        start: 0,
    }
}

/// Iterates ranges of items within a slice that share a common property.
pub struct IterSimilarRanges<'a, T, F> {
    items: &'a [T],
    is_eq: F,
    start: usize,
}

impl<'a, T, F> Iterator for IterSimilarRanges<'a, T, F>
where
    F: FnMut(&T, &T) -> bool,
{
    type Item = Range<usize>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.is_empty() {
            return None;
        }

        let first = &self.items[0];

        let mut i = 1;
        while i < self.items.len() && (self.is_eq)(first, &self.items[i]) {
            i += 1;
        }

        let start = self.start;
        self.start += i;
        self.items = &self.items[i..];

        Some(start..start + i)
    }
}

/// Iterates ranges of items within a slice that share a common property.
pub fn iter_similar_slices<'a, T, F>(items: &'a [T], is_eq: F) -> IterSimilarSlices<'a, T, F> {
    IterSimilarSlices { items, is_eq }
}

/// Iterates slices of items within a slice that share a common property.
pub struct IterSimilarSlices<'a, T, F> {
    items: &'a [T],
    is_eq: F,
}

impl<'a, T, F> Iterator for IterSimilarSlices<'a, T, F>
where
    F: FnMut(&T, &T) -> bool,
{
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.is_empty() {
            return None;
        }

        let first = &self.items[0];

        let mut i = 1;
        while i < self.items.len() && (self.is_eq)(first, &self.items[i]) {
            i += 1;
        }

        let (lo, hi) = self.items.split_at(i);
        self.items = hi;
        Some(lo)
    }
}

/// Iterates ranges of items within a slice that share a common property.
pub fn iter_similar_slices_mut<'a, T, F>(
    items: &'a mut [T],
    is_eq: F,
) -> IterSimilarSlicesMut<'a, T, F> {
    IterSimilarSlicesMut { items, is_eq }
}

/// Iterates slices of items within a slice that share a common property.
pub struct IterSimilarSlicesMut<'a, T, F> {
    items: &'a mut [T],
    is_eq: F,
}

impl<'a, T, F> Iterator for IterSimilarSlicesMut<'a, T, F>
where
    F: FnMut(&T, &T) -> bool,
{
    type Item = &'a mut [T];

    fn next(&mut self) -> Option<Self::Item> {
        if self.items.is_empty() {
            return None;
        }

        let items = std::mem::take(&mut self.items);
        let first = &items[0];

        let mut i = 1;
        while i < items.len() && (self.is_eq)(first, &items[i]) {
            i += 1;
        }

        let (lo, hi) = items.split_at_mut(i);
        self.items = hi;
        Some(lo)
    }
}
