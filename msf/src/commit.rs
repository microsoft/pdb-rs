//! Commits all pending changes to an MSF file.

use super::*;
use anyhow::Result;
use tracing::{debug, debug_span, info, info_span, trace, trace_span};

impl<F: ReadAt + WriteAt> Msf<F> {
    /// Commits all changes to the MSF file to disk.
    ///
    /// Returns `Ok(true)` if this `Msf` contained uncommitted changes and these changes have now
    /// been committed.
    ///
    /// Returns `Ok(false)` if this `Msf` did not contain any uncomitted changes. In this case,
    /// no `write()` calls are issued to the underlying storage.
    pub fn commit(&mut self) -> Result<bool> {
        let _span = info_span!("Msf::commit").entered();

        self.assert_invariants();

        // If this was not opened for write access then there are no pending changes at all.
        if self.access_mode != AccessMode::ReadWrite {
            info!("this Msf is not opened for read-write access");
            debug_assert!(self.modified_streams.is_empty());
            return Ok(false);
        };

        // We only support modifying Big MSF files.
        assert_eq!(self.kind, MsfKind::Big);

        // If no streams have been modified, then there is nothing to do.
        if self.modified_streams.is_empty() {
            info!("there are no modified streams; nothing to commit");
            return Ok(false);
        }

        let new_fpm_number: u32 = match self.active_fpm {
            FPM_NUMBER_1 => FPM_NUMBER_2,
            FPM_NUMBER_2 => FPM_NUMBER_1,
            _ => panic!("Active FPM has invalid value"),
        };
        info!(
            old_fpm = self.active_fpm,
            new_fpm = new_fpm_number,
            "beginning commit"
        );

        let stream_dir_info = self.write_new_stream_dir()?;

        self.pages.merge_freed_into_free();
        fill_last_word_of_fpm(&mut self.pages.fpm);

        self.write_fpm(new_fpm_number)?;

        let page_size = self.pages.page_size;
        let page_size_usize = usize::from(page_size);

        // Build the new Page 0.
        let mut page0: Vec<u8> = vec![0; page_size_usize];

        let msf_header = MsfHeader {
            magic: MSF_BIG_MAGIC,
            page_size: U32::new(u32::from(page_size)),
            active_fpm: U32::new(new_fpm_number),
            num_pages: U32::new(self.pages.num_pages),
            stream_dir_size: U32::new(stream_dir_info.dir_size),
            stream_dir_small_page_map: U32::new(0),
            // The stream directory page map pointers follows the MsfHeader.
        };

        let msf_header_bytes = msf_header.as_bytes();

        page0.as_mut_slice()[..msf_header_bytes.len()].copy_from_slice(msf_header_bytes.as_bytes());

        // Copy the stream dir page map into Page 0.
        let page_map_pages_bytes = stream_dir_info.map_pages.as_bytes();
        page0[STREAM_DIR_PAGE_MAP_FILE_OFFSET as usize..][..page_map_pages_bytes.len()]
            .copy_from_slice(page_map_pages_bytes);

        // ------------------------ THE BIG COMMIT ----------------------

        info!("writing MSF File Header");
        self.file.write_all_at(page0.as_bytes(), 0)?;

        // After this point, _nothing can fail_.
        // Any operation that could have failed should have been moved above the commit point.

        // --------------------- CLEANUP AFTER THE COMMIT ---------------

        // Update in-memory state to reflect the commit.
        //
        // This code runs after we write the new Page 0 to disk. That commits the changes to the
        // PDB. This function modifies in-memory state to reflect the successful commit. For this
        // reason, after this point, this function must NEVER return a failure code.
        {
            // Build the new in-memory stream directory. This is very similar to the version that we
            // just wrote to disk, so maybe we should unify the two.

            let _span = trace_span!("post_commit").entered();

            let page_size = self.pages.page_size;

            // We can easily determine the right size for allocating 'stream_pages'.
            let mut num_stream_pages: usize = 0;
            for &stream_size in self.stream_sizes.iter() {
                if stream_size != NIL_STREAM_SIZE {
                    num_stream_pages += num_pages_for_stream_size(stream_size, page_size) as usize;
                }
            }

            let mut stream_pages: Vec<Page> = Vec::with_capacity(num_stream_pages);
            let mut stream_page_starts: Vec<u32> = Vec::with_capacity(self.stream_sizes.len() + 1);

            for (stream, &stream_size) in self.stream_sizes.iter().enumerate() {
                stream_page_starts.push(stream_pages.len() as u32);

                if stream_size == NIL_STREAM_SIZE {
                    trace!(stream, "stream is nil");
                    continue;
                }

                let num_stream_pages = num_pages_for_stream_size(stream_size, page_size) as usize;

                // If this stream has been modified, then return the modified page list.
                let is_modified;
                let pages: &[Page] =
                    if let Some(pages) = self.modified_streams.get(&(stream as u32)) {
                        is_modified = true;
                        pages
                    } else {
                        is_modified = false;
                        let start = self.committed_stream_page_starts[stream] as usize;
                        &self.committed_stream_pages[start..start + num_stream_pages]
                    };
                assert_eq!(num_stream_pages, pages.len());

                trace!(stream, stream_size, num_stream_pages, is_modified,);

                stream_pages.extend_from_slice(pages);
            }

            stream_page_starts.push(stream_pages.len() as u32);

            // Now that we have written the Stream Directory (and the map pages, above it), we
            // need to mark the pages that contain the Stream Directory (and the map pages) as
            // *freed*.  Not free, but *freed*.  Fortunately, we still have this information, since
            // we built it above when we called write_new_stream_dir().
            //
            // The multiple_commits() and many_commits() tests verify this.
            {
                trace!("marking stream dir pages as free");

                for list in [
                    stream_dir_info.dir_pages.as_slice(),
                    stream_dir_info.map_pages.as_slice(),
                ] {
                    for &p in list.iter() {
                        let pi = p.get() as usize;
                        assert!(!self.pages.fpm_freed[pi]);
                        assert!(!self.pages.fpm[pi]);
                        self.pages.fpm_freed.set(pi, true);
                    }
                }
            }

            // Update state
            self.committed_stream_pages = stream_pages;
            self.committed_stream_page_starts = stream_page_starts;
            self.modified_streams.clear();

            self.pages.fresh.set_elements(0);
            self.pages.next_free_page_hint = 3; // positioned after file header and FPM1 and FPM2

            trace!(new_fpm_number, "setting active FPM");
            self.active_fpm = new_fpm_number;
        }

        info!("commit complete");

        self.assert_invariants();

        Ok(true)
    }

