//! Code for opening or creating MSF files.

use super::*;
use sync_file::RandomAccessFile;
use tracing::{trace, trace_span, warn};
use zerocopy::IntoBytes;

/// Options for creating a new PDB/MSF file.
#[derive(Clone, Debug)]
pub struct CreateOptions {
    /// The page size to use. This must be in the range [`MIN_PAGE_SIZE..=MAX_PAGE_SIZE`].
    pub page_size: PageSize,

    /// The maximum number of streams that we will allow to be created using `new_stream` or
    /// `nil_stream`. The default value is 0xfffe, which prevents overflowing the 16-bit stream
    /// indexes that are used in PDB (or confusing them with the "nil" stream index).
    ///
    /// Applications may increase this value beyond the default, but this will produce MSF files
    /// that are not usable by most PDB tools.
    pub max_streams: u32,
}

/// The maximum number of streams that PDB can tolerate.
const DEFAULT_MAX_STREAMS: u32 = 0xfffe;

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            page_size: DEFAULT_PAGE_SIZE,
            max_streams: DEFAULT_MAX_STREAMS,
        }
    }
}

impl Msf<RandomAccessFile> {
    /// Opens an MSF file for read access, given a file name.
    pub fn open(file_name: &Path) -> anyhow::Result<Self> {
        let file = File::open(file_name)?;
        let random_file = RandomAccessFile::from(file);
        Self::new_with_access_mode(random_file, AccessMode::Read)
    }

    /// Creates a new MSF file on disk (**truncating any existing file!**) and creates a new
    /// [`Msf`] object in-memory object with read/write access.
    ///
    /// This function does not write anything to disk until stream data is written or
    /// [`Self::commit`] is called.
    pub fn create(file_name: &Path, options: CreateOptions) -> anyhow::Result<Self> {
        let file = File::create(file_name)?;
        let random_file = RandomAccessFile::from(file);
        Self::create_with_file(random_file, options)
    }

    /// Opens an existing MSF file for read/write access, given a file name.
    pub fn modify(file_name: &Path) -> anyhow::Result<Self> {
        let file = File::options().read(true).write(true).open(file_name)?;
        let random_file = RandomAccessFile::from(file);
        Self::modify_with_file(random_file)
    }
}

impl<F: ReadAt> Msf<F> {
    /// Opens an MSF file for read access, given a [`File`] that has already been opened.
    pub fn open_with_file(file: F) -> anyhow::Result<Self> {
        Self::new_with_access_mode(file, AccessMode::Read)
    }

    /// Creates a new MSF file, given a file handle that has already been opened.
    ///
    /// **This function destroys the contents of the existing file.**
    pub fn create_with_file(file: F, options: CreateOptions) -> anyhow::Result<Self> {
        Self::create_for(file, options)
    }

    /// Opens an existing MSF file for read/write access, given an [`File`] that has already
    /// been opened.
    ///
    /// The `file` handle will be used for absolute reads and writes. The caller should never use
    /// this same file handle for reads (and especially not for writes) while also using [`Msf`]
    /// because the operating system's read/write file position may be updated by [`Msf`].
    pub fn modify_with_file(file: F) -> anyhow::Result<Self> {
        Self::new_with_access_mode(file, AccessMode::ReadWrite)
    }

