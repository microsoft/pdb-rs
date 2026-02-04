use crate::*;
use anyhow::{Result, bail};
use core::mem::size_of;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use sync_file::{RandomAccessFile, ReadAt};
use tracing::{debug, debug_span, info_span, trace, trace_span};
use zerocopy::IntoBytes;

/// Reads MSFZ files.
pub struct Msfz<F = RandomAccessFile> {
    file: F,
    /// The list of all fragments in all streams.
    ///
    /// `fragments` is sorted by stream index, then by the order of the fragments in each stream.
    /// Each stream has zero or more fragments associated with it. The set of fragments for a stream `s` is
    /// `&fragments[stream_fragments[s] .. stream_fragments[s + 1]]`.
    fragments: Vec<Fragment>,

    /// Contains the index of the first entry in `fragments` for a given stream.
    ///
    /// The last entry in this list does not point to a stream. It simply points to the end of
    /// the `fragments` list.
    ///
    /// Invariant: `stream_fragments.len() > 0`
    /// Invariant: `stream_fragments.len() == num_streams() + 1`.
    stream_fragments: Vec<u32>,

    chunk_table: Box<[ChunkEntry]>,
    chunk_cache: Vec<OnceLock<Arc<[u8]>>>,
}

/// Describes a region within a stream.
#[derive(Clone)]
pub struct Fragment {
    /// The size in bytes of the fragment
    pub size: u32,
    /// The location of the fragment
    pub location: FragmentLocation,
}

impl std::fmt::Debug for Fragment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "size 0x{:05x} at {:?}", self.size, self.location)
    }
}

impl std::fmt::Debug for FragmentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_nil() {
            f.write_str("nil")
        } else if self.is_compressed() {
            write!(
                f,
                "uncompressed at 0x{:06x}",
                self.uncompressed_file_offset()
            )
        } else {
            write!(
                f,
                "chunk {} : 0x{:04x}",
                self.compressed_first_chunk(),
                self.compressed_offset_within_chunk()
            )
        }
    }
}

const FRAGMENT_LOCATION_32BIT_IS_COMPRESSED_MASK: u32 = 1u32 << 31;

/// Represents the location of a fragment, either compressed or uncompressed.
#[derive(Copy, Clone)]
pub struct FragmentLocation {
    /// bits 0-31
    lo: u32,
    /// bits 32-63
    hi: u32,
}

impl FragmentLocation {
    /// This is a sentinel value for `FragmentLocation` that means "this stream is a nil stream".
    /// It is not an actual fragment.
    const NIL: Self = Self {
        lo: u32::MAX,
        hi: u32::MAX,
    };

    fn is_nil(&self) -> bool {
        self.lo == u32::MAX && self.hi == u32::MAX
    }

    /// Returns `true` if this is a compressed fragment
    pub fn is_compressed(&self) -> bool {
        (self.hi & FRAGMENT_LOCATION_32BIT_IS_COMPRESSED_MASK) != 0
    }

    /// Returns the chunk index for this compressed fragment.
    /// 
    /// You must check `is_compressed()` before calling this function.
    pub fn compressed_first_chunk(&self) -> u32 {
        debug_assert!(!self.is_nil());
        debug_assert!(self.is_compressed());
        self.hi & !FRAGMENT_LOCATION_32BIT_IS_COMPRESSED_MASK
    }

    fn compressed_offset_within_chunk(&self) -> u32 {
        debug_assert!(!self.is_nil());
        debug_assert!(self.is_compressed());
        self.lo
    }

    fn uncompressed_file_offset(&self) -> u64 {
        debug_assert!(!self.is_nil());
        debug_assert!(!self.is_compressed());
        ((self.hi as u64) << 32) | (self.lo as u64)
    }
}

impl Msfz<RandomAccessFile> {
    /// Opens an MSFZ file and validates its header.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let f = open_options_shared(File::options().read(true)).open(path)?;
        let raf = RandomAccessFile::from(f);
        Self::from_file(raf)
    }
}

