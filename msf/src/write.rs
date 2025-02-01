use super::*;
use std::collections::hash_map::Entry;
use std::io::Write;
use tracing::{trace, trace_span};

impl<'a, F: ReadAt + WriteAt> StreamWriter<'a, F> {
    /// Writes data to a stream at a given offset. This is the main driver for all `write()` calls
    /// and their variants.
    ///
    /// This function has to handle a lot of complexity:
    /// * alignment of the starting position to a page boundary
    /// * alignment of the ending position to a page boundary
    /// * allocating pages for new pages
    /// * allocating pages for copy-on-write
    /// * updating the size of a stream
    /// * writing zeroes into regions that were implicitly created
    ///
    /// Returns the new write position, which is immediately after the buffer that was provided.
    ///
    /// This implementation is input-dependent. That is, we will drive all of our state transitions
    /// by "walking" through the offsets in the stream, from 0 to the end of the stream or the end
    /// of the transfer, whichever is greater.
    ///
    /// For some operations (read-modify-write cycles), we use a temporary page buffer.
    ///
    /// This function always writes all of the data in `buf`. If it cannot, it returns `Err`.
    #[inline(never)]
    pub(super) fn write_core(&mut self, mut buf: &[u8], offset: u64) -> std::io::Result<()> {
        let _span = trace_span!("StreamWriter::write_core").entered();

        if buf.is_empty() {
            return Ok(());
        }

        if *self.size == NIL_STREAM_SIZE {
            *self.size = 0;
        }

        let page_size = self.page_allocator.page_size;

        // Validate the ranges of our inputs. We validate these now so that we can compute values
        // that depend on them without worrying about overflow.
        let Ok(buf_len) = u32::try_from(buf.len()) else {
            return Err(std::io::ErrorKind::InvalidInput.into());
        };
        let Ok(mut pos) = u32::try_from(offset) else {
            return Err(std::io::ErrorKind::InvalidInput.into());
        };
        let Some(_buf_end) = pos.checked_add(buf_len) else {
            return Err(std::io::ErrorKind::InvalidInput.into());
        };

        // Is there any implicit zero extension happening? If so, handle the zero extension now.
        // Note that this may transfer a small prefix of buf, if the end of the zero-extension
        // region is unaligned (i.e. pos is unaligned). If that consumes all of the data in buf,
        // then we finish early.
        if *self.size < pos {
            self.write_zero_extend(&mut buf, &mut pos)?;
            if buf.is_empty() {
                return Ok(());
            }
            assert_eq!(pos, *self.size);
        }

        assert!(!buf.is_empty());
        assert!(pos <= *self.size);

        // Are we doing any overwrite?
        if pos < *self.size {
            self.write_overwrite(&mut buf, &mut pos)?;
            if buf.is_empty() {
                return Ok(());
            }
            assert_eq!(pos, *self.size);
        }

        assert!(!buf.is_empty());
        assert_eq!(pos, *self.size);

        // Does the write position start at an unaligned page boundary?
        if !page_size.is_aligned(pos) {
            self.write_unaligned_start_page(&mut buf, &mut pos)?;
            if buf.is_empty() {
                return Ok(());
            }
        }

        assert!(!buf.is_empty());
        assert_eq!(pos, *self.size);
        assert!(page_size.is_aligned(pos));

        // From this point on, we no longer need to cow pages.
        // All pages that we write will be newly-allocated pages.

        self.write_append_complete_pages(&mut buf, &mut pos)?;
        if buf.is_empty() {
            return Ok(());
        }

        self.write_append_final_unaligned_page(&mut buf, &mut pos)?;
        assert!(buf.is_empty());

        Ok(())
    }

