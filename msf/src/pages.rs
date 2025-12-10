//! Page management code

use super::*;
use tracing::{trace, trace_span};
use zerocopy::FromZeros;

/// Given the size of a stream in bytes, returns the number of pages needed to store it.
///
/// This function correctly handles the case where the stream size is [`NIL_STREAM_SIZE`].
/// In this case, it returns 0.
pub(crate) fn num_pages_for_stream_size(stream_size: u32, page_size: PageSize) -> u32 {
    if stream_size == NIL_STREAM_SIZE {
        0
    } else {
        stream_size.div_round_up(page_size)
    }
}

/// Maps ranges of bytes within a stream to contiguous ranges of bytes in the containing MSF file.
pub(crate) struct StreamPageMapper<'a> {
    pages: &'a [Page],
    page_size: PageSize,
    stream_size: u32,
}

impl<'a> StreamPageMapper<'a> {
    pub(crate) fn new(pages: &'a [Page], page_size: PageSize, stream_size: u32) -> Self {
        assert_eq!(
            num_pages_for_stream_size(stream_size, page_size) as usize,
            pages.len()
        );

        Self {
            pages,
            page_size,
            stream_size,
        }
    }

    /// Maps a byte offset and a length within a stream to a contiguous run of bytes within the MSF file.
    ///
    /// Repeated calls to this function (with increasing values of `pos`) can be used to read/write
    /// the contents of a stream using the smallest number of read/write calls to the underlying
    /// MSF file.
    ///
    /// Returns `(file_offset, transfer_len)` where `file_offset` is the byte offset within the MSF
    /// file and `transfer_len` is the length of the longest contiguous sub-range of the requested
    /// range.
    ///
    /// If this returns `None` then no bytes can be mapped. This occurs when `pos >= stream_size`.
    ///
    /// Invariants:
    ///
    /// * if returned `Some`, then `transfer_len <= bytes_wanted`
    /// * if returned `Some`, then `transfer_len > 0`
    pub(crate) fn map(&self, pos: u32, bytes_wanted: u32) -> Option<(u64, u32)> {
        if self.stream_size == NIL_STREAM_SIZE {
            return None;
        }

        if pos >= self.stream_size {
            return None;
        }

        let bytes_available = self.stream_size - pos;
        let max_transfer_size = bytes_available.min(bytes_wanted);

        if max_transfer_size == 0 {
            return None;
        }

        // We will reduce transfer_size as needed.
        let transfer_size: u32;

        // Find the position within the file where the read will start.
        let first_page_index = pos >> self.page_size.exponent();
        let first_page_pointer = self.pages[first_page_index as usize];
        let first_page_file_offset = (first_page_pointer as u64) << self.page_size.exponent();

        let offset_within_first_page = pos - self.page_size.align_down(pos);
        let file_offset = first_page_file_offset + offset_within_first_page as u64;

        // Find the longest read we can execute in a single underlying read call.
        // If pages are numbered consecutively, then cover as many pages as we can.

        // Does the beginning of the read cross a page boundary?
        let bytes_available_first_page = u32::from(self.page_size) - offset_within_first_page;
        if max_transfer_size > bytes_available_first_page {
            // Yes, this read crosses a page boundary.
            // Set transfer_size to just the bytes in the first page.
            // Then, keep advancing through the page list as long as pages are sequential.
            let mut p = pos + bytes_available_first_page;
            assert!(self.page_size.is_aligned(p));

            let mut last_page_ptr = first_page_pointer;

            loop {
                assert!(
                    p - pos <= max_transfer_size,
                    "p = {p}, max_transfer_size = {max_transfer_size}"
                );
                let want_bytes = max_transfer_size - (p - pos);

                if p - pos == max_transfer_size {
                    // Reached max transfer size.
                    break;
                }

                let p_page = p >> self.page_size.exponent();

                let p_ptr = self.pages[p_page as usize];
                assert!(p_page > first_page_index);

                if p_ptr != last_page_ptr + 1 {
                    // The pages are not contiguous, so we stop here.
                    break;
                }

                // Advance over this page.
                p += want_bytes.min(u32::from(self.page_size));
                last_page_ptr += 1;
            }

            transfer_size = p - pos;
        } else {
            // This range does not cross a page boundary; it fits within a single page.
            transfer_size = max_transfer_size;
        }

        assert!(transfer_size > 0);

        assert!(
            transfer_size <= bytes_wanted,
            "transfer_size = {transfer_size}, bytes_wanted = {bytes_wanted}"
        );

        Some((file_offset, transfer_size))
    }
}

