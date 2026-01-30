use super::*;
use anyhow::anyhow;
use pow2::Pow2;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tracing::{debug, debug_span, trace, trace_span};
use zerocopy::IntoBytes;

/// The default threshold for compressing a chunk of data.
pub const DEFAULT_CHUNK_THRESHOLD: u32 = 0x40_0000; // 4 MiB

/// The minimum value for the uncompressed chunk size threshold.
pub const MIN_CHUNK_SIZE: u32 = 0x1000;

/// The maximum value for the uncompressed chunk size threshold.
pub const MAX_CHUNK_SIZE: u32 = 1 << 30;

/// Allows writing a new MSFZ file.
pub struct MsfzWriter<F: Write + Seek = File> {
    pub(crate) file: MsfzWriterFile<F>,

    /// The list of streams. This includes nil streams and non-nil streams. Nil streams are
    /// represented with `None`.
    pub(crate) streams: Vec<Option<Stream>>,
}

pub(crate) struct MsfzWriterFile<F: Write + Seek> {
    /// Max number of bytes to write into `uncompressed_chunk_data` before finishing (compressing
    /// and writing to disk) a chunk.
    uncompressed_chunk_size_threshold: u32,

    /// Holds data for the current chunk that we are building.
    uncompressed_chunk_data: Vec<u8>,

    /// A reusable buffer used for compressing the current chunk. This exists only to reduce
    /// memory allocation churn.
    compressed_chunk_buffer: Vec<u8>,
    /// The list of complete compressed chunks that have been written to disk.
    chunks: Vec<ChunkEntry>,

    /// Compression mode to use for the next chunk.
    chunk_compression_mode: Compression,

    /// The output file.
    pub(crate) out: F,
}

impl std::fmt::Debug for FragmentLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Uncompressed { file_offset } => {
                write!(f, "uncompressed at 0x{file_offset:06x}")
            }
            Self::Compressed {
                chunk_index,
                offset_within_chunk,
            } => write!(f, "chunk {chunk_index} : 0x{offset_within_chunk:04x}"),
        }
    }
}

// Describes a region within a stream.
#[derive(Clone, Debug)]
pub(crate) struct Fragment {
    pub(crate) size: u32,
    pub(crate) location: FragmentLocation,
}

#[derive(Default)]
pub(crate) struct Stream {
    pub(crate) fragments: Vec<Fragment>,
}

const FRAGMENT_LOCATION_CHUNK_BIT: u32 = 63;
const FRAGMENT_LOCATION_CHUNK_MASK: u64 = 1 << FRAGMENT_LOCATION_CHUNK_BIT;

#[derive(Clone)]
pub(crate) enum FragmentLocation {
    Uncompressed {
        file_offset: u64,
    },
    Compressed {
        chunk_index: u32,
        offset_within_chunk: u32,
    },
}

/// Describes the results of writing an MSFZ file.
#[non_exhaustive]
pub struct Summary {
    /// Number of chunks
    pub num_chunks: u32,
    /// Number of streams
    pub num_streams: u32,
}

impl std::fmt::Display for Summary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Number of chunks: {}", self.num_chunks)?;
        writeln!(f, "Number of streams: {}", self.num_streams)?;
        Ok(())
    }
}

impl MsfzWriter<File> {
    /// Creates a new writer on a file at a given path.
    ///
    /// This will *truncate* any existing file.
    pub fn create(file_name: &Path) -> Result<Self> {
        let f = open_options_exclusive(File::options().write(true).create(true).truncate(true))
            .open(file_name)?;
        Self::new(f)
    }
}