    /// Reads the header of a PDB file and provides access to the streams contained within the
    /// PDB file.
    ///
    /// This function reads the MSF File Header, which is the header for the entire file.
    /// It also reads the stream directory, so it knows how to find each of the streams
    /// and the pages of the streams.
    fn new_with_access_mode(file: F, access_mode: AccessMode) -> anyhow::Result<Self> {
        // Read the MSF File Header.

        let _span = trace_span!("Msf::new_with_access_mode").entered();

        const MIN_PAGE_SIZE_USIZE: usize = 1usize << MIN_PAGE_SIZE.exponent();

        let mut page0: [u8; MIN_PAGE_SIZE_USIZE] = [0; MIN_PAGE_SIZE_USIZE];

        // If this read fails, then the file is too small to be a valid PDB of any kind.
        file.read_exact_at(&mut page0, 0)?;

        let msf_kind: MsfKind;
        let page_size: u32;
        let active_fpm: u32;
        let num_pages: u32;
        let stream_dir_size: u32;

        if page0.starts_with(&MSF_BIG_MAGIC) {
            // unwrap() cannot fail because page0 has a fixed size that is larger than MsfHeader
            let (msf_header, _) = MsfHeader::ref_from_prefix(page0.as_slice()).unwrap();
            page_size = msf_header.page_size.get();
            active_fpm = msf_header.active_fpm.get();
            num_pages = msf_header.num_pages.get();
            stream_dir_size = msf_header.stream_dir_size.get();
            msf_kind = MsfKind::Big;

            // The active FPM can only be 1 or 2.
            if !matches!(active_fpm, 1 | 2) {
                bail!("The PDB header is invalid.  The active FPM is invalid.");
            }
        } else if page0.starts_with(&MSF_SMALL_MAGIC) {
            // Found an "old" MSF header.
            // unwrap() cannot fail because page0 has a fixed size that is larger than SmallMsfHeader
            let (msf_header, _) = SmallMsfHeader::ref_from_prefix(page0.as_slice()).unwrap();
            page_size = msf_header.page_size.get();
            active_fpm = msf_header.active_fpm.get() as u32;
            num_pages = msf_header.num_pages.get() as u32;
            stream_dir_size = msf_header.stream_dir_size.get();
            msf_kind = MsfKind::Small;
        } else if page0[16..24] == *b"PDB v1.0" {
            bail!("This file is a Portable PDB, which is not supported.");
        } else {
            bail!("PDB file does not have the correct header (magic is wrong).");
        }

        let Ok(page_size_pow2) = PageSize::try_from(page_size) else {
            bail!("The PDB header is invalid. The page size ({page_size}) is not a power of 2.",);
        };

        if num_pages == 0 {
            bail!("PDB specifies invalid value for num_pages (zero).");
        }

        let mut stream_sizes: Vec<u32>;

        // The number of pages in the stream directory.
        let stream_dir_num_pages = stream_dir_size.div_round_up(page_size_pow2);

        // Create the PageAllocator. This initializes the fpm vector to "everything is free"
        // and then sets Page 0 and the FPM pages as "free". Nothing is marked as "freed".
        let mut page_allocator = PageAllocator::new(num_pages as usize, page_size_pow2);

        let mut committed_stream_pages: Vec<Page>;
        let mut committed_stream_page_starts: Vec<u32>;

        match msf_kind {
            MsfKind::Big => {
                // "Big MSF" uses a 3-level hierarchy for the Stream Directory:
                //
                // stream_dir_map        <-- contains u32 pages to ↓
                // stream_dir_pages      <-- contains u32 pages to ↓
                // stream_dir_bytes      <-- bottom level, stored in pages
                //
                // stream_dir_map is an array of u32 page pointers. It is stored directly in
                // page 0, immediately after MsfHeader. These pointers point to pages that contain
                // the stream_dir_pages, which is the next level down.
                // The number of pages allocated to stream_dir_map = ceil(stream_dir_pages.len() * 4 / page_size).
                // The number of bytes used within stream_dir_map = stream_dir_pages.len() * 4.
                //
                // stream_dir_pages is a set of pages. When concatenated, they contain the page
                // pointers that point to the stream directory bytes.
                // The number of pages in stream_dir_pages = ceil(stream_dir_size / page_size).
                // The number of bytes used within stream_dir_pages is stream_dir_pages * 4.

                if stream_dir_size % 4 != 0 {
                    bail!("MSF Stream Directory has an invalid size; it is not a multiple of 4.");
                }

                // We are going to read the stream directory into this vector.
                let mut stream_dir: Vec<u32> = vec![0; stream_dir_size as usize / 4];

                // Read the page map for the stream directory.
                let stream_dir_l1_num_pages =
                    num_pages_for_stream_size(4 * stream_dir_num_pages, page_size_pow2) as usize;
                let Ok((page_map_l1_ptrs, _)) = <[U32<LE>]>::ref_from_prefix_with_elems(
                    &page0[STREAM_DIR_PAGE_MAP_FILE_OFFSET as usize..],
                    stream_dir_l1_num_pages,
                ) else {
                    bail!("Stream dir size is invalid (exceeds design limits)");
                };

                let stream_dir_bytes: &mut [u8] = stream_dir.as_mut_bytes();
                let mut stream_dir_chunks = stream_dir_bytes.chunks_mut(page_size as usize);
                // Now read the stream pages for the stream dir.
                let mut l1_page: Vec<u8> = vec![0; page_size as usize];
                'l1_loop: for &page_map_l1_ptr in page_map_l1_ptrs.iter() {
                    let page_map_l1_ptr: u32 = page_map_l1_ptr.get();

                    page_allocator.init_mark_stream_dir_page_busy(page_map_l1_ptr)?;
                    if is_special_page_big_msf(page_size_pow2, page_map_l1_ptr) {
                        bail!(
                            "Stream dir contains invalid page number: {page_map_l1_ptr}. \
                             Page points to Page 0 or to an FPM page."
                        );
                    }

                    // Read the page pointers.
                    let file_offset = page_to_offset(page_map_l1_ptr, page_size_pow2);
                    file.read_exact_at(l1_page.as_mut_slice(), file_offset)?;

                    // Now read the individual pages, as long as we have more.
                    let l2_page_u32 = <[U32<LE>]>::ref_from_bytes(l1_page.as_slice()).unwrap();

                    for &l2_page in l2_page_u32.iter() {
                        let l2_page: u32 = l2_page.get();

                        let Some(stream_dir_chunk) = stream_dir_chunks.next() else {
                            break 'l1_loop;
                        };

                        page_allocator.init_mark_stream_dir_page_busy(l2_page)?;
                        if is_special_page_big_msf(page_size_pow2, l2_page) {
                            bail!(
                                "Stream dir contains invalid page number: {l2_page}. \
                                 Page points to Page 0 or to an FPM page."
                            );
                        }

                        let l2_file_offset = page_to_offset(l2_page, page_size_pow2);
                        file.read_exact_at(stream_dir_chunk, l2_file_offset)?;
                    }
                }

                if stream_dir.is_empty() {
                    bail!("Stream directory is invalid (zero-length)");
                }

                // Bulk-convert the stream directory to host endian, if necessary.
                if !cfg!(target_endian = "little") {
                    for x in stream_dir.iter_mut() {
                        *x = u32::from_le(*x);
                    }
                }

                let num_streams = stream_dir[0] as usize;

                // Stream 0 is special and must exist.
                if num_streams == 0 {
                    bail!("MSF file is invalid, because num_streams = 0.");
                }

                let Some(stream_sizes_src) = stream_dir.get(1..1 + num_streams) else {
                    bail!("Stream directory is invalid (num_streams is not consistent with size)");
                };
                stream_sizes = stream_sizes_src.to_vec();

                let mut stream_pages_iter = &stream_dir[1 + num_streams..];

                // Build committed_stream_pages and committed_stream_page_starts.
                committed_stream_pages = Vec::with_capacity(stream_dir.len() - num_streams - 1);
                committed_stream_page_starts = Vec::with_capacity(num_streams + 1);

                for (stream, &stream_size) in stream_sizes_src.iter().enumerate() {
                    committed_stream_page_starts.push(committed_stream_pages.len() as u32);

                    if stream_size != NIL_STREAM_SIZE {
                        let num_stream_pages =
                            num_pages_for_stream_size(stream_size, page_size_pow2) as usize;
                        if num_stream_pages > stream_pages_iter.len() {
                            bail!(
                                "Stream directory is invalid.  Stream {stream} has size {stream_size}, \
                                 which exceeds the size of the stream directory."
                            );
                        }
                        let (this_stream_pages, next) =
                            stream_pages_iter.split_at(num_stream_pages);
                        stream_pages_iter = next;
                        committed_stream_pages.extend_from_slice(this_stream_pages);
                    }
                }
                committed_stream_page_starts.push(committed_stream_pages.len() as u32);

                // Now that we have finished reading the stream directory, we set the length
                // of stream 0 (the "Old Stream Directory") to 0. Nothing should ever read Stream 0.
                // If we modify a PDB/MSF file, then we want to write no pages at all for Stream 0.
                // Doing this here is the most convenient way to handle this.
                stream_sizes[0] = 0;
            }

            MsfKind::Small => {
                // Before Big MSF files, the stream directory was stored in a set of pages.
                // These pages were listed directly within page 0. Keep in mind that page numbers
                // are 16-bit in old MSF files.
                let page_pointers_size_bytes = stream_dir_num_pages * 2;

                let mut pages_u16: Vec<U16<LE>> = vec![U16::new(0); stream_dir_num_pages as usize];
                if page_pointers_size_bytes + size_of::<SmallMsfHeader>() as u32 > page_size {
                    bail!(
                        "The MSF header is invalid. The page pointers for the stream directory \
                         exceed the range of the first page. \
                         Stream dir size (in bytes): {stream_dir_size}  Page size: {page_size}"
                    );
                }

                file.read_exact_at(pages_u16.as_mut_bytes(), size_of::<SmallMsfHeader>() as u64)?;

                // Read the pages of the stream directory. Be careful with the last page.
                let mut page_iter = pages_u16.iter();
                let mut old_stream_dir_bytes: Vec<u8> = vec![0; stream_dir_size as usize];
                for stream_dir_chunk in old_stream_dir_bytes.chunks_mut(page_size as usize) {
                    // This unwrap should succeed because we computed the length of pages_u16
                    // based on the byte size of the stream directory.
                    let page = page_iter.next().unwrap().get() as u32;
                    page_allocator.init_mark_stream_dir_page_busy(page)?;
                    file.read_exact_at(stream_dir_chunk, page_to_offset(page, page_size_pow2))?;
                }

                let Ok((header, rest)) =
                    OldMsfStreamDirHeader::read_from_prefix(old_stream_dir_bytes.as_slice())
                else {
                    bail!("Invalid stream directory: too small");
                };

                let num_streams = header.num_streams.get() as usize;
                stream_sizes = Vec::with_capacity(num_streams);

                let Ok((entries, mut rest)) =
                    <[OldMsfStreamEntry]>::ref_from_prefix_with_elems(rest, num_streams)
                else {
                    bail!("Invalid stream directory: too small")
                };

                for entry in entries.iter() {
                    let stream_size = entry.stream_size.get();
                    stream_sizes.push(stream_size);
                }

                committed_stream_page_starts = Vec::with_capacity(num_streams + 1);
                committed_stream_pages = Vec::new(); // TODO: precompute capacity

                for &stream_size in stream_sizes.iter() {
                    committed_stream_page_starts.push(committed_stream_pages.len() as u32);
                    if stream_size != NIL_STREAM_SIZE {
                        let num_pages = stream_size.div_round_up(page_size_pow2);

                        let Ok((pages, r)) =
                            <[U16<LE>]>::ref_from_prefix_with_elems(rest, num_pages as usize)
                        else {
                            bail!("Invalid stream directory: too small");
                        };

                        rest = r; // update iterator state
                        for page in pages.iter() {
                            committed_stream_pages.push(page.get() as u32);
                        }
                    }
                }

                committed_stream_page_starts.push(committed_stream_pages.len() as u32);

                if !rest.is_empty() {
                    warn!(
                        unused_bytes = rest.len(),
                        "old-style stream dir contained unused bytes"
                    );
                }
            }
        }