#[test]
fn test_page_mapper_nil() {
    const PAGE_SIZE: PageSize = PageSize::from_exponent(12); // 0x1000

    let mapper = StreamPageMapper::new(&[], PAGE_SIZE, NIL_STREAM_SIZE);
    assert_eq!(mapper.map(0, 0), None);
    assert_eq!(mapper.map(0x1000, 0x1000), None);
}

#[test]
fn test_page_mapper_basic() {
    const PAGE_SIZE: PageSize = PageSize::from_exponent(12); // 0x1000

    let mapper = StreamPageMapper::new(&[5, 6, 7, 300, 301], PAGE_SIZE, 0x4abc);

    assert_eq!(mapper.map(0, 0), None, "empty read within stream boundary");

    assert_eq!(
        mapper.map(0x1000_0000, 0),
        None,
        "empty read outside stream boundary"
    );

    assert_eq!(
        mapper.map(0x1000_0000, 0x1000),
        None,
        "outside stream boundary"
    );

    assert_eq!(
        mapper.map(0, 0x10),
        Some((0x5000, 0x10)),
        "aligned start, unaligned end, within first page"
    );

    assert_eq!(
        mapper.map(0, 0x1000),
        Some((0x5000, 0x1000)),
        "aligned start, aligned end, single page"
    );

    assert_eq!(
        mapper.map(0, 0x1eee),
        Some((0x5000, 0x1eee)),
        "aligned start, crosses page boundary, unaligned end"
    );

    assert_eq!(
        mapper.map(0, 0x3eee),
        Some((0x5000, 0x3000)),
        "aligned start, crosses page boundary, unaligned end, clipped at page boundary"
    );

    assert_eq!(
        mapper.map(0, 0x1000_0000),
        Some((0x5000, 0x3000)),
        "aligned start, aligned end beyond stream size, max contiguous span"
    );

    assert_eq!(
        mapper.map(0xccc, 0x10),
        Some((0x5ccc, 0x10)),
        "unaligned start, ends within first page"
    );

    assert_eq!(
        mapper.map(0xccc, 0x1000),
        Some((0x5ccc, 0x1000)),
        "unaligned start, crosses page boundary, unaligned end"
    );

    assert_eq!(
        mapper.map(0xccc, 0x1000_0000),
        Some((0x5ccc, 0x2334)),
        "unaligned start, crosses page boundary, clipped at page boundary"
    );
}

/// Contains state for allocating pages.
///
/// The `fpm`, `fpm_freed` and `fresh` bit vectors all describe the state of pages. These are
/// parallel vectors; the same index in each vector is related to the same page. Only certain
/// combinations of values for these vectors are legal.
///
/// Each page `p` can be in one of the following states:
///
/// `fpm[p]`  | `fpm_freed[p]` | State    | Description
/// ----------|----------------|----------|------------------
/// `true`    | `false`        | FREE     | The page is available for use. This page is not used by any stream.
/// `false`   | `false`        | BUSY     | The page is being used by a stream.
/// `false`   | `true`         | DELETING | The page is used by the previous (committed) state of some stream, but has been deleted in the next (uncommitted) state.
/// `true`    | `true`         | (illegal) | (illegal)
pub(super) struct PageAllocator {
    /// Free Page Map (FPM): A bit vector that lists the pages in the MSF that are free.
    pub(super) fpm: BitVec<u32, Lsb0>,

    /// A bit vector that lists the pages in the MSF that are valid in the committed state
    /// but which have been deleted in the uncommitted state.
    pub(super) fpm_freed: BitVec<u32, Lsb0>,

    /// A bit vector that tells us whether we have copied a page and it is now writable.
    /// This vector is set to all-false. Whenever we allocate a new page and copy some contents to
    /// it, or when we allocate a new page, we set the corresponding bit in `fresh`.
    pub(super) fresh: BitVec<u32, Lsb0>,

    /// The index of the next free page to check in the Free Page Map, when allocating pages.
    ///
    /// This index is not guaranteed to point to an entry that is free. It points to the next index
    /// to check.
    ///
    /// Invariant: `next_free_page <= num_pages`
    pub(super) next_free_page_hint: Page,