    /// Builds the new stream directory.
    fn build_new_stream_dir(&self) -> Vec<U32<LE>> {
        let page_size = self.pages.page_size;

        let num_streams = self.stream_sizes.len();

        let mut stream_dir: Vec<U32<LE>> = Vec::new();
        stream_dir.push(U32::new(num_streams as u32));

        // Push a size of 0 for Stream 0.
        stream_dir.push(U32::new(0));

        for &stream_size in self.stream_sizes[1..].iter() {
            stream_dir.push(U32::new(stream_size));
        }

        for (stream, &stream_size) in self.stream_sizes.iter().enumerate() {
            if stream_size == NIL_STREAM_SIZE {
                debug!("stream {stream} : NIL");
                continue;
            }

            let num_stream_pages = num_pages_for_stream_size(stream_size, page_size) as usize;

            // If this stream has been modified, then return the modified page list.
            let pages: &[Page] = if let Some(pages) = self.modified_streams.get(&(stream as u32)) {
                pages
            } else {
                let start = self.committed_stream_page_starts[stream] as usize;
                &self.committed_stream_pages[start..start + num_stream_pages]
            };
            assert_eq!(num_stream_pages, pages.len());
            debug!(stream, stream_size);

            stream_dir.reserve(pages.len());
            for &p in pages.iter() {
                stream_dir.push(U32::new(p));
            }
        }

        stream_dir
    }

    /// Builds the new stream directory and writes it to disk.
    ///
    /// This builds the stream directory and the page map pages and writes it to disk. It returns
    /// the size in bytes of the stream directory and the page numbers of the page map.
    fn write_new_stream_dir(&mut self) -> anyhow::Result<StreamDirInfo> {
        let _span = debug_span!("Msf::write_new_stream_dir").entered();

        let page_size = self.pages.page_size;
        let page_size_usize = usize::from(page_size);

        // "Dir" pages contain the contents of the Stream Directory.
        // "Map" pages contain pointers to "dir" pages. They are "above" the dir pages.

        let stream_dir = self.build_new_stream_dir();
        let stream_dir_bytes = stream_dir.as_bytes();

        let mut reusable_page_data: Vec<u8> = vec![0; usize::from(page_size)];

        // The number of pages needed to store the Stream Directory.
        let num_stream_dir_pages =
            num_pages_for_stream_size(stream_dir_bytes.len() as u32, page_size) as usize;
        let mut dir_pages: Vec<U32<LE>> = Vec::with_capacity(num_stream_dir_pages);

        for stream_dir_chunk in stream_dir_bytes.chunks(page_size_usize) {
            // Allocate a page for the next stream dir page.
            let page = self.pages.alloc_page();
            dir_pages.push(U32::new(page));

            let page_bytes = if stream_dir_chunk.len() == page_size_usize {
                // It's a complete page, so there is no need for the bounce buffer.
                stream_dir_chunk
            } else {
                let (lo, hi) = reusable_page_data.split_at_mut(stream_dir_chunk.len());
                lo.copy_from_slice(stream_dir_chunk);
                hi.fill(0);
                reusable_page_data.as_slice()
            };

            let page_offset = page_to_offset(page, page_size);
            debug!(page, page_offset, "writing stream dir page");
            self.file.write_all_at(page_bytes, page_offset)?;
        }

        // Now we build the next level of indirection (the "page map"), and allocate pages for them
        // and write them.
        let mut map_pages: Vec<U32<LE>> = Vec::new();

        let num_u32s_per_page = u32::from(page_size) / 4;
        for map_page_contents in dir_pages.chunks(num_u32s_per_page as usize) {
            let map_page_index = self.pages.alloc_page();
            let map_file_offset = page_to_offset(map_page_index, page_size);
            let map_page_bytes = map_page_contents.as_bytes();
            let (lo, hi) = reusable_page_data.split_at_mut(map_page_bytes.len());
            lo.copy_from_slice(map_page_bytes);
            hi.fill(0);

            debug!(
                map_page_index,
                map_file_offset, "writing stream dir page map page"
            );
            self.file
                .write_all_at(&reusable_page_data, map_file_offset)?;

            map_pages.push(U32::new(map_page_index));
        }

        Ok(StreamDirInfo {
            dir_size: stream_dir_bytes.len() as u32,
            dir_pages,
            map_pages,
        })
    }