        // Mark the pages in all streams (except for stream 0) as busy. This will also detect
        // page numbers that are invalid (0 or FPM).
        {
            // pages is the list of the page numbers for all streams (except stream 0).
            let start = committed_stream_page_starts[1] as usize;
            let pages = &committed_stream_pages[start..];
            for &page in pages.iter() {
                page_allocator.init_mark_stream_page_busy(page, 0, 0)?;
            }
        }

        // We have finished building the in-memory FPM, including both the fpm and fpm_freed
        // vectors. We expect that every page is either FREE, BUSY, or DELETED. Check that now.
        page_allocator.check_vector_consistency()?;

        // Read the FPM from disk and compare it to the FPM that we just constructed. They should
        // be identical.
        // TODO: implement for small MSF
        let fpm_on_disk = read_fpm_big_msf(&file, active_fpm, num_pages, page_size_pow2)?;

        assert_eq!(fpm_on_disk.len(), page_allocator.fpm.len()); // because num_pages defines both

        if page_allocator.fpm != fpm_on_disk {
            warn!("FPM computed from Stream Directory is not equal to FPM found on disk.");
            warn!(
                "Num pages = {num_pages} (0x{num_pages:x} bytes, bit offset: 0x{:x}:{})",
                num_pages / 8,
                num_pages % 8
            );

            for i in 0..num_pages as usize {
                if fpm_on_disk[i] != page_allocator.fpm[i] {
                    warn!(
                        "  bit 0x{:04x} is different. disk = {}, computed = {}",
                        i, fpm_on_disk[i], page_allocator.fpm[i]
                    );
                }
            }

            // Clang's PDB writer sometimes places stream pages at illegal locations,
            // such as in the pages reserved for the FPM. We tolerate this for reading
            // but not for writing.
            if access_mode == AccessMode::ReadWrite {
                bail!("FPM is corrupted; FPM computed from Stream Directory is not equal to FPM found on disk.");
            }
        }