    /// The number of valid pages in the file. This value comes from the MSF File Header.
    pub(super) num_pages: u32,

    pub(super) page_size: PageSize,

    /// A reusable buffer whose length is `page_size`.
    #[allow(dead_code)]
    pub(super) page_buffer: Box<[u8]>,
}

impl PageAllocator {
    /// Allocate a bit vector for the FPM. We are going to compute this FPM from num_pages
    /// and the contents of the stream directory. After we finish computing it, we will also
    /// read the active FPM from disk and verify that it is exactly the same as the one that
    /// we computed.
    ///
    /// We begin with setting _all_ pages free. Then we mark Page 0 and the FPM pages as busy.
    /// It is the caller's responsibility to set other bits in the FPM accordingly.
    pub(crate) fn new(num_pages: usize, page_size: PageSize) -> Self {
        let mut fpm: BitVec<u32, Lsb0> = BitVec::with_capacity(num_pages);
        fpm.resize(num_pages, true);
        fpm.set(0, false);

        // Mark FPM pages as busy.
        for interval in 0u32.. {
            let fpm1_page = (interval << page_size.exponent()) + 1u32;
            let fpm2_page = fpm1_page + 1u32;
            if let Some(mut b) = fpm.get_mut(fpm1_page as usize) {
                b.set(false);
            }
            if let Some(mut b) = fpm.get_mut(fpm2_page as usize) {
                b.set(false);
            } else {
                break;
            }
        }

        let mut fpm_freed: BitVec<u32, Lsb0> = BitVec::with_capacity(num_pages);
        fpm_freed.resize(num_pages, false);

        let mut fresh: BitVec<u32, Lsb0> = BitVec::with_capacity(num_pages);
        fresh.resize(num_pages, false);

        Self {
            fpm,
            fpm_freed,
            fresh,
            next_free_page_hint: 0,
            num_pages: num_pages as u32,
            page_size,
            // unwrap() is for OOM handling
            page_buffer: FromZeros::new_box_zeroed_with_elems(usize::from(page_size)).unwrap(),
        }
    }

    pub(crate) fn alloc_page_buffer(&self) -> Box<[u8]> {
        // unwrap() is for OOM handling
        FromZeros::new_box_zeroed_with_elems(usize::from(self.page_size)).unwrap()
    }

    /// This marks a page as "busy but freed". This is called for pages of the Stream Directory,
    /// and when using Big MSF, for pages of the Page Map.
    ///
    /// These pages are marked "free pending" (freed) because these pages become unused after
    /// the next successful call to `commit()`.
    pub(crate) fn init_mark_stream_dir_page_busy(&mut self, page: Page) -> anyhow::Result<()> {
        // We mark the page as "freed". It is still marked "free" in the FPM.
        // The page allocator will not use this page, because it is marked "freed".

        let Some(mut b) = self.fpm.get_mut(page as usize) else {
            bail!(
                "Page {} is invalid; it is out of range (exceeds num_pages)",
                page
            );
        };

        if !*b {
            bail!(
                "Page {page} cannot be marked busy, because it is already marked busy. It may be used by more than one stream."
            );
        }
        b.set(false);

        // Now mark the page as "freed".
        let Some(mut freed) = self.fpm_freed.get_mut(page as usize) else {
            bail!(
                "Page {} is invalid; it is out of range (exceeds num_pages)",
                page
            );
        };
        if *freed {
            bail!(
                "Page {page} cannot be marked 'freed', because it is already marked freed. This indicates that the page was used more than once in the Stream Directory or Page Map."
            );
        }
        freed.set(true);

        Ok(())
    }

    /// Allocates a single page.
    ///
    /// This function does not do any disk I/O. It only updates in-memory state.
    pub(crate) fn alloc_page(&mut self) -> Page {
        let (page, run_len) = self.alloc_pages(1);
        debug_assert_eq!(run_len, 1);
        page
    }