impl<F: Write + Seek> MsfzWriter<F> {
    /// Creates a new writer on an object that implements [`Write`] (and [`Seek`]), such as
    /// [`File`].
    pub fn new(mut file: F) -> Result<Self> {
        let _span = trace_span!("MsfzWriter::new").entered();

        file.seek(SeekFrom::Start(0))?;

        // Write a meaningless (zero-filled) file header, just so we get the file position that we
        // want.  We will re-write this header at the end.
        let fake_file_header = MsfzFileHeader::new_zeroed();
        file.write_all(fake_file_header.as_bytes())?;

        // We do not know how many streams the writer will write. We reserve a small fixed number,
        // just guessing at the size.
        let mut streams = Vec::with_capacity(0x40);

        // Reserve stream 0 for the stream directory. The stream is zero-length.
        // Setting the size to 0 also prevents this stream from being written, which is good.
        streams.push(Some(Stream {
            fragments: Vec::new(),
        }));

        let mut this = Self {
            streams,
            file: MsfzWriterFile {
                uncompressed_chunk_size_threshold: DEFAULT_CHUNK_THRESHOLD,
                uncompressed_chunk_data: Vec::with_capacity(DEFAULT_CHUNK_THRESHOLD as usize),
                compressed_chunk_buffer: Vec::new(),
                out: file,
                chunks: Vec::new(),
                chunk_compression_mode: Compression::Zstd,
            },
        };
        this.file.write_align(Pow2::from_exponent(4))?;
        Ok(this)
    }

    /// Sets the compression mode that is used for chunked compression.
    pub fn set_chunk_compression_mode(&mut self, compression: Compression) {
        // If the current chunk buffer contains data, then leave it there. It will be compressed
        // with the new algorithm.
        self.file.chunk_compression_mode = compression;
    }

    /// Sets the maximum uncompressed size for each chunk.
    ///
    /// This is an optimization hint. The implementation will do its best to keep chunks below this
    /// size, but there are cases where the chunk has already exceeded the specified size.
    pub fn set_uncompressed_chunk_size_threshold(&mut self, value: u32) {
        self.file.uncompressed_chunk_size_threshold = value.max(MIN_CHUNK_SIZE).min(MAX_CHUNK_SIZE);
    }

    /// Gets the maximum uncompressed size for each chunk.
    pub fn uncompressed_chunk_size_threshold(&self) -> u32 {
        self.file.uncompressed_chunk_size_threshold
    }

    /// Reserves `num_streams` streams.
    ///
    /// If `num_streams` is less than or equal to the current number of streams, then this
    /// function has no effect.
    ///
    /// If `num_streams` is greater than the current number of streams, then new "nil" streams are
    /// added to the stream directory. These streams can be written by using the `stream_writer`
    /// function. The `stream_writer` function can only be called once for each stream index.
    pub fn reserve_num_streams(&mut self, num_streams: usize) {
        if num_streams <= self.streams.len() {
            return;
        }

        self.streams.resize_with(num_streams, Option::default);
    }

    /// Ends the current chunk, if any.
    ///
    /// This function is a performance hint for compression. It is not necessary to call this
    /// function. If you are writing two different streams that have very different contents, then
    /// it may be beneficial to put the streams into different compression chunks. This allows
    /// the compressor to adapt to the different contents of each stream.
    pub fn end_chunk(&mut self) -> std::io::Result<()> {
        self.file.finish_current_chunk()
    }

    /// Writes an existing stream.
    ///
    /// This function can only be called once for each stream index. Calling it more than once
    /// for the same stream is permitted. Note that settings on [`StreamWriter`] do not persist
    /// across multiple calls to `stream_writer()`, such as enabling/disabling chunked compression.
    pub fn stream_writer(&mut self, stream: u32) -> std::io::Result<StreamWriter<'_, F>> {
        assert!((stream as usize) < self.streams.len());

