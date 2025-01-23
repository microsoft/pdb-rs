//! Reads and writes Multi-Stream Files (MSF). MSF is the underlying container format used by
//! Program Database (PDB) files.
//!
//! MSF files contain a set of numbered _streams_. Each stream is like a file; a stream is a
//! sequence of bytes.
//!
//! The bytes stored within a single stream are usually not stored sequentially on disk. The
//! organization of the disk file and the mapping from stream locations to MSF file locations is
//! similar to a traditional file system; managing that mapping is the main purpose of the MSF
//! file format.
//!
//! MSF files are used as the container format for Program Database (PDB) files. PDB files are used
//! by compilers, debuggers, and other tools when targeting Windows.
//!
//! Most developers should not use this crate directly. This crate is a building block for tools
//! that read and write PDBs. This crate does not provide any means for building or parsing the
//! data structures of PDB files; it only handles storing files in the MSF container format.
//!
//! The `mspdb` crate uses this crate for reading and writing PDB files. It provides an interface
//! for reading PDB data structures, and in some cases for creating or modifying them. Most
//! developers should use `mspdb` instead of using `msf` directly.
//!
//! # References
//! * <https://llvm.org/docs/PDB/index.html>
//! * <https://llvm.org/docs/PDB/MsfFile.html>
//! * <https://github.com/microsoft/microsoft-pdb>

#![forbid(unused_must_use)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::needless_late_init)]
#![allow(clippy::needless_lifetimes)]

mod check;
mod commit;
mod open;
mod pages;
mod read;
mod stream_reader;
mod stream_writer;
mod write;

#[cfg(test)]
mod tests;

pub use open::CreateOptions;
pub use stream_reader::StreamReader;
pub use stream_writer::StreamWriter;

use anyhow::bail;
use bitvec::prelude::{BitVec, Lsb0};
use pow2::{IntOnlyPow2, Pow2};
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem::size_of;
use std::path::Path;
use sync_file::{RandomAccessFile, ReadAt, WriteAt};
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U16, U32};

use self::pages::{num_pages_for_stream_size, PageAllocator};

/// Identifies a page number in the MSF file. Not to be confused with `StreamPage`.
type Page = u32;

/// Identifies a page within a stream. `StreamPage` can be translated to `Page` by using the
/// stream page mapper.
type StreamPage = u32;

const FPM_NUMBER_1: u32 = 1;
const FPM_NUMBER_2: u32 = 2;

/// The value of `magic` for "big" MSF files.
const MSF_BIG_MAGIC: [u8; 32] = *b"Microsoft C/C++ MSF 7.00\r\n\x1a\x44\x53\x00\x00\x00";

/// This identifies MSF files before the transition to "big" MSF files.
const MSF_SMALL_MAGIC: [u8; 0x2c] = *b"Microsoft C/C++ program database 2.00\r\n\x1a\x4a\x47\0\0";

#[test]
fn show_magics() {
    use pretty_hex::PrettyHex;

    println!("MSF_SMALL_MAGIC:");
    println!("{:?}", MSF_SMALL_MAGIC.hex_dump());

    println!("MSF_BIG_MAGIC:");
    println!("{:?}", MSF_BIG_MAGIC.hex_dump());
}

/// The header of the PDB/MSF file, before the transition to "big" MSF files.
/// This is at file offset 0.
#[allow(missing_docs)]
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
struct SmallMsfHeader {
    /// Identifies this file as a PDB. Value must be [`MSF_SMALL_MAGIC`].
    magic: [u8; 0x2c],
    page_size: U32<LE>,
    active_fpm: U16<LE>,
    num_pages: U16<LE>,
    stream_dir_size: U32<LE>,
    /// This field contains a pointer to an in-memory data structure, and hence is meaningless.
    /// Decoders should ignore this field. Encoders should set this field to 0.
    stream_dir_ptr: U32<LE>,
    // mpspnpm: [U32<LE>]
}

/// The header of the PDB/MSF file. This is at file offset 0.
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
struct MsfHeader {
    /// Identifies this file as a PDB.
    magic: [u8; 32],

    /// The size of each page, in bytes.
    page_size: U32<LE>,

