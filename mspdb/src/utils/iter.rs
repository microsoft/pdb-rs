//! Iterator utilities

use std::ops::Range;

/// Allows iterators to report the remaining, unparsed bytes within an iterator.
///
/// This is for iterators that parse items from `&[u8]` buffers or similar.
pub trait HasRestLen {
    /// Returns the number of bytes (or elements, abstractly) that have not yet been parsed by this
    /// iterator.
    fn rest_len(&self) -> usize;
}

/// An iterator adapter which reports the byte ranges of the items that are iterated by the
/// underlying iterator. The underlying iterator must implement `HasRestLen`.
pub struct IterWithRange<I> {
    original_len: usize,
    inner: I,
}

impl<I> IterWithRange<I> {
    /// The number of items (usually bytes) that were present in the inner iterator when this
    /// `IterWithRange` was created. This value allows us to convert the "bytes remaining" value
    /// (`rest_len()`, which is what the inner iterator operates directly on) to an offset from the
    /// beginning of a buffer.
    pub fn original_len(&self) -> usize {
        self.original_len
    }

    /// Gets access to the inner iterator.
    pub fn inner(&self) -> &I {
        &self.inner
    }

    /// Gets mutable access to the inner iterator.
    ///
    /// Be warned!  If you modify this iterator, make sure you don't break its relationship with
    /// the `original_len` value.  Iterating items from it is fine, because that should never
    /// break the relationship with `original_len`.
    ///
    /// What would break it would be replacing the inner iterator with one that has a length that
    /// is greater than `original_len`.
    pub fn inner_mut(&mut self) -> &mut I {
        &mut self.inner
    }

    /// The current position in the iteration range.
    #[inline(always)]
    pub fn pos(&self) -> usize
    where
        I: HasRestLen,
    {
        self.original_len - self.inner.rest_len()
    }
}

impl<I: Iterator> Iterator for IterWithRange<I>
where
    I: HasRestLen,
{
    type Item = (Range<usize>, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let pos_before = self.pos();
        let item = self.inner.next()?;
        let pos_after = self.pos();
        Some((pos_before..pos_after, item))
    }
}

/// An extension trait for iterators that converts an `Iterator` into an `IterWithRange`.
/// Use `foo.with_ranges()` to convert (augment) the iterator.
pub trait IteratorWithRangesExt: Sized {
    /// Augments this iterator with information about the byte range of each underlying item.
    fn with_ranges(self) -> IterWithRange<Self>;
}

impl<I> IteratorWithRangesExt for I
where
    I: Iterator + HasRestLen,
{
    fn with_ranges(self) -> IterWithRange<Self> {
        IterWithRange {
            original_len: self.rest_len(),
            inner: self,
        }
    }
}

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
