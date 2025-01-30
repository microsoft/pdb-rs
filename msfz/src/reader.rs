use crate::*;
use anyhow::{bail, Result};
use core::mem::size_of;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, OnceLock};
use sync_file::{RandomAccessFile, ReadAt};
use tracing::{debug, debug_span, info, info_span, trace, trace_span};
use zerocopy::IntoBytes;

/// Reads MSFZ files.
pub struct Msfz<F = RandomAccessFile> {
    file: F,
    stream_dir: Vec<Option<Stream>>,
    chunk_table: Box<[ChunkEntry]>,
    chunk_cache: Vec<OnceLock<Arc<[u8]>>>,
}

impl Msfz<RandomAccessFile> {
    /// Opens an MSFZ file and validates its header.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let f = File::open(path)?;
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
        info!(
            num_streams,
            stream_dir_size_uncompressed,
            stream_dir_size_compressed,
            stream_dir_compression,
            stream_dir_file_offset,
            "reading stream directory"
        );

        let mut stream_dir_bytes: Vec<u8> = vec![0; stream_dir_size_uncompressed];
        if let Some(compression) = Compression::try_from_code_opt(stream_dir_compression)? {
            let mut compressed_stream_dir: Vec<u8> = vec![0; stream_dir_size_compressed];
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
            file.read_exact_at(stream_dir_bytes.as_mut_bytes(), stream_dir_file_offset)?;
        }

        let stream_dir = decode_stream_dir(&stream_dir_bytes, num_streams)?;

        // Load the chunk table.
        let num_chunks = header.num_chunks.get() as usize;
        let chunk_index_size = header.chunk_table_size.get() as usize;
        if chunk_index_size != num_chunks * size_of::<ChunkEntry>() {
            bail!("This PDZ file is invalid. num_chunks and chunk_index_size are not consistent.");
        }

        let chunk_table_offset = header.chunk_table_offset.get();
        // unwrap() is for OOM handling.
        let mut chunk_table: Box<[ChunkEntry]> =
            FromZeros::new_box_zeroed_with_elems(num_chunks).unwrap();
        if num_chunks != 0 {
            info!(
                num_chunks,
                chunk_table_offset, "reading compressed chunk table"
            );
            file.read_exact_at(chunk_table.as_mut_bytes(), chunk_table_offset)?;
        } else {
            // Don't issue a read. The writer code may not have actually extended the file.
        }

        let mut chunk_cache = Vec::with_capacity(num_chunks);
        chunk_cache.resize_with(num_chunks, Default::default);