    /// Page number of the active FPM. This can only be 1 or 2. In the C++ implementation, this is
    /// `pnFpm`.
    active_fpm: U32<LE>,

    /// The number of pages in this MSF file. In the C++ implementation, this is `pnMac`.
    num_pages: U32<LE>,

    /// Size of the Stream Directory, in bytes. In the C++ implementation, this is `siSt.cb`.
    stream_dir_size: U32<LE>,

    /// The page which contains the Stream Directory Map. This page contains a list of pages
    /// which contain the Stream Directory.
    ///
    /// This field is only used for "Small MSF" (pre-"Big MSF") encoding. When using Big MSF,
    /// this field is expected to be zero.
    ///
    /// In the C++ implementation, this is `mpspnpn` (map of stream page number to page number).
    stream_dir_small_page_map: U32<LE>,
    // When using "Big MSF", there is an array of u32 values that immediately follow
    // the MSfHeader. The size of the array is a function of stream_dir_size and num_pages:
    //
    //     divide_round_up(divide_round_up(stream_dir_size, num_pages) * 4), num_pages)
    //
    // pub stream_dir_big_page_map: [U32<LE>],
}

/// The length of the MSF File Header.
const MSF_HEADER_LEN: usize = size_of::<MsfHeader>();

/// The byte offset of the stream directory page map. This is a small array of page indices that
/// point to pages that contain the stream directory. This is used only with the Big MSF encoding.
const STREAM_DIR_PAGE_MAP_FILE_OFFSET: u64 = MSF_HEADER_LEN as u64;
static_assertions::const_assert_eq!(MSF_HEADER_LEN, 52);

/// The minimum page size.
pub const MIN_PAGE_SIZE: PageSize = PageSize::from_exponent(9);

/// The default page size.
pub const DEFAULT_PAGE_SIZE: PageSize = PageSize::from_exponent(12);

/// A large page size. This is less than the largest supported page size.
pub const LARGE_PAGE_SIZE: PageSize = PageSize::from_exponent(13);

/// The largest supported page size.
pub const MAX_PAGE_SIZE: PageSize = PageSize::from_exponent(16);

/// This size is used to mark a stream as "invalid". An invalid stream is different from a
/// stream with a length of zero bytes.
pub const NIL_STREAM_SIZE: u32 = 0xffff_ffff;

/// Specifies a page size used in an MSF file. This value is always a power of 2.
pub type PageSize = Pow2;

/// The stream index of the Stream Directory stream. This is reserved and cannot be used by
/// applications.
pub const STREAM_DIR_STREAM: u32 = 0;

/// Converts a page number to a file offset.
fn page_to_offset(page: u32, page_size: PageSize) -> u64 {
    (page as u64) << page_size.exponent()
}

/// Given an interval number, returns the page number of the first page of the interval.
fn interval_to_page(interval: u32, page_size: PageSize) -> u32 {
    interval << page_size.exponent()
}

/// Gets the byte offset within a page, for a given offset within a stream.
pub fn offset_within_page(offset: u32, page_size: PageSize) -> u32 {
    let page_low_mask = (1u32 << page_size.exponent()) - 1u32;
    offset & page_low_mask
}

/// Allows reading and writing the contents of a PDB/MSF file.
///
/// The [`Msf::open`] function opens an MSF file for read access, given a file. This is the most
/// commonly-used way to open a file.
pub struct Msf<F = RandomAccessFile> {
    /// The data source.
    file: F,

    kind: MsfKind,

    /// The FPM number for the committed (active) FPM.
    ///
    /// The `commit()` function can change this number.
    active_fpm: u32,

    /// Contains the sizes of all streams. The length of `stream_sizes` implicitly defines
    /// the number of streams.
    ///
    /// Values in this vector may be [`NIL_STREAM_SIZE`], indicating that the stream is present
    /// but is a nil stream.
    ///
    /// As streams are modified, this vector changes. It contains a combination of both committed
    /// and uncommitted state.
    stream_sizes: Vec<u32>,

    /// The maximum number of streams that we will allow to be created using `new_stream` or
    /// `nil_stream`. The default value is 0xfffe, which prevents overflowing the 16-bit stream
    /// indexes that are used in PDB (or confusing them with the "nil" stream index).
    max_streams: u32,