    /// Append complete pages to the file.
    fn write_append_complete_pages(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;

        assert_eq!(*self.size, *pos);
        assert!(page_size.is_aligned(*pos));

        // Append complete pages. We do this iteratively so that we can minimize our calls to
        // alloc_pages(). Each iteration of this loop allocates a contiguous run of pages and
        // uses a single write() call to the lower level for transferring data.
        //
        // This is expected to be the main "hot path" for generating new PDBs (not just editing
        // existing ones). The page allocator should usually give us a long run of pages; it only
        // needs to break up runs when we cross interval boundaries (because of the FPM pages).

        loop {
            let num_pages_wanted = (buf.len() as u32) / page_size;
            if num_pages_wanted == 0 {
                break;
            }

            // Allocate pages, add them to our stream, and update the stream size.
            // These must all be done before I/O so that our in-memory state is consistent.
            let (first_page, run_len) = self.page_allocator.alloc_pages(num_pages_wanted);
            assert!(run_len > 0);
            let xfer_len: u32 = run_len << page_size.exponent();
            for i in 0..run_len {
                self.pages.push(first_page + i);
            }

            let buf_head = take_n(buf, xfer_len as usize);

            let file_offset = page_to_offset(first_page, page_size);

            trace!(
                stream_pos = *pos,
                first_page = first_page,
                file_offset,
                xfer_len,
                "write_append_complete_pages"
            );

            self.file.write_all_at(buf_head, file_offset)?;

            *self.size += xfer_len;
            *pos += xfer_len;
        }

        Ok(())
    }

    /// Append the final unaligned page to the file, if any.
    fn write_append_final_unaligned_page(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;

        assert_eq!(*self.size, *pos);
        assert!(page_size.is_aligned(*pos));

        // The only thing left is a single partial page at the end. We use the page buffer so that we
        // are sending down complete page writes to the lower-level storage device.
        assert!(buf.len() < usize::from(page_size));
        if buf.is_empty() {
            return Ok(());
        }

        let page = self.page_allocator.alloc_page();

        let mut page_buffer = self.page_allocator.alloc_page_buffer();
        page_buffer[..buf.len()].copy_from_slice(buf);
        page_buffer[buf.len()..].fill(0);

        self.pages.push(page);
        *self.size += buf.len() as u32;

        let file_offset = page_to_offset(page, page_size);

        trace!(
            stream_pos = *pos,
            page = page,
            file_offset,
            unaligned_len = buf.len(),
            "write_append_final_unaligned_page"
        );

        self.file.write_all_at(&page_buffer, file_offset)?;

        *pos += buf.len() as u32;
        *buf = &[];

        Ok(())
    }

    /// Handles zero-extending a stream. This occurs when the write position is beyond the
    /// current size of the stream. This implicitly writes zeroes from the old end of the stream
    /// to the start of the write request.
    ///
    /// This implementation will transfer the prefix of `buf` if `pos` is unaligned. If `buf` fits
    /// entirely on one page, then this finishes the entire transfer. If some portion of `buf` is
    /// transferred, then it will be written to 1 or 2 pages. It will be written to 2 pages if it
    /// crosses a page boundary.
    fn write_zero_extend(&mut self, buf: &mut &[u8], pos: &mut u32) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;

        self.write_zero_extend_unaligned_start(buf, pos)?;
        if buf.is_empty() {
            return Ok(());
        }

        // If we have more bytes to write, then write_zero_extend_unaligned_start() should
        // have aligned the stream size to a page boundary.
        assert!(page_size.is_aligned(*self.size));

        if *self.size < *pos {
            self.write_zero_extend_whole_pages(*pos)?;
        }

        if *self.size < *pos {
            self.write_zero_extend_unaligned_end(buf, pos)?;
        }