impl<F: ReadAt> Msfz<F> {
    /// Opens an MSFZ file using an implementation of the [`ReadAt`] trait.
    pub fn from_file(file: F) -> Result<Self> {
        let _span = info_span!("Msfz::from_file").entered();

        let mut header: MsfzFileHeader = MsfzFileHeader::new_zeroed();
        file.read_exact_at(header.as_mut_bytes(), 0)?;

        if header.signature != MSFZ_FILE_SIGNATURE {
            bail!("This file does not have a PDZ file signature.");
        }

        if header.version.get() != MSFZ_FILE_VERSION_V0 {
            bail!("This PDZ file uses a version number that is not supported.");
        }

        // Load the stream directory.
        let num_streams = header.num_streams.get();
        if num_streams == 0 {
            bail!("The stream directory is invalid; it is empty.");
        }

        let stream_dir_size_uncompressed = header.stream_dir_size_uncompressed.get() as usize;
        let stream_dir_size_compressed = header.stream_dir_size_compressed.get() as usize;
        let stream_dir_file_offset = header.stream_dir_offset.get();
        let stream_dir_compression = header.stream_dir_compression.get();
        debug!(
            num_streams,
            stream_dir_size_uncompressed,
            stream_dir_size_compressed,
            stream_dir_compression,
            stream_dir_file_offset,
            "reading stream directory"
        );

        let mut stream_dir_bytes: Vec<u8> =
            map_alloc_error(FromZeros::new_vec_zeroed(stream_dir_size_uncompressed))?;
        if let Some(compression) = Compression::try_from_code_opt(stream_dir_compression)? {
            let mut compressed_stream_dir: Vec<u8> =
                map_alloc_error(FromZeros::new_vec_zeroed(stream_dir_size_compressed))?;
            file.read_exact_at(
                compressed_stream_dir.as_mut_bytes(),
                header.stream_dir_offset.get(),
            )?;

            debug!("decompressing stream directory");

            crate::compress_utils::decompress_to_slice(
                compression,
                &compressed_stream_dir,
                &mut stream_dir_bytes,
            )?;
        } else {
            if stream_dir_size_uncompressed != stream_dir_size_compressed {
                bail!(
                    "This PDZ file is invalid. The Stream Directory is not compressed, but has inconsistent compressed vs. uncompressed sizes."
                );
            }
            file.read_exact_at(stream_dir_bytes.as_mut_bytes(), stream_dir_file_offset)?;
        }

        // Load the chunk table.
        let num_chunks = header.num_chunks.get() as usize;
        let chunk_index_size = header.chunk_table_size.get() as usize;
        if chunk_index_size != num_chunks * size_of::<ChunkEntry>() {
            bail!("This PDZ file is invalid. num_chunks and chunk_index_size are not consistent.");
        }

        let chunk_table_offset = header.chunk_table_offset.get();
        let mut chunk_table: Box<[ChunkEntry]> =
            map_alloc_error(FromZeros::new_box_zeroed_with_elems(num_chunks))?;
        if num_chunks != 0 {
            debug!(
                num_chunks,
                chunk_table_offset, "reading compressed chunk table"
            );
            file.read_exact_at(chunk_table.as_mut_bytes(), chunk_table_offset)?;
        } else {
            // Don't issue a read. The writer code may not have actually extended the file.
        }

        let mut chunk_cache = Vec::with_capacity(num_chunks);
        chunk_cache.resize_with(num_chunks, Default::default);

        // Decode the Stream Directory. We do this after loading the chunk table so that we can
        // validate fragment records within the Stream Directory now.
        let stream_dir = decode_stream_dir(&stream_dir_bytes, num_streams, &chunk_table)?;

        Ok(Self {
            file,
            fragments: stream_dir.fragments,
            stream_fragments: stream_dir.stream_fragments,
            chunk_table,
            chunk_cache,
        })
    }

    /// The total number of streams in this MSFZ file. This count includes nil streams.
    pub fn num_streams(&self) -> u32 {
        (self.stream_fragments.len() - 1) as u32
    }

    fn stream_fragments_result(&self, stream: u32) -> Result<&[Fragment]> {
        self.stream_fragments(stream)
            .ok_or_else(|| anyhow::anyhow!("Stream index is out of range"))
    }