    /// Contains the page numbers for all streams in the committed state.
    committed_stream_pages: Vec<Page>,

    /// Vector contains offsets into `committed_stream_pages` where the pages for a given stream start.
    committed_stream_page_starts: Vec<u32>,

    /// Handles allocating pages.
    pages: PageAllocator,

    /// If a stream has been modified then there is an entry in this table for it. The key for
    /// each entry is the stream number. The value is the sequence of pages for that stream.
    ///
    /// One of the side-effects of the [`Msf::commit`] function is that the `modified_streams`
    /// table is cleared.
    ///
    /// This table is always empty if `access_mode == AccessMode::Read`.
    modified_streams: HashMap<u32, Vec<Page>>,

    access_mode: AccessMode,
}

/// Specifies the versions used for the MSF.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum MsfKind {
    /// The obsolete, pre-Big MSF encoding
    Small,
    /// The fancy new modern Big MSF encoding
    Big,
}

/// Specifies the access mode for opening a PDB/MSF file.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum AccessMode {
    /// Read-only access
    Read,
    /// Read-write access
    ReadWrite,
}

impl<F> Msf<F> {
    /// Returns the page size used for this file.
    pub fn page_size(&self) -> PageSize {
        self.pages.page_size
    }

    /// Gets access to the stream page pointers for a given stream. The stream page pointers
    /// provide the mapping from offsets within a stream to offsets within the entire PDB (MSF) file.
    ///
    /// If the stream is a NIL stream, then this returns `(NIL_STREAM_SIZE, &[])`.
    pub fn stream_size_and_pages(&self, stream: u32) -> Result<(u32, &[u32]), anyhow::Error> {
        let Some(&stream_size) = self.stream_sizes.get(stream as usize) else {
            bail!("Stream index is out of range.  Index: {stream}");
        };

        if stream_size == NIL_STREAM_SIZE {
            // This is a NIL stream.
            return Ok((NIL_STREAM_SIZE, &[]));
        }

        // The stream index is valid and the stream is not a NIL stream.
        let num_stream_pages =
            num_pages_for_stream_size(stream_size, self.pages.page_size) as usize;

        if num_stream_pages == 0 {
            // The stream is valid (is not nil) and is a zero-length stream.
            // It has no pages assigned to it.
            return Ok((0, &[]));
        }

        // If this stream has been modified, then return the modified page list.
        if let Some(pages) = self.modified_streams.get(&stream) {
            assert_eq!(num_stream_pages, pages.len());
            return Ok((stream_size, pages.as_slice()));
        }

        let start = self.committed_stream_page_starts[stream as usize] as usize;
        let pages = &self.committed_stream_pages[start..start + num_stream_pages];
        Ok((stream_size, pages))
    }

    /// The total number of streams in this PDB, including nil streams.
    pub fn num_streams(&self) -> u32 {
        self.stream_sizes.len() as u32
    }

    /// Gets the size of a given stream, in bytes.
    ///
    /// The `stream` value must be in a valid range of `0..num_streams()`.
    ///
    /// If `stream` is a NIL stream, this function returns 0.
    pub fn stream_size(&self, stream: u32) -> u32 {
        assert!((stream as usize) < self.stream_sizes.len());
        let stream_size = self.stream_sizes[stream as usize];
        if stream_size == NIL_STREAM_SIZE {
            0
        } else {
            stream_size
        }
    }

    /// Indicates whether a given stream index is valid.
    pub fn is_valid_stream_index(&self, stream: u32) -> bool {
        (stream as usize) < self.stream_sizes.len()
    }

    /// Indicates that a stream index is valid, and that its length is valid.
    pub fn is_stream_valid(&self, stream: u32) -> bool {
        if (stream as usize) < self.stream_sizes.len() {
            self.stream_sizes[stream as usize] != NIL_STREAM_SIZE
        } else {
            false
        }
    }

    /// Return the nominal length of this file, in bytes.
    ///
    /// This is the number of pages multiplied by the page size. It is not guaranteed to be equal to
    /// the on-disk size of the file, but in practice it usually is.
    pub fn nominal_size(&self) -> u64 {
        page_to_offset(self.pages.num_pages, self.pages.page_size)
    }