        Ok(())
    }

    fn write_zero_extend_unaligned_start(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        assert!(*self.size < *pos); // caller should have already checked this
        let num_zx = *pos - *self.size; // number of zero-extend bytes we need

        let page_size = self.page_allocator.page_size;

        // current end of the stream
        let end_spage: StreamPage = *self.size >> page_size.exponent();
        let end_phase = offset_within_page(*self.size, page_size);

        // where the new data begins
        let pos_spage: StreamPage = *pos >> page_size.exponent();
        let pos_phase = offset_within_page(*pos, page_size);

        if end_phase != 0 {
            // The end of the stream is not page-aligned and we are extending the end of the stream
            // by one or more bytes.

            // Prepare the page buffer that we are assembling. Zero-fill it.
            let mut page_buffer = self.page_allocator.alloc_page_buffer();
            page_buffer.fill(0);

            // Read the old data from the page. The read starts at phase 0 within the page and ends
            // at end_phase. If this fails, that's OK, we haven't made any state changes yet and the
            // error propagates.
            {
                let file_page = self.pages[end_spage as usize];
                let file_offset = page_to_offset(file_page, page_size);
                self.file
                    .read_exact_at(&mut page_buffer[..end_phase as usize], file_offset)?;
            }

            if end_spage == pos_spage {
                // The stream ends on the same page that new-data begins on, and the end of
                // the stream is unaligned. This means that we have a very complicated page to
                // deal with. It has old stream data, zero-extend bytes, and 1 or more bytes of
                // new data. We also have to deal with copy-on-write for the page.
                //
                // We expect zero-extending to be a rare case. We implement this by allocating a
                // page buffer, reading the unaligned piece of the old page, zeroing the middle,
                // copying the unaligned piece of the new data, and optionally zeroing the tail.
                // Then we allocate a fresh page (if needed) and write the page data that we built.
                // Then we write the page to disk and update the stream page pointer.
                //
                // There are two subcases to consider:
                // 1) the new data does not reach the end of this page (is unaligned), so there are
                //    some undefined bytes at the end of the page
                // 2) the new data reaches or crosses the end of this page (is aligned), so we
                //    "paint" the entire page to its end with new data.

                // within end_spage:
                // |------old-data-------|-------zeroes------|-------new-data-----|----
                //                       |                   |
                //               end_phase                   |
                //                                           |
                //                                           pos_phase

                assert!(end_phase <= pos_phase);

                // Copy the new data into the page buffer. The new data may end within this page
                // or may cross the boundary to the next page, which is what the min() call handles.
                let buf_head_len = (usize::from(page_size) - pos_phase as usize).min(buf.len());
                let buf_head = take_n(buf, buf_head_len);
                page_buffer[pos_phase as usize..][..buf_head.len()].copy_from_slice(buf_head);

                // Move pos because we have consumed data from 'buf'
                *pos += buf_head_len as u32;

                // In this case, all of the zero-extend bytes have been handled in this first page,
                // so we can advance pos by num_zx.
                *self.size += num_zx;
                *self.size += buf_head_len as u32;
            } else {
                // The new data does not overlap the page we are working on. That means the
                // zero-extend region reaches to the end of the page.

                // within end_spage:
                // |------old-data-------|-------zeroes-------------------------------|
                //                       |                                            |
                //               end_phase                                            |
                //                                                                    |
                //                                                               page_size

                let num_zx_this_page = u32::from(page_size) - end_phase;
                *self.size += num_zx_this_page;
            }

            // COW the page and write it.
            self.cow_page_and_write(end_spage, &page_buffer)?;
        }

        Ok(())
    }

    /// Writes zero or more complete zero pages during zero-extension. The size of the stream has
    /// already been aligned to a page boundary.
    ///
    /// This does not read data from the current transfer request, so it does not need `buf`.
    /// It also does not change `pos`.
    fn write_zero_extend_whole_pages(&mut self, pos: u32) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;

        assert!(*self.size <= pos);
        assert!(page_size.is_aligned(*self.size));

        if (pos - *self.size) / page_size == 0 {
            return Ok(());
        }

        let mut page_buffer = self.page_allocator.alloc_page_buffer();
        page_buffer.fill(0); // probably redundant

        loop {
            let num_pages_wanted = (pos - *self.size) / page_size;
            if num_pages_wanted == 0 {
                break;
            }

            let (first_page, run_len) = self.page_allocator.alloc_pages(num_pages_wanted);
            assert!(run_len > 0);
            for i in 0..run_len {
                self.pages.push(first_page + i);
            }

            let run_size_bytes = run_len << page_size.exponent();
            *self.size += run_size_bytes;

            assert!(*self.size <= pos);

            // Write the zeroed pages.
            for i in 0..run_len {
                let page = first_page + i;
                self.file
                    .write_at(&page_buffer, page_to_offset(page, page_size))?;
            }
        }

        Ok(())
    }

    /// If the zero-extend region ends with an unaligned final page, then this will write that page.
    /// This may transfer data from `buf`.
    fn write_zero_extend_unaligned_end(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;

        assert!(*self.size <= *pos);
        assert!(page_size.is_aligned(*self.size));

        // We should have at most a partial page.
        let num_zx_bytes = *pos - *self.size;
        assert!(num_zx_bytes < u32::from(page_size));

        if num_zx_bytes == 0 {
            return Ok(());
        }

        let mut page_buffer = self.page_allocator.alloc_page_buffer();

        page_buffer[0..num_zx_bytes as usize].fill(0);

        let num_data_len = buf
            .len()
            .min(usize::from(page_size) - num_zx_bytes as usize);
        let buf_head = take_n(buf, num_data_len);
        *pos += num_data_len as u32;

        page_buffer[num_zx_bytes as usize..num_zx_bytes as usize + num_data_len]
            .copy_from_slice(buf_head);

        let end_of_data = num_zx_bytes as usize + num_data_len;
        page_buffer[end_of_data..usize::from(page_size)].fill(0);

        let page = self.page_allocator.alloc_page();
        self.pages.push(page);
        *self.size += end_of_data as u32;

        self.file
            .write_at(&page_buffer, page_to_offset(page, page_size))?;

        Ok(())
    }

    /// Handles _most_ of overwrite.
    ///
    /// This handles:
    /// * the unaligned page at the start of overwrite (if any)
    /// * the complete, aligned pages in the middle of overwrite (if any)
    ///
    /// When this returns, even if buf still has data, we do not guarantee that pos == stream_size.
    fn write_overwrite(&mut self, buf: &mut &[u8], pos: &mut u32) -> std::io::Result<()> {
        assert!(*pos < *self.size);

        self.write_overwrite_unaligned_start(buf, pos)?;
        if buf.is_empty() {
            return Ok(());
        }

        assert!(self.page_allocator.page_size.is_aligned(*pos));

        self.write_overwrite_aligned_pages(buf, pos)?;
        if buf.is_empty() {
            return Ok(());
        }

        assert!(self.page_allocator.page_size.is_aligned(*pos));
        assert!(self.page_allocator.page_size.is_aligned(*self.size));
        Ok(())
    }

    /// Handles writing the first page during overwrite, if the first page is unaligned.
    ///
    /// # Requires
    /// * `pos <= stream_size`
    ///
    /// # Ensures
    /// * `buf.is_empty() || pos == stream_size`
    fn write_overwrite_unaligned_start(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        assert!(*pos <= *self.size);

        let page_size = self.page_allocator.page_size;

        let pos_spage: StreamPage = *pos >> page_size.exponent();
        let pos_phase = offset_within_page(*pos, page_size);
        if pos_phase == 0 {
            // The overwrite starts on an aligned page boundary. This function has no work to do.
            return Ok(());
        }

        // In this case, we need to assemble a page from some old data and some new data.
        // And because we need to cow the page, it is easier to reassemble everything into a
        // page buffer.
        //
        // Contents of this page:
        //
        //
        //
        //        not empty            not empty             can be empty          can be empty
        // |----old-data----------|------new-data--------|-------old-data------|-----garbage-----
        // 0                      |                      |                     |
        //                        |                      |                     |
        //                  pos_phase
        //
        // At this point, we don't know which subcase we're in. It depends on whether new_data
        // reaches the end of this page and whether writing the new-data extends the stream size.

        // Read the old page data into our page buffer. This makes cow'ing the page easier.
        // We read the entire page because that simplifies the case where the new-data ends
        // before stream_size.
        let mut page_buffer = self.page_allocator.alloc_page_buffer();
        self.read_page(pos_spage, &mut page_buffer)?;

        // Copy the data from 'buf' (new-data) into the page and advance 'buf' and 'pos'.
        let new_data_len = (usize::from(page_size) - pos_phase as usize).min(buf.len());
        assert!(new_data_len > 0);
        let buf_head = take_n(buf, new_data_len);
        page_buffer[pos_phase as usize..][..buf_head.len()].copy_from_slice(buf_head);
        *pos += new_data_len as u32;

        // Cow the page and write its new contents.
        self.cow_page_and_write(pos_spage, &page_buffer)?;

        // We may have written enough data that we extended stream_size. This only happens if we
        // drag in some of the prefix of buf.
        if *pos > *self.size {
            *self.size = *pos;
        }

        Ok(())
    }

    /// If we are doing overwrite and the remaining buffer contains one or more whole pages,
    /// then this function transfers those.
    ///
    /// This function may extend the stream size. If it does, then it will not extend it enough
    /// to cross a page boundary. This form of extension happens when we are overwriting beyond
    /// the existing end of the stream.
    ///
    /// This function will cow pages, but will not allocate new page slots in the stream.
    ///
    /// # Requires
    /// * `pos` is page-aligned
    /// * `pos <= stream_size`
    ///
    /// # Ensures
    /// * `pos` is page-aligned
    /// * `buf.is_empty() || stream_size is page-aligned`
    fn write_overwrite_aligned_pages(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        let page_size = self.page_allocator.page_size;
        assert!(*pos <= *self.size);
        assert!(page_size.is_aligned(*pos));

        if *pos == *self.size {
            return Ok(());
        }

        // The stream page where this transfer will begin (if any).
        let pos_spage = *pos / page_size;

        // Number of complete pages that can be read from buf.
        let num_buf_pages = buf.len() as u32 / page_size;

        // Number of pages at our write position that are assigned to the stream.
        // This includes the partial page at the end, if any.
        let num_pages_total = self.size.div_round_up(page_size);
        assert_eq!(num_pages_total, self.pages.len() as u32);
        let num_pages_at_pos = *pos / page_size;
        let num_pages_owned = num_pages_total - num_pages_at_pos;

        // Number of complete pages we are going to transfer from buf to disk.
        // Since we are writing whole pages, we do not need the old contents of any page.
        // Cow the pages, just so we get fresh pages.
        let num_xfer_pages = num_pages_owned.min(num_buf_pages);
        if num_xfer_pages != 0 {
            trace!(num_pages = num_xfer_pages, "writing whole pages");

            let num_xfer_bytes = num_xfer_pages << page_size.exponent();
            let buf_head = take_n(buf, num_xfer_bytes as usize);
            *pos += num_xfer_bytes;

            let pages = &mut self.pages[pos_spage as usize..][..num_xfer_pages as usize];
            self.page_allocator.make_pages_fresh(pages);
            write_runs(&self.file, buf_head, pages, page_size)?;

            // If the last page that we overwrite was a partial page, then we will have extended
            // the size of the stream. This will not extend stream_size beyond a page boundary.
            if *pos > *self.size {
                *self.size = *pos;
            }

            if buf.is_empty() {
                return Ok(());
            }
        }

        assert!(page_size.is_aligned(*pos));
        assert!(*pos <= *self.size);

        // We may have gotten here because buf.len() is now less than a full page size, but we still
        // have another page assigned in the stream. Cow it now.
        if *self.size - *pos > 0 {
            trace!(buf_len = buf.len(), "buffer has partial page remaining.");
            assert!(
                buf.len() < usize::from(page_size),
                "size = {:x}, pos = {:x}, buf.len = 0x{:x}",
                *self.size,
                *pos,
                buf.len()
            );

            let spage = *pos / page_size;
            let old_len = *self.size - *pos;

            let mut page_buffer = self.page_allocator.alloc_page_buffer();
            if old_len > buf.len() as u32 {
                self.read_page(spage, &mut page_buffer)?;
            }

            page_buffer[..buf.len()].copy_from_slice(buf);

            self.cow_page_and_write(spage, &page_buffer)?;
            *pos += buf.len() as u32;
            if *pos > *self.size {
                *self.size = *pos;
            }

            *buf = &[];
            return Ok(());
        }

        assert_eq!(*pos, *self.size);

        Ok(())
    }

    /// Writes the unaligned start page at the beginning of the new-data. This page is only
    /// present if `pos` is not aligned.
    ///
    /// # Requires
    /// * `stream_size == pos`
    fn write_unaligned_start_page(
        &mut self,
        buf: &mut &[u8],
        pos: &mut u32,
    ) -> std::io::Result<()> {
        assert_eq!(*pos, *self.size);

        // In this case, we need to assemble a page from some old data and some new data.
        // And because we need to cow the page, it is easier to reassemble everything into a
        // page buffer.
        //
        // Contents of this page:
        //
        //
        //
        //        not empty            not empty             can be empty          can be empty
        // |----old-data----------|------new-data--------|-------old-data------|-----garbage-----
        // 0                      |                      |                     |
        //                        |                      |                     |
        //                  pos_phase
        //
        // At this point, we don't know which subcase we're in. It depends on whether new_data
        // reaches the end of this page and whether writing the new-data extends the stream size.

        let page_size = self.page_allocator.page_size;

        // where the new data begins
        let pos_spage: StreamPage = *pos >> page_size.exponent();
        let pos_phase = offset_within_page(*pos, page_size); // <-- we know this is non-zero

        // Read the old page data into our page buffer. This makes cow'ing the page easier.
        // We read the entire page because that simplifies the case where the new-data ends
        // before stream_size.
        let mut page_buffer = self.page_allocator.alloc_page_buffer();

        let file_offset = page_to_offset(self.pages[pos_spage as usize], page_size);
        trace!(
            stream_pos = *pos,
            file_offset,
            len = u32::from(page_size),
            "write_unaligned_start_page: reading existing unaligned data"
        );

        self.file.read_exact_at(&mut page_buffer, file_offset)?;

        // Copy the data from 'buf' (new-data) into the page and advance 'buf' and 'pos'.
        let new_data_len = (usize::from(page_size) - pos_phase as usize).min(buf.len());
        assert!(new_data_len > 0);
        let buf_head = take_n(buf, new_data_len);
        page_buffer[pos_phase as usize..][..buf_head.len()].copy_from_slice(buf_head);
        *pos += new_data_len as u32;

        // Cow the page and write its new contents.
        self.cow_page_and_write(pos_spage, &page_buffer)?;

        // We may have written enough data that we extended stream_size.
        if *pos > *self.size {
            *self.size = *pos;
        }

        Ok(())
    }

    /// Ensures that a stream page is writable (is "fresh"). If the page is not writable, then it
    /// allocates a new page. This function returns the page number of the writable page.
    fn cow_page(&mut self, spage: StreamPage) -> Page {
        self.page_allocator
            .make_page_fresh(&mut self.pages[spage as usize])
    }

    /// Ensures that a stream page is writable and then writes it.
    ///
    /// `data` should contain at most one page of data.
    pub(super) fn cow_page_and_write(
        &mut self,
        spage: StreamPage,
        data: &[u8],
    ) -> std::io::Result<()> {
        // At most one full page of data can be written.
        debug_assert!(
            data.len() <= usize::from(self.page_allocator.page_size),
            "buffer cannot exceed size of a single page"
        );

        let page = self.cow_page(spage);
        let file_offset = page_to_offset(page, self.page_allocator.page_size);

        trace!(
            stream_page = spage,
            file_offset,
            len = u32::from(self.page_allocator.page_size),
            "cow_page_and_write"
        );

        self.file.write_all_at(data, file_offset)
    }

    /// Reads a stream page.  The length of `data` must be exactly one page, or less. It cannot
    /// cross page boundaries.
    pub(super) fn read_page(
        &self,
        stream_page: StreamPage,
        data: &mut [u8],
    ) -> std::io::Result<()> {
        debug_assert!(
            data.len() <= usize::from(self.page_allocator.page_size),
            "buffer cannot exceed size of a single page"
        );

        let page = self.pages[stream_page as usize];
        let offset = page_to_offset(page, self.page_allocator.page_size);
        self.file.read_exact_at(data, offset)
    }

    /// The caller **must** guarantee that this page is already writable.
    pub(super) fn write_page(&self, stream_page: StreamPage, data: &[u8]) -> std::io::Result<()> {
        let page = self.pages[stream_page as usize];
        assert!(
            self.page_allocator.fresh[page as usize],
            "page is required to be fresh (writable)"
        );

        let file_offset = page_to_offset(page, self.page_allocator.page_size);

        trace!(
            stream_page,
            page = page,
            file_offset,
            len = u32::from(self.page_allocator.page_size),
            "write_page"
        );

        self.file.write_all_at(data, file_offset)
    }
}

