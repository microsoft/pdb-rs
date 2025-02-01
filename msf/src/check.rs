//! Consistency checks for MSF, both in-memory and on-disk

use super::*;

impl<F> Msf<F> {
    #[cfg(not(test))]
    #[inline(always)]
    pub(super) fn assert_invariants(&self) {}

    #[cfg(test)]
    #[inline(never)]
    pub(super) fn assert_invariants(&self) {
        // There is always at least one stream, because stream 0 is special.
        assert!(!self.stream_sizes.is_empty());

        // This is implied by stream_sizes not being empty.
        assert!(self.committed_stream_page_starts.len() >= 2);

        // The pages assigned to Stream 0 are marked "free" and are not marked "deleted".
        {
            let stream0_pages = &self.committed_stream_pages[self.committed_stream_page_starts[0]
                as usize
                ..self.committed_stream_page_starts[1] as usize];

            for &page in stream0_pages.iter() {
                assert!(self.pages.fpm[page as usize]);
                assert!(!self.pages.fpm_freed[page as usize]);
            }
        }

        // The pages assigned to all streams in the committed state are disjoint.
        // The pages assigned to all streams are < num_pages.
        {
            let mut busy_pages: BitVec<u32, Lsb0> = BitVec::new();
            busy_pages.resize(self.pages.num_pages as usize, false);
            for &page in self.committed_stream_pages.iter() {
                assert!(page <= self.pages.num_pages, "page {page} is out of range",);
                assert!(
                    !busy_pages[page as usize],
                    "page {page} is used by more than one stream",
                );
                assert!(
                    !self.pages.fresh[page as usize],
                    "page {page} cannot be fresh if it is used by a committed stream"
                );

                busy_pages.set(page as usize, true);
            }
        }

        // All entries in modified_streams have a stream index that is valid.
        for &stream_index in self.modified_streams.keys() {
            assert!(stream_index < self.stream_sizes.len() as u32);
        }
    }
}