    /// Returns the number of free pages.
    ///
    /// This number counts the pages that are _less than_ `num_pages`. There may be pages assigned
    /// to the MSF file beyond `num_pages`, but if there are then this does not count that space.
    ///
    /// This value does not count Page 0, pages assigned to the FPM, streams, or the current
    /// Stream Directory. It does count pages assigned to the old stream directory.
    pub fn num_free_pages(&self) -> u32 {
        self.pages.fpm.count_ones() as u32
    }

    /// Extracts the underlying file for this MSF. **All pending modifications are dropped**.
    pub fn into_file(self) -> F {
        self.file
    }

    /// Gets access to the contained file
    pub fn file(&self) -> &F {
        &self.file
    }

    /// Gets mutable access to the contained file
    pub fn file_mut(&mut self) -> &mut F {
        &mut self.file
    }

    /// Indicates whether this [`Msf`] was opened for read/write access.
    pub fn is_writable(&self) -> bool {
        self.access_mode == AccessMode::ReadWrite
    }
}

impl<F: ReadAt> Msf<F> {
    /// Reads a portion of a stream to a vector.
    pub fn read_stream_section_to_vec(
        &self,
        stream: u32,
        start: u32,
        size: u32,
    ) -> anyhow::Result<Vec<u8>>
    where
        F: ReadAt,
    {
        let mut reader = self.get_stream_reader(stream)?;
        let mut buffer: Vec<u8> = vec![0; size as usize];
        reader.seek(SeekFrom::Start(start as u64))?;
        reader.read_exact(&mut buffer)?;
        Ok(buffer)
    }

    /// Reads an entire stream to a vector.
    pub fn read_stream_to_vec(&self, stream: u32) -> anyhow::Result<Vec<u8>> {
        let mut stream_data = Vec::new();
        self.read_stream_to_vec_mut(stream, &mut stream_data)?;
        Ok(stream_data)
    }

    /// Reads an entire stream into an existing vector.
    #[inline(never)]
    pub fn read_stream_to_vec_mut(
        &self,
        stream: u32,
        stream_data: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        let mut reader = self.get_stream_reader(stream)?;

        let stream_len = reader.len() as usize;

        // Do not clear and resize. Doing so requires zeroing all the data in the vector.
        // Since we are going to read into the vector, that means we would modify every byte twice.
        // That's expensive when you're working with a lot of data.
        stream_data.resize(stream_len, 0);

        let mut total_bytes_read: usize = 0;

        while total_bytes_read < stream_len {
            let bytes_requested = stream_len - total_bytes_read;
            let bytes_read = reader
                .read(&mut stream_data[total_bytes_read..total_bytes_read + bytes_requested])?;
            if bytes_read == 0 {
                break;
            }
            total_bytes_read += bytes_read;
        }

        stream_data.truncate(total_bytes_read);
        Ok(())
    }

    /// Returns an object which can read from a given stream.  The returned object implements
    /// the [`Read`], [`Seek`], and [`ReadAt`] traits.
    pub fn get_stream_reader(&self, stream: u32) -> anyhow::Result<StreamReader<'_, F>>
    where
        F: ReadAt,
    {
        let (stream_size, stream_pages) = self.stream_size_and_pages(stream)?;
        Ok(StreamReader::new(self, stream_size, stream_pages, 0))
    }
}

/// Checks whether the header of a file appears to be a valid MSF file.
///
/// This only looks at the signature; it does not read anything else in the file. This is useful
/// for quickly determining whether a file could be an MSF file, but without any validation.
pub fn is_file_header_msf(header: &[u8]) -> bool {
    header.starts_with(&MSF_BIG_MAGIC) || header.starts_with(&MSF_SMALL_MAGIC)
}

/// The absolute minimum size of a slice that could contain a valid MSF file header, as tested by
/// [`is_file_header_msf`].
///
/// This does not specify the minimum valid size of an MSF file. It is only a recommended minimum
/// for callers of [`is_file_header_msf`].
pub const MIN_FILE_HEADER_SIZE: usize = 0x100;