    /// Gets the fragments for a given stream.
    ///
    /// If `stream` is out of range, returns `None`.
    pub fn stream_fragments(&self, stream: u32) -> Option<&[Fragment]> {
        let i = stream as usize;
        if i < self.stream_fragments.len() - 1 {
            let start = self.stream_fragments[i] as usize;
            let end = self.stream_fragments[i + 1] as usize;
            let fragments = &self.fragments[start..end];
            match fragments {
                [f, ..] if f.location.is_nil() => Some(&[]),
                _ => Some(fragments),
            }
        } else {
            None
        }
    }

    /// Gets the size of a given stream, in bytes.
    ///
    /// The `stream` value must be in a valid range of `0..num_streams()`.
    ///
    /// If `stream` is a NIL stream, this function returns 0.
    pub fn stream_size(&self, stream: u32) -> Result<u64> {
        let fragments = self.stream_fragments_result(stream)?;
        Ok(fragments.iter().map(|f| f.size as u64).sum())
    }

    /// Returns `true` if `stream` is a valid stream index and the stream is non-nil.
    ///
    /// * If `stream` is 0, returns `false`.
    /// * if `stream` is greater than `num_streams()`, returns false.
    /// * If `stream` is a nil stream, this returns `false`.
    /// * Else returns `true`.
    #[allow(clippy::match_like_matches_macro)]
    pub fn is_stream_valid(&self, stream: u32) -> bool {
        assert!(!self.stream_fragments.is_empty());

        if stream == 0 {
            return false;
        }

        let i = stream as usize;
        if i < self.stream_fragments.len() - 1 {
            let start = self.stream_fragments[i] as usize;
            let end = self.stream_fragments[i + 1] as usize;
            let fragments = &self.fragments[start..end];
            match fragments {
                [f, ..] if f.location.is_nil() => false,
                _ => true,
            }
        } else {
            false
        }
    }