    /// Allocates one or more contiguous pages.
    ///
    /// This function handles crossing interval boundaries. If `num_pages_wanted` is large enough that
    /// it crosses an interval boundary, then this function will return a run length that is
    /// smaller than `num_pages_wanted`.  If this function does not cross an interval boundary, then
    /// the returned `run_len` value will be equal to `num_pages_wanted`.
    ///
    /// This function does not do any disk I/O. It only updates in-memory state.
    pub(crate) fn alloc_pages(&mut self, num_pages_wanted: u32) -> (Page, u32) {
        let _span = trace_span!("alloc_pages").entered();
        trace!(num_pages_wanted);

        assert!(num_pages_wanted > 0);
        assert_eq!(self.num_pages as usize, self.fpm.len());
        assert_eq!(self.num_pages as usize, self.fpm_freed.len());
        assert!(self.next_free_page_hint <= self.num_pages);

        // First, check to see whether an existing page is free.
        if self.next_free_page_hint < self.fpm.len() as u32 {
            if let Some(i) = self.fpm.as_bitslice()[self.next_free_page_hint as usize..].first_one()
            {
                let p0: Page = self.next_free_page_hint + i as u32;

                // We found an existing free page. Mark the page as busy.
                self.fpm.set(p0 as usize, false);
                self.fresh.set(p0 as usize, true);
                self.next_free_page_hint = p0 + 1;

                let mut run_len: u32 = 1;

                // See if the pages immediately following this first page are also free.
                // If they are, then claim them, too.
                while run_len < num_pages_wanted
                    && p0 + run_len < self.num_pages
                    && self.fpm[self.next_free_page_hint as usize]
                {
                    self.fpm.set(self.next_free_page_hint as usize, false);
                    self.fresh.set(self.next_free_page_hint as usize, true);
                    self.next_free_page_hint += 1;
                    run_len += 1;
                }

                trace!(first_page = p0, run_len, "allocated pages");
                return (p0, run_len);
            }

            // There are no more free pages. Fast-forward to the end of the FPM so we don't
            // waste time re-scanning this part of the FPM on future calls.
            trace!("there are no free pages");
            self.next_free_page_hint = self.fpm.len() as u32;
        }

        // We need to add new pages to the MSF file.
        trace!(num_pages = self.num_pages, "adding new pages to MSF file");
        assert_eq!(self.next_free_page_hint, self.num_pages);
        let low_mask = (1u32 << self.page_size.exponent()) - 1;
        let page_size = u32::from(self.page_size);
        let num_pages_available = match self.num_pages & low_mask {
            0 => {
                // This is an unusual but legal case. num_pages is currently positioned exactly at
                // the beginning of an interval. There is exactly 1 usable page at the start of an
                // interval; after that page is the FPM1 and then the FPM2. So we can only allocate
                // a single page.
                trace!(
                    "num_pages is positioned on first page of an interval; can only allocate 1 page"
                );
                1
            }
            1 => {
                // num_pages is positioned on FPM1. That's fine.  Step over FPM1 and FPM2.
                // Increment phase to pretend like that's how we got here in the first place.
                trace!(
                    "num_pages is positioned on FPM1; incrementing by 2 and using remainder of interval"
                );
                self.fpm_freed.push(false); // FPM1
                self.fpm_freed.push(false); // FPM2
                self.fpm.push(false); // FPM1
                self.fpm.push(false); // FPM2
                self.num_pages += 2;
                self.next_free_page_hint += 2;
                page_size - 2
            }
            2 => {
                // We are positioned on FPM2. That's unusual but OK. Step over FPM2 and mark it
                // as busy.
                trace!(
                    "num_pages is positioned on FPM2; incrementing by 1 and using remainder of interval"
                );
                self.fpm_freed.push(false);
                self.fpm.push(false);
                self.num_pages += 1;
                self.next_free_page_hint += 1;
                page_size - 2
            }
            phase => page_size - phase + 1,
        };

        assert_eq!(self.next_free_page_hint, self.num_pages);

        let num_pages_allocated = num_pages_available.min(num_pages_wanted);
        assert!(num_pages_allocated > 0);

        let start_page = self.num_pages;
        self.num_pages += num_pages_allocated;

        // Extend the bitmaps and set their new values.
        self.fpm_freed.resize(self.num_pages as usize, false);
        self.fpm.resize(self.num_pages as usize, false);
        self.fresh.resize(self.num_pages as usize, true);

        // Advance next_free_page so that we don't keep re-scanning the same region of
        // the FPM repeatedly.
        self.next_free_page_hint = self.num_pages;

        assert_eq!(self.num_pages as usize, self.fpm.len());
        assert_eq!(self.num_pages as usize, self.fpm_freed.len());
        assert_eq!(self.num_pages as usize, self.fresh.len());

        trace!(start_page, num_pages_allocated, "allocated pages");
        (start_page, num_pages_allocated)
    }