        // We have finished checking all the data that we have read from disk.
        // Now check the consistency of our in-memory data structures.
        page_allocator.assert_invariants();

        match (access_mode, msf_kind) {
            (AccessMode::ReadWrite, MsfKind::Small) => {
                bail!(
                    "This PDB file uses the obsolete 'Small MSF' encoding. \
                     This library does not support read-write mode with Small MSF files."
                );
            }

            (AccessMode::ReadWrite, MsfKind::Big) => {}

            (AccessMode::Read, _) => {}
        }

        Ok(Self {
            file,
            access_mode,
            active_fpm,
            committed_stream_pages,
            committed_stream_page_starts,
            stream_sizes,
            kind: msf_kind,
            pages: page_allocator,
            modified_streams: HashMap::new(),
            max_streams: DEFAULT_MAX_STREAMS,
        })
    }

    /// Creates a new MSF object in memory. The on-disk file is not modified until `commit()` is
    /// called.
    pub fn create_for(file: F, options: CreateOptions) -> anyhow::Result<Self> {
        let _span = trace_span!("Msf::create_for").entered();

        assert!(options.page_size >= MIN_PAGE_SIZE);
        assert!(options.page_size <= MAX_PAGE_SIZE);

        let num_pages: usize = 3;

        let mut this = Self {
            file,
            access_mode: AccessMode::ReadWrite,
            committed_stream_pages: vec![],
            committed_stream_page_starts: vec![0; 2],
            kind: MsfKind::Big,
            pages: PageAllocator::new(num_pages, options.page_size),
            modified_streams: HashMap::new(),
            stream_sizes: vec![0],
            active_fpm: 2,
            max_streams: options.max_streams,
        };

        // Set up the 4 fixed-index streams. They are created as nil streams.
        for _ in 1..=4 {
            let _stream_index = this.nil_stream()?;
        }

        Ok(this)
    }
}