    /// Gets a slice of a chunk. `offset` is the offset within the chunk and `size` is the
    /// length in bytes of the slice. The chunk is loaded and decompressed, if necessary.
    fn get_chunk_slice(&self, chunk: u32, offset: u32, size: u32) -> std::io::Result<&[u8]> {
        let chunk_data = self.get_chunk_data(chunk)?;
        if let Some(slice) = chunk_data.get(offset as usize..offset as usize + size as usize) {
            Ok(slice)
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "PDZ file contains invalid byte ranges within a chunk",
            ))
        }
    }

    fn get_chunk_data(&self, chunk_index: u32) -> std::io::Result<&Arc<[u8]>> {
        let _span = trace_span!("get_chunk_data").entered();
        trace!(chunk_index);

        debug_assert_eq!(self.chunk_cache.len(), self.chunk_table.len());

        let Some(slot) = self.chunk_cache.get(chunk_index as usize) else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Chunk index is out of range.",
            ));
        };

        if let Some(arc) = slot.get() {
            trace!(chunk_index, "found chunk in cache");
            return Ok(arc);
        }

        let arc = self.load_chunk_data(chunk_index)?;
        Ok(slot.get_or_init(move || arc))
    }

    /// This is the slow path for `get_chunk_data`, which loads the chunk data from disk and
    /// decompresses it.
    #[inline(never)]
    fn load_chunk_data(&self, chunk_index: u32) -> std::io::Result<Arc<[u8]>> {
        assert_eq!(self.chunk_cache.len(), self.chunk_table.len());

        let _span = debug_span!("load_chunk_data").entered();

        // We may race with another read that is loading the same entry.
        // For now, that's OK, but in the future we should be smarter about de-duping
        // cache fill requests.

        // We have already implicitly validated the chunk index.
        let entry = &self.chunk_table[chunk_index as usize];

        let compression_opt =
            Compression::try_from_code_opt(entry.compression.get()).map_err(|_| {
                std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    "Chunk uses an unrecognized compression algorithm",
                )
            })?;

        // Read the data from disk.
        let mut compressed_data: Box<[u8]> =
            FromZeros::new_box_zeroed_with_elems(entry.compressed_size.get() as usize)
                .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        self.file
            .read_exact_at(&mut compressed_data, entry.file_offset.get())?;

        let uncompressed_data: Box<[u8]> = if let Some(compression) = compression_opt {
            let mut uncompressed_data: Box<[u8]> =
                FromZeros::new_box_zeroed_with_elems(entry.uncompressed_size.get() as usize)
                    .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;

            self::compress_utils::decompress_to_slice(
                compression,
                &compressed_data,
                &mut uncompressed_data,
            )?;
            uncompressed_data
        } else {
            // This chunk is not compressed.
            compressed_data
        };

        // This conversion should not need to allocate memory for the buffer.  The conversion from
        // Box to Arc should allocate a new Arc object, but the backing allocation for the buffer
        // should simply be transferred.
        Ok(Arc::from(uncompressed_data))
    }

    /// Reads an entire stream to a vector.
    ///
    /// If the stream data fits entirely within a single decompressed chunk, then this function
    /// returns a slice to the data, without copying it.
    pub fn read_stream(&self, stream: u32) -> anyhow::Result<StreamData> {
        let _span = trace_span!("read_stream_to_cow").entered();
        trace!(stream);

        let mut fragments = self.stream_fragments_result(stream)?;

        match fragments.first() {
            Some(f) if f.location.is_nil() => fragments = &[],
            _ => {}
        }

        // If the stream is zero-length, then things are really simple.
        if fragments.is_empty() {
            return Ok(StreamData::empty());
        }

        // If this stream fits in a single fragment and the fragment is compressed, then we can
        // return a single borrowed reference to it. This is common, and is one of the most
        // important optimizations.
        if fragments.len() == 1 && fragments[0].location.is_compressed() {
            let chunk_index = fragments[0].location.compressed_first_chunk();
            let offset_within_chunk = fragments[0].location.compressed_offset_within_chunk();

            let chunk_data = self.get_chunk_data(chunk_index)?;
            let fragment_range = offset_within_chunk as usize
                ..offset_within_chunk as usize + fragments[0].size as usize;

            // Validate the fragment range.
            if chunk_data.get(fragment_range.clone()).is_none() {
                bail!("PDZ data is invalid. Stream fragment byte range is out of range.");
            }

            return Ok(StreamData::ArcSlice(Arc::clone(chunk_data), fragment_range));
        }

        let stream_size: u32 = fragments.iter().map(|f| f.size).sum();
        let stream_usize = stream_size as usize;

        // Allocate a buffer and copy data from each chunk.
        let mut output_buffer: Box<[u8]> = FromZeros::new_box_zeroed_with_elems(stream_usize)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        let mut output_slice: &mut [u8] = &mut output_buffer;

        for fragment in fragments.iter() {
            let stream_offset = stream_usize - output_slice.len();

            // Because we computed stream_usize by summing the fragment sizes, this
            // split_at_mut() call should not fail.
            let (fragment_output_slice, rest) = output_slice.split_at_mut(fragment.size as usize);
            output_slice = rest;

            if fragment.location.is_compressed() {
                let chunk_index = fragment.location.compressed_first_chunk();
                let offset_within_chunk = fragment.location.compressed_offset_within_chunk();

                let chunk_data = self.get_chunk_data(chunk_index)?;
                if let Some(chunk_slice) = chunk_data.get(
                    offset_within_chunk as usize
                        ..offset_within_chunk as usize + fragment.size as usize,
                ) {
                    fragment_output_slice.copy_from_slice(chunk_slice);
                } else {
                    bail!("PDZ data is invalid. Stream fragment byte range is out of range.");
                }
            } else {
                let file_offset = fragment.location.uncompressed_file_offset();
                // Read an uncompressed fragment.
                trace!(
                    file_offset,
                    stream_offset,
                    fragment_len = fragment_output_slice.len(),
                    "reading uncompressed fragment"
                );
                self.file
                    .read_exact_at(fragment_output_slice, file_offset)?;
            }
        }

        assert!(output_slice.is_empty());

        Ok(StreamData::Box(output_buffer))
    }

    /// Returns an object which can read from a given stream.  The returned object implements
    /// the [`Read`], [`Seek`], and [`ReadAt`] traits.
    ///
    /// If `stream` is out of range (greater than or equal to `num_streams()`) then this function
    /// returns an error.
    ///
    /// If `stream` is a nil stream then this function returns a `StreamReader` whose size is 0.
    pub fn get_stream_reader(&self, stream: u32) -> Result<StreamReader<'_, F>> {
        let fragments = self.stream_fragments_result(stream)?;
        Ok(StreamReader {
            msfz: self,
            size: fragments.iter().map(|f| f.size as u64).sum(),
            fragments,
            pos: 0,
        })
    }

    /// The total number of fragments in the MSFZ file.
    pub fn num_fragments(&self) -> usize {
        self.fragments.len()
    }

    /// Raw access to the Fragments table
    pub fn fragments(&self) -> &[Fragment] {
        &self.fragments
    }

    /// The total number of compressed chunks.
    pub fn num_chunks(&self) -> usize {
        self.chunk_table.len()
    }

    /// Raw access to the Chunks table
    pub fn chunks(&self) -> &[ChunkEntry] {
        &self.chunk_table
    }
}