        Ok(Self {
            file,
            stream_dir,
            chunk_table,
            chunk_cache,
        })
    }

    /// The total number of streams in this MSFZ file. This count includes nil streams.
    pub fn num_streams(&self) -> u32 {
        self.stream_dir.len() as u32
    }

    /// Gets the size of a given stream, in bytes.
    ///
    /// The `stream` value must be in a valid range of `0..num_streams()`.
    ///
    /// If `stream` is a NIL stream, this function returns 0.
    pub fn stream_size(&self, stream: u32) -> u64 {
        assert!((stream as usize) < self.stream_dir.len());
        if let Some(stream) = &self.stream_dir[stream as usize] {
            stream.fragments.iter().map(|f| f.size as u64).sum()
        } else {
            0
        }
    }

    /// Returns `true` if `stream` is a valid stream index and the stream is non-nil.
    ///
    /// Stream index 0 is reserved; this function always returns `true` for stream index 0,
    /// but the stream cannot be used to store data.
    pub fn is_stream_valid(&self, stream: u32) -> bool {
        if let Some(s) = self.stream_dir.get(stream as usize) {
            s.is_some()
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

        let stream = match self.stream_dir.get(stream as usize) {
            // Stream index is out of range.
            None => bail!("Invalid stream index"),

            // Nil stream case.
            Some(None) => return Ok(StreamData::empty()),

            Some(Some(entry)) => entry,
        };

        // If the stream is zero-length, then things are really simple.
        if stream.fragments.is_empty() {
            return Ok(StreamData::empty());
        }

        // If this stream fits in a single fragment and the fragment is compressed, then we can
        // return a single borrowed reference to it. This is common, and is one of the most
        // important optimizations.
        if stream.fragments.len() == 1 {
            if let Fragment {
                size,
                location:
                    FragmentLocation::Compressed {
                        chunk_index,
                        offset_within_chunk,
                    },
            } = &stream.fragments[0]
            {
                let chunk_data = self.get_chunk_data(*chunk_index)?;
                let fragment_range =
                    *offset_within_chunk as usize..*offset_within_chunk as usize + *size as usize;

                // Validate the fragment range.
                if chunk_data.get(fragment_range.clone()).is_none() {
                    bail!("PDZ data is invalid. Stream fragment byte range is out of range.");
                }

                return Ok(StreamData::ArcSlice(Arc::clone(chunk_data), fragment_range));
            }
        }

        let stream_size: u32 = stream.fragments.iter().map(|f| f.size).sum();
        let stream_usize = stream_size as usize;

        // Allocate a buffer and copy data from each chunk.
        let mut output_buffer: Box<[u8]> = FromZeros::new_box_zeroed_with_elems(stream_usize)
            .map_err(|_| std::io::Error::from(std::io::ErrorKind::OutOfMemory))?;
        let mut output_slice: &mut [u8] = &mut output_buffer;

        trace!(num_fragments = stream.fragments.len());

        for fragment in stream.fragments.iter() {
            let stream_offset = stream_usize - output_slice.len();

            // Because we computed stream_usize by summing the fragment sizes, this
            // split_at_mut() call should not fail.
            let (fragment_output_slice, rest) = output_slice.split_at_mut(fragment.size as usize);
            output_slice = rest;

            match fragment.location {
                FragmentLocation::Compressed {
                    chunk_index,
                    offset_within_chunk,
                } => {
                    let chunk_data = self.get_chunk_data(chunk_index)?;
                    if let Some(chunk_slice) = chunk_data.get(
                        offset_within_chunk as usize
                            ..offset_within_chunk as usize + fragment.size as usize,
                    ) {
                        fragment_output_slice.copy_from_slice(chunk_slice);
                    } else {
                        bail!("PDZ data is invalid. Stream fragment byte range is out of range.");
                    };
                }

                FragmentLocation::Uncompressed { file_offset } => {
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
        }

        assert!(output_slice.is_empty());

        Ok(StreamData::Box(output_buffer))
    }

    /// Returns an object which can read from a given stream.  The returned object implements
    /// the [`Read`], [`Seek`], and [`ReadAt`] traits.
    pub fn get_stream_reader(&self, stream: u32) -> Result<StreamReader<'_, F>> {
        match self.stream_dir.get(stream as usize) {
            None => bail!("Invalid stream index"),

            Some(None) => Ok(StreamReader {
                msfz: self,
                fragments: &[],
                size: 0,
                pos: 0,
            }),

            Some(Some(entry)) => Ok(StreamReader {
                msfz: self,
                fragments: &entry.fragments,
                size: entry.fragments.iter().map(|f| f.size).sum(),
                pos: 0,
            }),
        }
    }
}

/// Allows reading a stream using the [`Read`], [`Seek`], and [`ReadAt`] traits.
pub struct StreamReader<'a, F> {
    msfz: &'a Msfz<F>,
    size: u32,
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
    pub fn stream_size(&self) -> u32 {
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

            match fragment.location {
                FragmentLocation::Compressed {
                    chunk_index,
                    offset_within_chunk,
                } => {
                    let chunk_slice = self.msfz.get_chunk_slice(
                        chunk_index,
                        offset_within_chunk + current_offset as u32,
                        num_bytes_xfer as u32,
                    )?;
                    buf_xfer.copy_from_slice(chunk_slice);
                }

                FragmentLocation::Uncompressed { file_offset } => {
                    // Read the stream data directly from disk.
                    self.msfz
                        .file
                        .read_exact_at(buf_xfer, file_offset + current_offset)?;
                }
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

fn decode_stream_dir(
    stream_dir_bytes: &[u8],
    num_streams: u32,
) -> anyhow::Result<Vec<Option<Stream>>> {
    let mut dec = Decoder {
        bytes: stream_dir_bytes,
    };

    let mut streams: Vec<Option<Stream>> = Vec::with_capacity(num_streams as usize);

    // Reusable buffer. We do this so that we can allocate exactly-sized fragments vectors for
    // each stream.
    let mut fragments: Vec<Fragment> = Vec::with_capacity(0x20);

    for _ in 0..num_streams {
        assert!(fragments.is_empty());

        let mut fragment_size = dec.u32()?;

        if fragment_size == NIL_STREAM_SIZE {
            // Nil stream.
            streams.push(None);
            continue;
        }

        while fragment_size != 0 {
            debug_assert_ne!(fragment_size, NIL_STREAM_SIZE);

            let mut location_bits = dec.u64()?;

            let location = if (location_bits & FRAGMENT_LOCATION_CHUNK_MASK) != 0 {
                location_bits &= !FRAGMENT_LOCATION_CHUNK_MASK;
                FragmentLocation::Compressed {
                    chunk_index: (location_bits >> 32) as u32,
                    offset_within_chunk: location_bits as u32,
                }
            } else {
                // This is an uncompressed fragment. Location is a file offset.
                FragmentLocation::Uncompressed {
                    file_offset: location_bits,
                }
            };
            fragments.push(Fragment {
                size: fragment_size,
                location,
            });

            // Read the fragment size for the next fragment. A value of zero terminates the list,
            // which is handled at the start of the while loop.
            fragment_size = dec.u32()?;
            if fragment_size == NIL_STREAM_SIZE {
                bail!("Stream directory is malformed. It contains a non-initial fragment with size = NIL_STREAM_SIZE.");
            }
            // continue for more
        }

        // Move the fragments to a new buffer with exact size, now that we know how many fragments
        // there are in this stream.
        let mut taken_fragments = Vec::with_capacity(fragments.len());
        taken_fragments.append(&mut fragments);
        streams.push(Some(Stream {
            fragments: taken_fragments,
        }));
    }

    Ok(streams)
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

    fn u64(&mut self) -> anyhow::Result<u64> {
        Ok(u64::from_le_bytes(*self.next_n()?))
    }
}