        Ok(StreamWriter {
            file: &mut self.file,
            stream: self.streams[stream as usize].get_or_insert_with(Stream::default),
            chunked_compression_enabled: true,
            alignment: Pow2::from_exponent(2), // default is 4-byte alignment
        })
    }

    /// Creates a new stream and returns a [`StreamWriter`] for it.
    pub fn new_stream_writer(&mut self) -> Result<(u32, StreamWriter<'_, F>)> {
        let stream = self.streams.len() as u32;
        self.streams.push(Some(Stream::default()));
        let w = self.stream_writer(stream)?;
        Ok((stream, w))
    }

    /// Finishes writing the MSFZ file.
    ///
    /// This writes the Stream Directory, the Chunk Table, and then writes the MSFZ file header.
    /// It then returns the inner file object. However, the caller should not write more data to
    /// the returned file object.
    pub fn finish(self) -> Result<(Summary, F)> {
        self.finish_with_options(MsfzFinishOptions::default())
    }

    /// Finishes writing the MSFZ file.
    ///
    /// This writes the Stream Directory, the Chunk Table, and then writes the MSFZ file header.
    /// It then returns the inner file object. However, the caller should not write more data to
    /// the returned file object.
    ///
    /// This function also allows the caller to pass `MsfzFinishOptions`.
    pub fn finish_with_options(mut self, options: MsfzFinishOptions) -> Result<(Summary, F)> {
        let _span = debug_span!("MsfzWriter::finish").entered();

        self.file.finish_current_chunk()?;

        // Write the stream directory, and optionally compress it.
        let directory_offset = self.file.write_align(Pow2::from_exponent(4))?;

        let stream_dir_bytes: Vec<u8> = encode_stream_dir(&self.streams);
        let stream_dir_size_uncompressed = u32::try_from(stream_dir_bytes.len())
            .map_err(|_| anyhow!("The stream directory is too large."))?;
        let stream_dir_size_compressed: u32;
        let stream_dir_compression: u32;
        if let Some(compression) = options.stream_dir_compression {
            stream_dir_compression = compression.to_code();
            let stream_dir_compressed_bytes =
                crate::compress_utils::compress_to_vec(compression, &stream_dir_bytes)?;
            stream_dir_size_compressed = stream_dir_compressed_bytes.len() as u32;
            self.file.out.write_all(&stream_dir_compressed_bytes)?;
        } else {
            self.file.out.write_all(&stream_dir_bytes)?;
            stream_dir_size_compressed = stream_dir_size_uncompressed;
            stream_dir_compression = COMPRESSION_NONE;
        }

        // Write the chunk list.
        let chunk_table_offset = self.file.write_align(Pow2::from_exponent(4))?;
        let chunk_table_bytes = self.file.chunks.as_bytes();
        let chunk_table_size = u32::try_from(chunk_table_bytes.len())
            .map_err(|_| anyhow!("The chunk index is too large."))?;
        self.file.out.write_all(chunk_table_bytes)?;

        // Rewind and write the real file header.
        let file_header = MsfzFileHeader {
            signature: MSFZ_FILE_SIGNATURE,
            version: U64::new(MSFZ_FILE_VERSION_V0),
            num_streams: U32::new(self.streams.len() as u32),
            stream_dir_compression: U32::new(stream_dir_compression),
            stream_dir_offset: U64::new(directory_offset),
            stream_dir_size_compressed: U32::new(stream_dir_size_compressed),
            stream_dir_size_uncompressed: U32::new(stream_dir_size_uncompressed),
            num_chunks: U32::new(self.file.chunks.len() as u32),
            chunk_table_size: U32::new(chunk_table_size),
            chunk_table_offset: U64::new(chunk_table_offset),
        };
        self.file.out.seek(SeekFrom::Start(0))?;
        self.file.out.write_all(file_header.as_bytes())?;

        if options.min_file_size != 0 {
            let file_length = self.file.out.seek(SeekFrom::End(0))?;
            if file_length < options.min_file_size {
                debug!(
                    file_length,
                    options.min_file_size, "Extending file to meet minimum length requirement"
                );
                // Write a single byte at the end of the file. We do this because there is no
                // way to set the stream length without writing some bytes.
                self.file
                    .out
                    .seek(SeekFrom::Start(options.min_file_size - 1))?;
                self.file.out.write_all(&[0u8])?;
            }
        }

        let summary = Summary {
            num_chunks: self.file.chunks.len() as u32,
            num_streams: self.streams.len() as u32,
        };

        Ok((summary, self.file.out))
    }
}

/// Handles packing and unpacking the `file_offset` for compressed streams.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct ChunkAndOffset {
    chunk: u32,
    offset: u32,
}