/// Allows reading a stream using the [`Read`], [`Seek`], and [`ReadAt`] traits.
pub struct StreamReader<'a, F> {
    msfz: &'a Msfz<F>,
    size: u64,
    fragments: &'a [Fragment],
    pos: u64,
}

impl<'a, F> StreamReader<'a, F> {
    /// Returns `true` if this is a zero-length stream or a nil stream.
    pub fn is_empty(&self) -> bool {
        self.stream_size() == 0
    }

    /// Size in bytes of the stream.
    ///
    /// This returns zero for nil streams.
    pub fn stream_size(&self) -> u64 {
        self.size
    }
}

impl<'a, F: ReadAt> ReadAt for StreamReader<'a, F> {
    fn read_at(&self, mut buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        let original_buf_len = buf.len();
        let mut current_offset: u64 = offset;

        for fragment in self.fragments.iter() {
            debug_assert!(!buf.is_empty());

            if current_offset >= fragment.size as u64 {
                current_offset -= fragment.size as u64;
                continue;
            }

            // Because of the range check above, we know that this cannot overflow.
            let fragment_bytes_available = fragment.size - current_offset as u32;

            let num_bytes_xfer = buf.len().min(fragment_bytes_available as usize);
            let (buf_xfer, buf_rest) = buf.split_at_mut(num_bytes_xfer);
            buf = buf_rest;

            if fragment.location.is_compressed() {
                let chunk_index = fragment.location.compressed_first_chunk();
                let offset_within_chunk = fragment.location.compressed_offset_within_chunk();

                let chunk_slice = self.msfz.get_chunk_slice(
                    chunk_index,
                    offset_within_chunk + current_offset as u32,
                    num_bytes_xfer as u32,
                )?;
                buf_xfer.copy_from_slice(chunk_slice);
            } else {
                // Read the stream data directly from disk.
                let file_offset = fragment.location.uncompressed_file_offset();
                self.msfz
                    .file
                    .read_exact_at(buf_xfer, file_offset + current_offset)?;
            }

            if buf.is_empty() {
                break;
            }

            if current_offset >= num_bytes_xfer as u64 {
                current_offset -= num_bytes_xfer as u64;
            } else {
                current_offset = 0;
            }
        }

        Ok(original_buf_len - buf.len())
    }
}

impl<'a, F: ReadAt> Read for StreamReader<'a, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let n = self.read_at(buf, self.pos)?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<'a, F> Seek for StreamReader<'a, F> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match pos {
            SeekFrom::Start(p) => self.pos = p,
            SeekFrom::End(offset) => {
                let new_pos = self.stream_size() as i64 + offset;
                if new_pos < 0 {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.pos = new_pos as u64;
            }
            SeekFrom::Current(offset) => {
                let new_pos = self.pos as i64 + offset;
                if new_pos < 0 {
                    return Err(std::io::ErrorKind::InvalidInput.into());
                }
                self.pos = new_pos as u64;
            }
        }
        Ok(self.pos)
    }
}

struct DecodedStreamDir {
    fragments: Vec<Fragment>,
    stream_fragments: Vec<u32>,
}