/// Read each page of the FPM. Each page of the FPM is stored in a different interval;
/// they are not contiguous.
///
/// num_pages is the total number of pages in the FPM.
fn read_fpm_big_msf<F: ReadAt>(
    file: &F,
    active_fpm: u32,
    num_pages: u32,
    page_size: PageSize,
) -> anyhow::Result<BitVec<u32, Lsb0>> {
    let _span = trace_span!("read_fpm_big_msf").entered();

    assert!(num_pages > 0);

    let mut free_page_map: BitVec<u32, Lsb0> = BitVec::new();
    free_page_map.resize(num_pages as usize, false);
    let fpm_bytes: &mut [u8] = free_page_map.as_raw_mut_slice().as_mut_bytes();
    let page_size_usize = usize::from(page_size);

    for (interval, fpm_page_bytes) in fpm_bytes.chunks_mut(page_size_usize).enumerate() {
        let interval_page = interval_to_page(interval as u32, page_size);
        let file_pos = page_to_offset(interval_page + active_fpm, page_size);

        trace!(
            interval,
            interval_page,
            file_pos,
            "reading FPM page, interval_page = 0x{interval_page:x}, file_pos = 0x{file_pos:x}"
        );
        file.read_exact_at(fpm_page_bytes, file_pos)?;
    }

    // Check our invariants for the FPM. If these checks fail then we return Err because we
    // are validating data that we read from disk. After these checks succeed, we switch to using
    // assert_invariants(), which uses assert!(). That verifies that we preserve our invariants.

    // Check that page 0, which stores the MSF File Header, is busy.
    if free_page_map[0] {
        bail!("FPM is invalid: Page 0 should always be BUSY");
    }

    // Check that the pages assigned to the FPM are marked "busy" in all intervals.

    let mut interval: u32 = 0;
    loop {
        let interval_page = interval_to_page(interval, page_size) as usize;
        let fpm1_index = interval_page + 1;
        let fpm2_index = interval_page + 2;

        if fpm1_index < free_page_map.len() {
            if free_page_map[fpm1_index] {
                bail!("All FPM pages should be marked BUSY");
            }
        }

        if fpm2_index < free_page_map.len() {
            if free_page_map[fpm2_index] {
                bail!("All FPM pages should be marked BUSY");
            }
            interval += 1;
        } else {
            break;
        }
    }

    Ok(free_page_map)
}