impl<F: Write + Seek> MsfzWriterFile<F> {
    /// Writes `data` to the compressed chunk stream and returns the location of the start of the
    /// data.
    ///
    /// This function does its best to keep chunks below the `uncompressed_chunk_size_threshold`.
    /// If `data.len()` would cause the current chunk to overflow the threshold, then this function
    /// finishes the current chunk and starts a new one.
    ///
    /// If `data.len()` itself is larger than `uncompressed_chunk_size_threshold`, then this
    /// function will write a chunk that is larger than `uncompressed_chunk_size_threshold`.
    /// This is common for very large data streams, such as the TPI or GSS.  Writers that want to
    /// avoid encoding very large chunks will need to break up the data and call
    /// `write_to_chunks()` repeatedly.
    ///
    /// All of the bytes of `data` will be written to a single chunk; this function never splits
    /// the data across multiple chunks.
    fn write_to_chunks(&mut self, data: &[u8]) -> std::io::Result<ChunkAndOffset> {
        let _span = debug_span!("write_to_chunks").entered();

        if data.len() + self.uncompressed_chunk_data.len()
            >= self.uncompressed_chunk_size_threshold as usize
        {
            self.finish_current_chunk()?;
        }

        // There is no guarantee that the input data fits below our threshold, of course. If we
        // receive a buffer whose size exceeds our threshold, we'll just write a larger-than-usual
        // chunk. That's ok, everything should still work.

        let chunk = self.chunks.len() as u32;
        let offset_within_chunk = self.uncompressed_chunk_data.len();

        self.uncompressed_chunk_data.extend_from_slice(data);
        Ok(ChunkAndOffset {
            chunk,
            offset: offset_within_chunk as u32,
        })
    }

    #[inline(never)]
    fn finish_current_chunk(&mut self) -> std::io::Result<()> {
        let _span = debug_span!("finish_current_chunk").entered();

        if self.uncompressed_chunk_data.is_empty() {
            return Ok(());
        }

        let _span = trace_span!("MsfzWriter::finish_current_chunk").entered();

        {
            let _span = trace_span!("compress chunk").entered();
            self.compressed_chunk_buffer.clear();
            crate::compress_utils::compress_to_vec_mut(
                self.chunk_compression_mode,
                &self.uncompressed_chunk_data,
                &mut self.compressed_chunk_buffer,
            )?;
        }

        let file_pos;
        {
            let _span = trace_span!("write to disk").entered();
            file_pos = self.out.stream_position()?;
            self.out.write_all(&self.compressed_chunk_buffer)?;
        }

        trace!(
            file_pos,
            compressed_size = self.compressed_chunk_buffer.len(),
            uncompressed_size = self.uncompressed_chunk_data.len()
        );

        self.chunks.push(ChunkEntry {
            compressed_size: U32::new(self.compressed_chunk_buffer.len() as u32),
            uncompressed_size: U32::new(self.uncompressed_chunk_data.len() as u32),
            file_offset: U64::new(file_pos),
            compression: U32::new(self.chunk_compression_mode.to_code()),
        });

        self.uncompressed_chunk_data.clear();
        self.compressed_chunk_buffer.clear();

        Ok(())
    }

    /// Ensures that the current stream write position on the output file is aligned to a multiple
    /// of the given alignment.
    fn write_align(&mut self, alignment: Pow2) -> std::io::Result<u64> {
        let pos = self.out.stream_position()?;
        if alignment.is_aligned(pos) {
            return Ok(pos);
        }

        let Some(aligned_pos) = alignment.align_up(pos) else {
            return Err(std::io::ErrorKind::InvalidInput.into());
        };

        self.out.seek(SeekFrom::Start(aligned_pos))?;
        Ok(aligned_pos)
    }
}