/// Finds the length of the prefix of pages in `pages` that are numbered sequentially.
///
/// This function assumes that there are no entries with the value 0xffff_ffff. (That would cause
/// overflow.)
fn find_longest_page_run(pages: &[Page]) -> usize {
    if pages.is_empty() {
        0
    } else {
        let mut prev = pages[0];
        let mut i = 1;
        while i < pages.len() && pages[i] == prev + 1 {
            prev = pages[i];
            i += 1;
        }
        i
    }
}

#[test]
fn test_find_longest_page_run() {
    assert_eq!(find_longest_page_run(&[]), 0);
    assert_eq!(find_longest_page_run(&[1]), 1);
    assert_eq!(find_longest_page_run(&[1, 2, 3]), 3);
    assert_eq!(find_longest_page_run(&[1, 3, 2]), 1);
    assert_eq!(find_longest_page_run(&[1, 2, 3, 9, 9, 9]), 3);
}

/// Given a page map that corresponds to a buffer of data to write, write all of the data.
/// Write it in a sequence of function calls that group together consecutive pages, so that
/// we minimize the number of write() calls.
fn write_runs<F: WriteAt>(
    file: &F,
    mut buf: &[u8],
    pages: &[Page],
    page_size: PageSize,
) -> std::io::Result<()> {
    let mut region_pages = pages;

    assert_eq!(buf.len(), pages.len() << page_size.exponent());

    loop {
        let run_len = find_longest_page_run(region_pages);
        if run_len == 0 {
            break;
        }

        let page0: Page = region_pages[0];
        let xfer_len: usize = run_len << page_size.exponent();
        let buf_head = take_n(&mut buf, xfer_len);

        // Write the run of pages. If this write fails, then the contents of the stream are
        // now in an undefined state.
        file.write_at(buf_head, page_to_offset(page0, page_size))?;

        // Advance iterator state
        region_pages = &region_pages[run_len..];
    }

    Ok(())
}

