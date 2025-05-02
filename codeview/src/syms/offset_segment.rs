use super::*;

/// Stores an `offset` and `segment` pair, in that order. This structure is directly embedded in
/// on-disk structures.
#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned, Default, Clone, Copy, Eq)]
pub struct OffsetSegment {
    /// The offset in bytes of a symbol within a segment.
    pub offset: U32<LE>,

    /// The segment (section) index.
    pub segment: U16<LE>,
}

impl PartialEq for OffsetSegment {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.as_u64() == other.as_u64()
    }
}

impl PartialOrd for OffsetSegment {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OffsetSegment {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Ord::cmp(&self.as_u64(), &other.as_u64())
    }
}

impl OffsetSegment {
    /// The offset in bytes of a symbol within a segment.
    #[inline]
    pub fn offset(&self) -> u32 {
        self.offset.get()
    }

    /// The segment (section) index.
    #[inline]
    pub fn segment(&self) -> u16 {
        self.segment.get()
    }

    /// Combines the segment and offset into a tuple. The segment is the first element and the
    /// offset is the second element. This order gives a sorting order that sorts by segment first.
    #[inline]
    pub fn as_tuple(&self) -> (u16, u32) {
        (self.segment.get(), self.offset.get())
    }

    /// Combines the segment and offset into a single `u64` value, with the segment in the
    /// higher-order bits. This allows for efficient comparisons.
    #[inline]
    pub fn as_u64(&self) -> u64 {
        ((self.segment.get() as u64) << 32) | (self.offset.get() as u64)
    }
}

impl std::fmt::Display for OffsetSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:04x}:{:08x}]", self.segment.get(), self.offset.get())
    }
}

impl Debug for OffsetSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        <Self as std::fmt::Display>::fmt(self, f)
    }
}

impl std::hash::Hash for OffsetSegment {
    #[inline]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u16(self.segment());
        state.write_u32(self.offset());
    }
}
