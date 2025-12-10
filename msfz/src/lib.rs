//! Multi-Stream File - Compressed
//!
//! This crate allows reading and writing PDZ/MSFZ files. PDZ/MSFZ files are similar to PDB/MSF
//! files. They contain a set of streams, which are indexed by number. Each stream is a sequence
//! of bytes, similar to an ordinary file.
//!
//! See the [`spec`] module for a description of the MSFZ file format.

#![forbid(unsafe_code)]
#![forbid(unused_must_use)]
#![warn(missing_docs)]
#![allow(clippy::needless_lifetimes)]

#[cfg(doc)]
pub mod spec {
    #![doc = include_str!("msfz.md")]
    use super::*;
}

use std::fs::OpenOptions;

use anyhow::Result;
use zerocopy::{FromBytes, FromZeros, Immutable, IntoBytes, KnownLayout, LE, U32, U64, Unaligned};

mod compress_utils;
mod reader;
mod stream_data;
#[cfg(test)]
mod tests;
mod writer;

pub use reader::*;
pub use stream_data::StreamData;
pub use writer::*;

/// Describes the header at the start of the MSFZ file.
///
/// This describes the on-disk layout of the file header. It is stored at the beginning of the
/// MSFZ file.
#[derive(IntoBytes, FromBytes, Unaligned, Immutable, KnownLayout)]
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
#[derive(IntoBytes, FromBytes, Unaligned, Immutable, KnownLayout)]
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
    use pretty_hex::PrettyHex;
    println!("\n{:?}", MSFZ_FILE_SIGNATURE.hex_dump());
}

/// The current version of the PDZ specification being developed.
pub const MSFZ_FILE_VERSION_V0: u64 = 0;

/// Checks whether the header of a file appears to be a valid MSFZ/PDZ file.
///
/// This only looks at the signature; it doens't read anything else in the file.
pub fn is_header_msfz(header: &[u8]) -> bool {
    header.starts_with(&MSFZ_FILE_SIGNATURE)
}

fn open_options_shared(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const FILE_SHARE_READ: u32 = 1;
        options.share_mode(FILE_SHARE_READ)
    }
    #[cfg(not(windows))]
    {
        options
    }
}

fn open_options_exclusive(options: &mut OpenOptions) -> &mut OpenOptions {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0)
    }
    #[cfg(not(windows))]
    {
        options
    }
}