impl<F> Msf<F> {
    /// Adds a new stream to the MSF file. The stream has a length of zero.
    pub fn new_stream(&mut self) -> anyhow::Result<(u32, StreamWriter<'_, F>)> {
        let _span = trace_span!("new_stream").entered();

        self.requires_writeable()?;
        self.check_can_add_stream()?;

        let new_stream_index = self.stream_sizes.len() as u32;
        trace!(new_stream_index);

        self.stream_sizes.push(0);
        let size = self.stream_sizes.last_mut().unwrap();

        let pages = match self.modified_streams.entry(new_stream_index) {
            Entry::Occupied(_) => {
                panic!("Found entry in modified streams table that should not be present.")
            }
            Entry::Vacant(v) => v.insert(Vec::new()),
        };

        Ok((
            new_stream_index,
            StreamWriter {
                stream: new_stream_index,
                file: &self.file,
                size,
                page_allocator: &mut self.pages,
                pos: 0,
                pages,
            },
        ))
    }

    fn check_can_add_stream(&self) -> anyhow::Result<()> {
        if self.stream_sizes.len() as u32 >= self.max_streams {
            bail!("A new stream cannot be created because the maximum number of streams has been reached.");
        }
        Ok(())
    }

    /// Adds a new stream to the MSF file, given the byte contents. This function returns the
    /// stream index of the new stream.
    pub fn new_stream_data(&mut self, data: &[u8]) -> anyhow::Result<u32>
    where
        F: ReadAt + WriteAt,
    {
        let (stream_index, mut writer) = self.new_stream()?;
        writer.set_contents(data)?;
        Ok(stream_index)
    }