/// Allows writing data to a stream.
///
/// This object does not implement [`Seek`] and there is no variant of this object that allows
/// seeking or writing to arbitrary offsets. Stream data must be written sequentially.
///
/// After a [`StreamWriter`] is closed (dropped), it is not possible to create a new `StreamWriter`
/// for the same stream.
///
/// # Write calls are never split across chunks
///
/// The [`Write`] implementation of this type makes a guarantee: For a given call to
/// [`StreamWriter::write`], if the current stream is using chunked compression, then the data will
/// be written to a single compressed chunk. This is an implementation guarantee; it is not required
/// by the MSFZ specification.
///
/// This allows readers to rely on complete records being stored within a single chunk. For example,
/// when copying the TPI, an encoder _could_ issue a sequence of `write()` calls whose boundaries
/// align with the boundaries of the records within the TPI. This would allow the reader to read
/// records directly from the chunk decompressed buffer, without needing to allocate a separate
/// buffer or copy the records. (We do not currently implement that behavior; this is describing a
/// hypothetical.)
pub struct StreamWriter<'a, F: Write + Seek> {
    file: &'a mut MsfzWriterFile<F>,
    stream: &'a mut Stream,
    alignment: Pow2,
    chunked_compression_enabled: bool,
}

impl<'a, F: Write + Seek> StreamWriter<'a, F> {
    /// Ends the current chunk, if any.
    ///
    /// This function is a performance hint for compression. It is not necessary to call this
    /// function.
    pub fn end_chunk(&mut self) -> std::io::Result<()> {
        self.file.finish_current_chunk()
    }

    /// Specifies whether to use chunked compression or not. The default value for this setting is
    /// `true` (chunked compression is enabled).
    ///
    /// This does not have any immediate effect. It controls the behavior of the `write()`
    /// implementation for this stream.
    ///
    /// If this is called with `false`, then `write()` calls that follow this will cause stream
    /// data to be written to disk without compression.
    pub fn set_compression_enabled(&mut self, value: bool) {
        self.chunked_compression_enabled = value;
    }

    /// Specifies the on-disk alignment requirement for the start of the stream data.
    ///
    /// This only applies to uncompressed streams. Compressed stream data is always stored within
    /// compressed chunks, so the alignment is meaningless.
    pub fn set_alignment(&mut self, value: Pow2) {
        self.alignment = value;
    }
}

impl<'a, F: Write + Seek> Write for StreamWriter<'a, F> {
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _span = trace_span!("StreamWriter::write").entered();
        trace!(buf_len = buf.len());

        if buf.is_empty() {
            return Ok(0);
        }

        let old_stream_size: u32 = self.stream.fragments.iter().map(|f| f.size).sum();
        let is_first_write = old_stream_size == 0;
        let max_new_bytes = NIL_STREAM_SIZE - old_stream_size;

        // Check that buf.len() can be converted to u32, that the increase in size does not
        // overflow u32, and that writing the new data will not cause the stream size to erroneously
        // become NIL_STREAM_SIZE.
        let buf_len = match u32::try_from(buf.len()) {
            Ok(buf_len) if buf_len < max_new_bytes => buf_len,
            _ => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "The input is too large for an MSFZ stream.",
                ));
            }
        };

        if self.chunked_compression_enabled {
            let chunk_at = self.file.write_to_chunks(buf)?;

            add_fragment_compressed(
                &mut self.stream.fragments,
                buf_len,
                chunk_at.chunk,
                chunk_at.offset,
            );
        } else {
            let fragment_file_offset: u64 = if is_first_write {
                self.file.write_align(self.alignment)?
            } else {
                self.file.out.stream_position()?
            };
            self.file.out.write_all(buf)?;

            add_fragment_uncompressed(&mut self.stream.fragments, buf_len, fragment_file_offset);
        }

        Ok(buf.len())
    }
}