fn decode_stream_dir(
    stream_dir_bytes: &[u8],
    num_streams: u32,
    chunk_table: &[ChunkEntry],
) -> anyhow::Result<DecodedStreamDir> {
    let mut dec = Decoder {
        bytes: stream_dir_bytes,
    };

    let mut fragments: Vec<Fragment> = Vec::new();
    let mut stream_fragments: Vec<u32> = Vec::with_capacity(num_streams as usize + 1);

    for _ in 0..num_streams {
        stream_fragments.push(fragments.len() as u32);

        let mut fragment_size = dec.u32()?;

        if fragment_size == NIL_STREAM_SIZE {
            // Nil stream. We synthesize a fake fragment record so that we can distinguish
            // nil streams and non-nil streams, and yet optimize for the case where nearly all
            // streams are non-nil.
            fragments.push(Fragment {
                size: 0,
                location: FragmentLocation::NIL,
            });
            continue;
        }

        while fragment_size != 0 {
            debug_assert_ne!(fragment_size, NIL_STREAM_SIZE);

            let location_lo = dec.u32()?;
            let location_hi = dec.u32()?;

            if location_lo == u32::MAX && location_hi == u32::MAX {
                bail!("The Stream Directory contains an invalid fragment record.");
            }

            let location = FragmentLocation {
                lo: location_lo,
                hi: location_hi,
            };

            if location.is_compressed() {
                let first_chunk = location.compressed_first_chunk();
                let offset_within_chunk = location.compressed_offset_within_chunk();

                let Some(chunk) = chunk_table.get(first_chunk as usize) else {
                    bail!(
                        "The Stream Directory contains an invalid fragment record. Chunk index {first_chunk} exceeds the size of the chunk table."
                    );
                };

                let uncompressed_chunk_size = chunk.uncompressed_size.get();

                // Testing for greater-than-or-equal instead of greater-than is correct. Fragments
                // always have a size that is non-zero, so at least one byte must come from the
                // first chunk identified by a compressed fragment.
                if offset_within_chunk >= uncompressed_chunk_size {
                    bail!(
                        "The Stream Directory contains an invalid fragment record. offset_within_chunk {offset_within_chunk} exceeds the size of the chunk."
                    );
                };

                // We could go further and validate that the current fragment extends beyond a
                // valid number of chunks. The stream reader code handles that, though.
            } else {
                // We could validate that the uncompressed fragment lies entirely within the MSFZ
                // file, if we knew the length of the file. Unfortunately, ReadAt does not provide
                // the length of the file, so we will not validate the fragment here. If the
                // fragment is invalid it will cause a read failure within the StreamReader,
                // which will be propagated to the application.
            }

            fragments.push(Fragment {
                size: fragment_size,
                location,
            });

            // Read the fragment size for the next fragment. A value of zero terminates the list,
            // which is handled at the start of the while loop.
            fragment_size = dec.u32()?;
            if fragment_size == NIL_STREAM_SIZE {
                bail!(
                    "Stream directory is malformed. It contains a non-initial fragment with size = NIL_STREAM_SIZE."
                );
            }
            // continue for more
        }
    }

    stream_fragments.push(fragments.len() as u32);

    fragments.shrink_to_fit();

    Ok(DecodedStreamDir {
        fragments,
        stream_fragments,
    })
}

struct Decoder<'a> {
    bytes: &'a [u8],
}

impl<'a> Decoder<'a> {
    fn next_n<const N: usize>(&mut self) -> anyhow::Result<&'a [u8; N]> {
        if self.bytes.len() < N {
            bail!("Buffer ran out of bytes");
        }

        let (lo, hi) = self.bytes.split_at(N);
        self.bytes = hi;
        // This unwrap() should never fail because we just tested the length, above.
        // The optimizer should eliminate the unwrap() call.
        Ok(<&[u8; N]>::try_from(lo).unwrap())
    }

    fn u32(&mut self) -> anyhow::Result<u32> {
        Ok(u32::from_le_bytes(*self.next_n()?))
    }
}

fn map_alloc_error<T>(result: Result<T, zerocopy::AllocError>) -> anyhow::Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(zerocopy::AllocError) => {
            Err(std::io::Error::from(std::io::ErrorKind::OutOfMemory).into())
        }
    }
}