    /// Adds a new nil stream to the MSF file.
    pub fn nil_stream(&mut self) -> anyhow::Result<u32> {
        self.requires_writeable()?;
        self.check_can_add_stream()?;

        let new_stream_index = self.stream_sizes.len() as u32;
        self.stream_sizes.push(NIL_STREAM_SIZE);

        match self.modified_streams.entry(new_stream_index) {
            Entry::Occupied(_) => {
                panic!("Found entry in modified streams table that should be present.")
            }
            Entry::Vacant(v) => {
                v.insert(Vec::new());
            }
        }

        Ok(new_stream_index)
    }

    /// Given the stream index for a stream, returns a `StreamWriter` that allows read/write
    /// for the stream.
    ///
    /// If `stream` is out of range for the current set of streams, then the set of streams is
    /// increased until `stream` is in range. For example, if a new MSF file is created, then
    /// it is legal to immediately call `msf.write_stream(10)` on it. This will expand the Stream
    /// Directory so that `num_streams()` returns 11 (because it must include the new stream index).
    /// All streams lower than `stream` will be allocated as nil streams.
    ///
    /// If `stream` is currently a nil stream, then this function promotes the stream to a
    /// non-nil stream.
    pub fn write_stream(&mut self, stream: u32) -> anyhow::Result<StreamWriter<'_, F>> {
        assert!(stream <= MAX_STREAM);
        self.requires_writeable()?;