/// Adds a fragment record to a fragment list for a compressed fragment. If possible, the new
/// fragment is coalesced with the last record.
fn add_fragment_compressed(
    fragments: &mut Vec<Fragment>,
    new_fragment_size: u32,
    new_chunk: u32,
    new_offset_within_chunk: u32,
) {
    debug!(
        new_fragment_size,
        new_chunk, new_offset_within_chunk, "add_fragment_compressed"
    );

    // Either create a new fragment for this write or coalesce it with the previous fragment.
    match fragments.last_mut() {
        Some(Fragment {
            size: last_fragment_size,
            location:
                FragmentLocation::Compressed {
                    chunk_index: last_chunk,
                    offset_within_chunk: last_offset_within_chunk,
                },
        }) if *last_chunk == new_chunk
            && *last_offset_within_chunk + new_fragment_size == new_offset_within_chunk =>
        {
            *last_fragment_size += new_fragment_size;
        }

        _ => {
            // We cannot extend the last fragment, or there is no last fragment.
            fragments.push(Fragment {
                size: new_fragment_size,
                location: FragmentLocation::Compressed {
                    chunk_index: new_chunk,
                    offset_within_chunk: new_offset_within_chunk,
                },
            });
        }
    }
}

/// Adds a fragment record to a fragment list for an uncompressed fragment. If possible, the new
/// fragment is coalesced with the last record.
fn add_fragment_uncompressed(
    fragments: &mut Vec<Fragment>,
    new_fragment_size: u32,
    new_file_offset: u64,
) {
    debug!(
        new_fragment_size,
        new_file_offset, "add_fragment_uncompressed"
    );

    match fragments.last_mut() {
        Some(Fragment {
            size: last_fragment_size,
            location:
                FragmentLocation::Uncompressed {
                    file_offset: last_fragment_file_offset,
                },
        }) if *last_fragment_file_offset + new_fragment_size as u64 == new_file_offset => {
            *last_fragment_size += new_fragment_size;
        }

        _ => {
            // We cannot extend the last fragment, or there is no last fragment.
            fragments.push(Fragment {
                size: new_fragment_size,
                location: FragmentLocation::Uncompressed {
                    file_offset: new_file_offset,
                },
            });
        }
    }
}

/// Encodes a stream directory to its byte representation.
pub(crate) fn encode_stream_dir(streams: &[Option<Stream>]) -> Vec<u8> {
    let _span = trace_span!("encode_stream_dir").entered();

    let mut stream_dir_encoded: Vec<u8> = Vec::new();
    let mut enc = Encoder {
        vec: &mut stream_dir_encoded,
    };

    for stream_opt in streams.iter() {
        if let Some(stream) = stream_opt {
            for fragment in stream.fragments.iter() {
                assert_ne!(fragment.size, 0);
                assert_ne!(fragment.size, NIL_STREAM_SIZE);
                enc.u32(fragment.size);

                let location: u64 = match fragment.location {
                    FragmentLocation::Compressed {
                        chunk_index,
                        offset_within_chunk,
                    } => {
                        ((chunk_index as u64) << 32)
                            | (offset_within_chunk as u64)
                            | FRAGMENT_LOCATION_CHUNK_MASK
                    }
                    FragmentLocation::Uncompressed { file_offset } => file_offset,
                };

                enc.u64(location)
            }

            // Write 0 to the list to terminate the list of fragments.
            enc.u32(0);
        } else {
            // It's a nil stream. Our encoding writes a single NIL_STREAM_SIZE value to the
            // stream directory. It is _not_ followed by a fragment list.
            enc.u32(NIL_STREAM_SIZE);
        }
    }

    stream_dir_encoded.as_bytes().to_vec()
}

struct Encoder<'a> {
    vec: &'a mut Vec<u8>,
}

impl<'a> Encoder<'a> {
    fn u32(&mut self, value: u32) {
        self.vec.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.vec.extend_from_slice(&value.to_le_bytes());
    }
}

/// Defines options for finishing an MSFZ file.
#[derive(Clone, Debug, Default)]
pub struct MsfzFinishOptions {
    /// The minimum output file size. Use `MIN_FILE_SIZE_16K` to guarantee compatibility with
    /// MSVC tools that can read PDZ files.
    pub min_file_size: u64,

    /// If `Some`, then the Stream Directory will be compressed.
    pub stream_dir_compression: Option<Compression>,
}

/// This is the minimum file size that is guaranteed to avoid triggering a bug in the first
/// version of the PDZ decoder compiled into DIA (and other MSVC-derived tools).
pub const MIN_FILE_SIZE_16K: u64 = 0x4000;
