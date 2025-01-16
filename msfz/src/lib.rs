//! Multi-Stream File - Compressed
//!
//! This crate allows reading and writing PDZ/MSFZ files. PDZ/MSFZ files are similar to PDB/MSF
//! files. They contain a set of streams, which are indexed by number. Each stream is a sequence
//! of bytes, similar to an ordinary file.
//!
//! See the [`docs`] module for a description of the MSFZ file format.

#![forbid(unsafe_code)]
#![forbid(unused_must_use)]
#![warn(missing_docs)]

#[cfg(doc)]
pub mod docs {
    #![doc = include_str!("docs.md")]
    use super::*;
}

use anyhow::Result;
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U32, U64};

mod compress_utils;
mod reader;
#[cfg(test)]
mod tests;
mod writer;

pub use reader::*;
pub use writer::*;

/// Describes the header at the start of the MSFZ file.
///
/// This describes the on-disk layout of the file header. It is stored at the beginning of the
/// MSFZ file.
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
pub struct MsfzFileHeader {
    /// Identifies this as an MSFZ file. The value must always be [`MSFZ_FILE_SIGNATURE`].
    pub signature: [u8; 32],

    /// Specifies the version of the MSFZ file layout.
    pub version: U64<LE>,

    /// The file offset of the stream directory.
    pub stream_dir_offset: U64<LE>,

    /// The file offset of the Chunk Table, which has type `[ChunkEntry; num_chunks]`.
    pub chunk_table_offset: U64<LE>,

    /// The number of streams stored within this MSFZ file.
    pub num_streams: U32<LE>,

    /// The compression algorithm applied to the stream directory.
    pub stream_dir_compression: U32<LE>,

    /// The size in bytes of the stream directory, compressed (on-disk).
    pub stream_dir_size_compressed: U32<LE>,

    /// The size in bytes of the stream directory after decompression (in-memory).
    pub stream_dir_size_uncompressed: U32<LE>,

    /// The number of compression chunks.
    pub num_chunks: U32<LE>,

    /// The size in bytes of the Chunk Table.
    pub chunk_table_size: U32<LE>,
}

/// Describes one compressed chunk.
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
pub struct ChunkEntry {
    /// File offset (within the MSFZ file) the compressed chunk.
    pub file_offset: U64<LE>,

    /// The compression algorithm for this chunk.
    pub compression: U32<LE>,

    /// Size in bytes of the compressed data on disk.
    ///
    /// This value should be non-zero.
    pub compressed_size: U32<LE>,

    /// Number of bytes after decompression; this is the in-memory size.
    ///
    /// This value should be non-zero.
    pub uncompressed_size: U32<LE>,
}

/// The special value used for stream size to indicate a nil stream.
pub const NIL_STREAM_SIZE: u32 = 0xffff_ffff;

/// Indicates that no compression is used.
pub const COMPRESSION_NONE: u32 = 0;

/// Identifies the [`Zstd`](https://github.com/facebook/zstd) compression algorithm.
pub const COMPRESSION_ZSTD: u32 = 1;

/// Identifies the [`Deflate`](https://en.wikipedia.org/wiki/Deflate) compression algorithm.
///
/// This uses the "raw" Deflate stream. It _does not_ use the GZIP encapsulation header.
pub const COMPRESSION_DEFLATE: u32 = 2;

/// Specifies the compression algorithms that are supported by this library.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[non_exhaustive]
pub enum Compression {
    /// Identifies the [`Zstd`](https://github.com/facebook/zstd) compression algorithm.
    Zstd,
    /// Identifies the [`Deflate`](https://en.wikipedia.org/wiki/Deflate) compression algorithm.
    Deflate,
}

impl Compression {
    fn to_code(self) -> u32 {
        match self {
            Self::Zstd => COMPRESSION_ZSTD,
            Self::Deflate => COMPRESSION_DEFLATE,
        }
    }

    fn try_from_code(code: u32) -> Result<Self, UnsupportedCompressionError> {
        match code {
            COMPRESSION_ZSTD => Ok(Self::Zstd),
            COMPRESSION_DEFLATE => Ok(Self::Deflate),
            _ => Err(UnsupportedCompressionError),
        }
    }

    fn try_from_code_opt(code: u32) -> Result<Option<Self>, UnsupportedCompressionError> {
        if code != COMPRESSION_NONE {
            Ok(Some(Self::try_from_code(code)?))
        } else {
            Ok(None)
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct UnsupportedCompressionError;

impl std::error::Error for UnsupportedCompressionError {}

impl std::fmt::Display for UnsupportedCompressionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "The specified compression mode is not recognized or supported."
        )
    }
}

/// The signature of a MSFZ/PDZ file.
pub const MSFZ_FILE_SIGNATURE: [u8; 32] = *b"Microsoft MSFZ Container\r\n\x1aALD\0\0";

#[test]
fn print_file_signature() {
    use dump_utils::HexDump;
    println!("\n{:?}", HexDump::new(&MSFZ_FILE_SIGNATURE));
}

/// The current version of the PDZ specification being developed.
pub const MSFZ_FILE_VERSION_V0: u64 = 0;

/// Handles packing and unpacking the `file_offset` for compressed streams.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct ChunkAndOffset {
    chunk: u32,
    offset: u32,
}

/// Checks whether the header of a file appears to be a valid MSFZ/PDZ file.
///
/// This only looks at the signature; it doens't read anything else in the file.
pub fn is_header_msfz(header: &[u8]) -> bool {
    header.starts_with(&MSFZ_FILE_SIGNATURE)
}

#[derive(Default)]
struct Stream {
    fragments: Vec<Fragment>,
}

// Describes a region within a stream.
#[derive(Clone)]
struct Fragment {
    size: u32,
    location: FragmentLocation,
}

impl std::fmt::Debug for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "size 0x{:05x} at {:?}", self.size, self.location)
    }
}

impl std::fmt::Debug for FragmentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uncompressed { file_offset } => {
                write!(f, "uncompressed at 0x{:06x}", file_offset)
            }
            Self::Compressed {
                chunk_index,
                offset_within_chunk,
            } => write!(f, "chunk {} : 0x{:04x}", chunk_index, offset_within_chunk),
        }
    }
}

const FRAGMENT_LOCATION_CHUNK_BIT: u32 = 63;
const FRAGMENT_LOCATION_CHUNK_MASK: u64 = 1 << FRAGMENT_LOCATION_CHUNK_BIT;

#[derive(Clone)]
enum FragmentLocation {
    Uncompressed {
        file_offset: u64,
    },
    Compressed {
        chunk_index: u32,
        offset_within_chunk: u32,
    },
}