/// Computes the low-bits-on mask for the page mask.
fn low_page_mask(page_size: PageSize) -> u32 {
    (1u32 << page_size.exponent()).wrapping_sub(1u32)
}

/// Tests whether `page` contributes to either FPM1 or FPM2.
fn is_fpm_page_big_msf(page_size: PageSize, page: u32) -> bool {
    let page_within_interval = page & low_page_mask(page_size);
    matches!(page_within_interval, 1 | 2)
}

/// Tests whether `page` is one of the special pages (Page 0, FPM1, or FPM2)
fn is_special_page_big_msf(page_size: PageSize, page: u32) -> bool {
    page == 0 || is_fpm_page_big_msf(page_size, page)
}

/// Describes the "old" MSF Stream Directory Header.
#[derive(Clone, IntoBytes, FromBytes, Unaligned, KnownLayout, Immutable)]
#[repr(C)]
struct OldMsfStreamDirHeader {
    num_streams: U16<LE>,
    ignored: U16<LE>,
}

/// An entry in the "old" MSF Stream Directory.
#[derive(Clone, IntoBytes, FromBytes, Unaligned, KnownLayout, Immutable)]
#[repr(C)]
struct OldMsfStreamEntry {
    stream_size: U32<LE>,
    ignored: U32<LE>,
}