        while (self.stream_sizes.len() as u32) <= stream {
            _ = self.nil_stream()?;
        }

        let Some(size) = self.stream_sizes.get_mut(stream as usize) else {
            bail!("Stream index is out of range");
        };

        // If the stream is currently a nil stream, then promote it to a zero-length stream.
        if *size == NIL_STREAM_SIZE {
            *size = 0;
        }

        let pages = match self.modified_streams.entry(stream) {
            Entry::Occupied(occ) => occ.into_mut(),
            Entry::Vacant(v) => {
                // Copy the existing page list to a new page list.
                //
                // Copying the page list _does not_ imply that we can safely write to those pages,
                // because they may still be owned by the previous committed state. Copy-on-write
                // is handled elsewhere.
                let starts = &self.committed_stream_page_starts[stream as usize..];
                let old_pages =
                    &self.committed_stream_pages[starts[0] as usize..starts[1] as usize];
                v.insert(old_pages.to_vec())
            }
        };

        Ok(StreamWriter {
            stream,
            file: &self.file,
            size,
            page_allocator: &mut self.pages,
            pos: 0,
            pages,
        })
    }

    pub(crate) fn requires_writeable(&self) -> anyhow::Result<()> {
        match self.access_mode {
            AccessMode::ReadWrite => Ok(()),
            AccessMode::Read => bail!("This PDB was not opened for read/write access."),
        }
    }

    /// Copies a stream from another PDB/MSF into this one.
    pub fn copy_stream<Input: ReadAt>(
        &mut self,
        source: &Msf<Input>,
        source_stream: u32,
    ) -> anyhow::Result<u32>
    where
        F: ReadAt + WriteAt,
    {
        const BUFFER_LEN: usize = 16 << 20; // 16 MiB

        let mut source_reader = source.get_stream_reader(source_stream)?;
        let source_len = source_reader.len();

        let mut buffer = vec![0; (source_len as usize).min(BUFFER_LEN)];
        let (dest_stream_index, mut dest_writer) = self.new_stream()?;

        loop {
            let n = source_reader.read(&mut buffer)?;
            if n == 0 {
                break;
            }

            dest_writer.write_all(&buffer[..n])?;
        }

        Ok(dest_stream_index)
    }

    /// Copies a stream that implements `Read` into this PDB/MSF file.
    pub fn copy_stream_read<Input: Read>(&mut self, source: &mut Input) -> anyhow::Result<u32>
    where
        F: ReadAt + WriteAt,
    {
        const BUFFER_LEN: usize = 16 << 20; // 16 MiB

        let mut buffer = vec![0; BUFFER_LEN];
        let (dest_stream_index, mut dest_writer) = self.new_stream()?;

        loop {
            let n = source.read(&mut buffer)?;
            if n == 0 {
                break;
            }

            dest_writer.write_all(&buffer[..n])?;
        }

        Ok(dest_stream_index)
    }

    /// Copies a stream that implements `ReadAt` into this PDB/MSF file.
    pub fn copy_stream_read_at<Input: ReadAt>(&mut self, source: &Input) -> anyhow::Result<u32>
    where
        F: ReadAt + WriteAt,
    {
        const BUFFER_LEN: usize = 16 << 20; // 16 MiB

        let mut buffer = vec![0; BUFFER_LEN];
        let (dest_stream_index, mut dest_writer) = self.new_stream()?;

        let mut pos: u64 = 0;

        loop {
            let n = source.read_at(&mut buffer, pos)?;
            if n == 0 {
                break;
            }

            dest_writer.write_all(&buffer[..n])?;
            pos += n as u64;
        }

        Ok(dest_stream_index)
    }
}

/// Splits a slice `items` at a given index `n`. The slice is modified to point to the items
/// after `n`. The function returns the items up to `n`.
fn take_n<'a, T>(items: &mut &'a [T], n: usize) -> &'a [T] {
    let (lo, hi) = items.split_at(n);
    *items = hi;
    lo
}
