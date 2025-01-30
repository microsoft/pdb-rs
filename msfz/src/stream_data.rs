use core::ops::Range;
use std::sync::Arc;

use zerocopy::FromZeros;

#[cfg(doc)]
use crate::Msfz;

/// Contains the contents of an entire stream.
///
/// This is used as the return type for [`Msfz::read_stream`] function. This type either contains
/// an owned buffer (`Vec`) or a counted reference to a slice of an `Arc<[u8]>`.
///
/// See the `[Msfz::read_stream]` function for more details.
pub enum StreamData {
    /// Owned contents of stream data
    Box(Box<[u8]>),
    /// Shared contents of stream data.  The `Range` gives the range of bytes within the `Arc`.
    ArcSlice(Arc<[u8]>, Range<usize>),
}

impl StreamData {
    /// Gets a slice over the contained stream data.
    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        match self {
            Self::Box(v) => v,
            Self::ArcSlice(arc, range) => &arc[range.clone()],
        }
    }

    /// Returns `true` if the stream contains no data.
    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    /// Converts this `StreamData` into an owned `Vec<u8>`.
    pub fn into_vec(self) -> Vec<u8> {
        self.into_boxed().into()
    }

    /// Converts this `StreamData` into an owned `Box<[u8]>`.
    pub fn into_boxed(self) -> Box<[u8]> {
        match self {
            Self::Box(b) => b,
            Self::ArcSlice(arc, range) => {
                let mut b: Box<[u8]> = FromZeros::new_box_zeroed_with_elems(range.len()).unwrap();
                b.copy_from_slice(&arc[range]);
                b
            }
        }
    }
}

impl From<StreamData> for Box<[u8]> {
    fn from(s: StreamData) -> Self {
        s.into_boxed()
    }
}

impl core::ops::Deref for StreamData {
    type Target = [u8];

    #[inline(always)]
    fn deref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl AsRef<[u8]> for StreamData {
    #[inline(always)]
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl Default for StreamData {
    fn default() -> Self {
        Self::empty()
    }
}

impl StreamData {
    /// An empty value for `StreamData`
    pub fn empty() -> Self {
        Self::Box(Box::from(&[] as &[u8]))
    }
}