    /// Writes the FPM for the new transaction state.
    fn write_fpm(&mut self, new_fpm_number: u32) -> anyhow::Result<()> {
        let _span = debug_span!("write_fpm").entered();

        let page_size = self.pages.page_size;
        let page_size_usize = usize::from(page_size);
        let num_intervals = self.pages.num_pages.div_round_up(page_size);

        assert_eq!(self.pages.num_pages as usize, self.pages.fpm.len());
        let fpm_words: &[u32] = self.pages.fpm.as_raw_slice();
        let fpm_bytes: &[u8] = fpm_words.as_bytes();

        // This iterates the contents of the pages of the FPM. Each item iterated is a &[u8]
        // containing the piece of the FPM that should be written to a single on-disk page.
        // The last page iterated can be a partial (incomplete) page.
        //
        // For example: page_size = 4096, so there are 4096 bytes in each FPM page within
        // an interval.  That means there are 4096 * 8 bits in each FPM page, or 32,768 bits.
        // These bits cover _much_ more than a single interval; each FPM page covers 8
        // intervals worth of pages.
        //
        // This is basically a bug in the design of the FPM; the FPM is 8x larger than it
        // needs to be. But the design is frozen, so we must do it this way.

        let mut fpm_pages_data_iter = fpm_bytes.chunks(page_size_usize);

        // This is a buffer where we assemble complete FPM pages before writing them to disk.
        // This ensures that we always write a complete page. This is more efficient for storage
        // stacks, since pages are usually larger than on-disk block sizes and are block-size
        // aligned, so this avoids the need for a read-modify-write cycle in the underlying
        // filesystem. This is only necessary for the last (partial) page.
        let mut fpm_page_buffer: Vec<u8> = vec![0; page_size_usize];

        for interval_index in 0..num_intervals {
            let this_fpm_page_data = fpm_pages_data_iter.next().unwrap_or(&[]);
            assert!(this_fpm_page_data.len() <= fpm_page_buffer.len());

            let slice_to_write = if this_fpm_page_data.len() < page_size_usize {
                fpm_page_buffer[..this_fpm_page_data.len()].copy_from_slice(this_fpm_page_data);
                fpm_page_buffer[this_fpm_page_data.len()..].fill(0xff); // fill the rest with "free"
                fpm_page_buffer.as_slice()
            } else {
                // We already have a perfectly-sized slice. Just use it.
                this_fpm_page_data
            };

            let interval_page = interval_to_page(interval_index, page_size);
            let new_fpm_page = interval_page + new_fpm_number;

            debug!(interval = interval_index, "writing fpm chunk");

            self.file
                .write_all_at(slice_to_write, page_to_offset(new_fpm_page, page_size))?;
        }

        Ok(())
    }
}

/// This ensures that the last few bits of the FPM are set to "free".
///
/// The MSPDB library uses a bit vector implementation that packs bits into an array of `u32`
/// values, just as this Rust implementation does. However, if the number of _bits_ in the FPM
/// is not a multiple of 32, then the MSPDB library accidentally reads the unaligned bits in the
/// last `u32` and expects them to be "free".
fn fill_last_word_of_fpm(fpm: &mut BitVec<u32, Lsb0>) {
    let unaligned_len = fpm.len() & 0x1f;
    if unaligned_len == 0 {
        return;
    }

    let fpm_words = fpm.as_raw_mut_slice();
    let last = fpm_words.last_mut().unwrap();

    // Because unaligned_len is the result of masking with 0x1f, we know that the shift count
    // cannot overflow.
    *last |= 0xffff_ffff << unaligned_len;
}

/// Information about the new Stream Directory that we just constructed and wrote to disk.
struct StreamDirInfo {
    /// Size in bytes of the Stream Directory
    dir_size: u32,

    /// The list of pages that contain the Stream Directory
    dir_pages: Vec<U32<LE>>,

    /// The list of pages that contain the Stream Directory Map (the level _above_ `dir_pages`)
    map_pages: Vec<U32<LE>>,
}
