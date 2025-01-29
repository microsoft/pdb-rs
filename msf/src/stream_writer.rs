use super::*;
use std::cell::RefCell;
use tracing::{trace, trace_span};

/// Provides read/write access for a stream within an MSF (PDB) file.
pub struct StreamWriter<'a, F> {
    /// The stream number. This is used only for diagnostics.
    pub(super) stream: u32,

    pub(super) file: &'a F,

    /// The current byte size of this stream. Points directly into the `stream_sizes` vector.
    ///
    /// This value can be [`NIL_STREAM_SIZE`].
    pub(super) size: &'a mut u32,

    pub(super) page_allocator: &'a mut PageAllocator,

    /// The set of pages owned by this stream.
    pub(super) pages: &'a mut Vec<Page>,

    /// The seek position of this `StreamWriter`.
    ///
    /// `pos` may be greater than `size`. If data is written to a stream when `pos` is greater than
    /// (or equal to) `size`, then the stream is extended to contain the data.
    pub(super) pos: u64,
}

impl<'a, F> StreamWriter<'a, F> {
    /// The length in bytes of this stream. This will return zero for nil streams.
    pub fn len(&self) -> u32 {
        if *self.size == NIL_STREAM_SIZE {
            0
        } else {
            *self.size
        }
    }

    /// Returns `true` if the stream is zero-length or is a nil stream.
    pub fn is_empty(&self) -> bool {
        *self.size == 0 || *self.size == NIL_STREAM_SIZE
    }

    /// Replaces the contents of a stream. The length of the stream is set to the length of `data`.
    pub fn set_contents(&mut self, data: &[u8]) -> std::io::Result<()>
    where
        F: ReadAt + WriteAt,
    {
        let _span = trace_span!("StreamWriter::set_contents").entered();

        if data.len() as u64 >= NIL_STREAM_SIZE as u64 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        let data_len = data.len() as u32;

        // If the existing stream has more data than the caller, then truncate now.  This is a
        // half-hearted attempt to avoid unnecessary read-modify-write cycles.
        if *self.size > data_len {
            self.set_len(data_len)?;
        }

        self.write_core(data, 0)?;
        self.set_len(data.len() as u32)?;
        Ok(())
    }

    /// Writes data to a given position in the file.
    ///
    /// This method exists to work around the limitation of `WriteAt`, which takes `&self`
    /// instead of `&mut self`.
    pub fn write_at_mut(&mut self, buf: &[u8], offset: u64) -> std::io::Result<usize>
    where
        F: ReadAt + WriteAt,
    {
        let _span = trace_span!("StreamWriter::write_at_mut").entered();

        self.write_core(buf, offset)?;
        Ok(buf.len())
    }

    /// Writes data to a given position in the file.
    ///
    /// This method exists to work around the limitation of `WriteAt`, which takes `&self`
    /// instead of `&mut self`.
    pub fn write_all_at_mut(&mut self, buf: &[u8], offset: u64) -> std::io::Result<()>
    where
        F: ReadAt + WriteAt,
    {
        self.write_core(buf, offset)
    }