    /// Ensures that a given page is mutable (is "fresh", i.e. can be modified in the uncommitted
    /// state).
    ///
    /// * If `*page` is not fresh, then this function allocates a new page and assigns its page
    ///   number to `*page`. This function _does not_ do any disk I/O; it does not copy the
    ///   contents of the old page to the new.
    ///
    /// * If `*page` is already fresh, then this function does nothing.
    pub(crate) fn make_page_fresh(&mut self, page: &mut Page) -> Page {
        let p = *page as usize;

        if self.fresh[p] {
            *page
        } else {
            let (new_page, _) = self.alloc_pages(1);
            self.fpm_freed.set(p, true);
            *page = new_page;
            new_page
        }
    }

    /// Ensures that a sequence of pages are "fresh" (can be modified in the uncommitted state).
    ///
    /// This function ensures that each page number in `pages` points to a fresh page. If an
    /// existing page number is not fresh, then this function will allocate a new page and replace
    /// the old page number with the new one.
    ///
    /// This function _does not_ do any disk I/O. It does not copy the contents of old pages to
    /// new pages.
    pub(crate) fn make_pages_fresh(&mut self, pages: &mut [Page]) {
        for p in pages.iter_mut() {
            self.make_page_fresh(p);
        }
    }

    /// Checks that the `fpm` and `fpm_freed` vectors are consistent.
    pub(crate) fn check_vector_consistency(&self) -> anyhow::Result<()> {
        let num_pages = self.num_pages as usize;
        assert_eq!(num_pages, self.fpm.len());
        assert_eq!(num_pages, self.fpm_freed.len());

        for i in 0..num_pages {
            let free = self.fpm[i];
            let freed = self.fpm_freed[i];

            match (free, freed) {
                (true, false) => {} // FREE
                (true, true) => {
                    bail!("Page {i} is in illegal state: marked 'free' and 'freed' (free pending)")
                }
                (false, false) => {} // BUSY
                (false, true) => {}  // FREED
            }
        }

        Ok(())
    }

    /// Merges the "freed" bit map into the "free" bitmap and clears the "freed" bitmap.
    ///
    /// This is part of the commit protocol.
    pub fn merge_freed_into_free(&mut self) {
        let fpm_words: &mut [u32] = self.fpm.as_raw_mut_slice();
        let freed_words: &mut [u32] = self.fpm_freed.as_raw_mut_slice();

        for (free, freed) in fpm_words.iter_mut().zip(freed_words.iter_mut()) {
            *free |= *freed;
            *freed = 0;
        }
    }

    /// Checks invariants that are visible at this scope.
    #[inline(never)]
    pub fn assert_invariants(&self) {
        assert!(self.num_pages > 0);
        assert_eq!(self.num_pages as usize, self.fpm.len());
        assert_eq!(self.num_pages as usize, self.fpm_freed.len());

        // Check that page 0, which stores the MSF File Header, is busy.
        assert!(!self.fpm[0], "Page 0 should always be BUSY");
        assert!(!self.fpm_freed[0], "Page 0 should never be deleted");

        // Check that the pages assigned to the FPM are marked "busy" in all intervals.

        let mut interval: u32 = 0;
        loop {
            let p = (interval << self.page_size.exponent()) as usize;
            let fpm1_index = p + 1;
            let fpm2_index = p + 2;

            if fpm1_index < self.fpm.len() {
                assert!(!self.fpm[fpm1_index], "All FPM pages should be marked BUSY");
                assert!(
                    !self.fpm_freed[fpm1_index],
                    "FPM pages should never be deleted"
                );
            }

            if fpm2_index < self.fpm.len() {
                assert!(!self.fpm[fpm2_index], "All FPM pages should be marked BUSY");
                assert!(
                    !self.fpm_freed[fpm2_index],
                    "FPM pages should never be deleted"
                );
                interval += 1;
            } else {
                break;
            }
        }

        // Check that the free/deleted bit vectors are consistent.
        for page in 0..self.num_pages {
            let is_free = self.fpm[page as usize];
            let is_freed = self.fpm_freed[page as usize];
            assert!(
                !(is_free && is_freed),
                "page {page} is in illegal state (both 'free' and 'freed')"
            );
        }
    }
}