    /// Sets the length of this stream, in bytes.
    ///
    /// If the stream is a nil stream, this changes it to a non-nil stream, even if `len` is zero.
    ///
    /// If you are writing data to a stream, try to avoid using `set_len` to preallocate storage.
    /// It will allocate pages (yay) but it will waste I/O time by writing zero-fill data to
    /// all of the pages.  There is currently no optimized path for preallocating pages that relies
    /// on the underlying storage provide zero-filling pages.
    ///
    /// If you are replacing the contents of a large stream, just write everything from page zero
    /// and then call `set_len` at the end of the write.
    pub fn set_len(&mut self, mut len: u32) -> std::io::Result<()>
    where
        F: ReadAt + WriteAt,
    {
        use std::cmp::Ordering;

        let _span = trace_span!("StreamWriter::set_len").entered();
        trace!(new_len = len);

        if *self.size == NIL_STREAM_SIZE {
            trace!("stream changes from nil to non-nil");
            *self.size = 0;
        }

        let page_size = self.page_allocator.page_size;

        match Ord::cmp(&len, self.size) {
            Ordering::Equal => {
                trace!(len = self.size, "no change in stream size");
            }

            Ordering::Less => {
                // Truncating the stream. Find the number of pages that need to be freed.
                trace!(old_len = self.size, new_len = len, "reducing stream size");

                let num_pages_old = num_pages_for_stream_size(*self.size, page_size) as usize;
                let num_pages_new = num_pages_for_stream_size(len, page_size) as usize;
                assert!(num_pages_new <= num_pages_old);

                for &page in self.pages[num_pages_new..num_pages_old].iter() {
                    self.page_allocator.fpm_freed.set(page as usize, true);
                }

                self.pages.truncate(num_pages_new);
                *self.size = len;
            }

            Ordering::Greater => {
                // Zero-extend the stream.
                trace!(
                    old_len = self.size,
                    new_len = len,
                    "increasing stream size (zero-filling)"
                );

                let end_phase = offset_within_page(*self.size, page_size);
                if end_phase != 0 {
                    // Total number of bytes we need to fill
                    let total_zx_bytes = len - *self.size;

                    // zero-extend partial page
                    let end_spage = *self.size / page_size;
                    let num_zx_bytes = (u32::from(page_size) - end_phase).min(total_zx_bytes);

                    let mut page_buffer = self.page_allocator.alloc_page_buffer();
                    self.read_page(end_spage, &mut page_buffer)?;
                    page_buffer[end_phase as usize..].fill(0);
                    self.cow_page_and_write(end_spage, &page_buffer)?;

                    *self.size += num_zx_bytes;

                    len -= num_zx_bytes;
                    if len == 0 {
                        // We may have finished without reaching the end of this page.
                        return Ok(());
                    }
                }

                // The code above should have handled aligning the current size of the stream
                // (or returning if we are done).
                assert!(page_size.is_aligned(*self.size));

                let mut page_buffer = self.page_allocator.alloc_page_buffer();
                page_buffer.fill(0);

                assert!(page_size.is_aligned(*self.size));

                // num_zx_pages includes any partial page at the end.
                let num_zx_pages_wanted = (len - *self.size).div_round_up(page_size);

                let (first_page, run_len) = self.page_allocator.alloc_pages(num_zx_pages_wanted);
                assert!(run_len > 0);

                let old_num_pages = self.pages.len() as u32;

                for i in 0..run_len {
                    self.pages.push(first_page + i);
                }

                // This size increase may cover a partial page.
                *self.size += len;

                // TODO: If the app calls set_len() with a large size, this will be inefficient,
                // since we issue one write per page.  We could avoid that in the case where we
                // are extending the MSF file with fresh pages, at the end, and rely on a single
                // "set length" call to the underlying file.
                for i in 0..run_len {
                    self.write_page(old_num_pages + i, &page_buffer)?;
                }
            }
        }

        Ok(())
    }

    /// Converts this `StreamWriter` into a `RandomStreamWriter`.
    pub fn into_random(self) -> RandomStreamWriter<'a, F> {
        RandomStreamWriter {
            cell: RefCell::new(self),
        }
    }
}

impl<'a, F: ReadAt> std::io::Seek for StreamWriter<'a, F> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let new_pos: i64 = match from {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(signed_offset) => signed_offset + *self.size as i64,
            SeekFrom::Current(signed_offset) => self.pos as i64 + signed_offset,
        };

        if new_pos < 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }

        self.pos = new_pos as u64;
        Ok(self.pos)
    }
}

impl<'a, F: ReadAt> std::io::Read for StreamWriter<'a, F> {
    fn read(&mut self, dst: &mut [u8]) -> std::io::Result<usize> {
        let (n, new_pos) = super::read::read_stream_core(
            self.stream,
            self.file,
            self.page_allocator.page_size,
            *self.size,
            self.pages,
            self.pos,
            dst,
        )?;
        self.pos = new_pos;
        Ok(n)
    }
}

impl<'a, F: ReadAt + WriteAt> std::io::Write for StreamWriter<'a, F> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.write_core(buf, self.pos)?;
        self.pos += buf.len() as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct RandomStreamWriter<'a, F> {
    cell: RefCell<StreamWriter<'a, F>>,
}

impl<'a, F: ReadAt> ReadAt for RandomStreamWriter<'a, F> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        let sw = self.cell.borrow();
        let (n, _new_pos) = super::read::read_stream_core(
            sw.stream,
            &sw.file,
            sw.page_allocator.page_size,
            *sw.size,
            sw.pages,
            offset,
            buf,
        )?;
        if n != buf.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        Ok(())
    }

    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        let sw = self.cell.borrow();
        let (n, _new_pos) = super::read::read_stream_core(
            sw.stream,
            &sw.file,
            sw.page_allocator.page_size,
            *sw.size,
            sw.pages,
            offset,
            buf,
        )?;
        Ok(n)
    }
}

impl<'a, F: ReadAt + WriteAt> WriteAt for RandomStreamWriter<'a, F> {
    fn write_at(&self, buf: &[u8], offset: u64) -> std::io::Result<usize> {
        let mut sw = self.cell.borrow_mut();
        sw.write_core(buf, offset)?;
        Ok(buf.len())
    }

    fn write_all_at(&self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        let mut sw = self.cell.borrow_mut();
        sw.write_core(buf, offset)
    }
}
