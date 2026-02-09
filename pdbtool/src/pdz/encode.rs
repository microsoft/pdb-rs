//! Compresses PDB files into "PDZ" (compressed PDB) files.

use crate::pdz::util::*;
use anyhow::{Context, Result, bail};
use ms_pdb::codeview::HasRestLen;
use ms_pdb::dbi::DbiStreamHeader;
use ms_pdb::msf::Msf;
use ms_pdb::msfz::{self, MIN_FILE_SIZE_16K, MsfzFinishOptions, MsfzWriter, StreamWriter};
use ms_pdb::syms::SymIter;
use ms_pdb::tpi;
use ms_pdb::types::TypesIter;
use ms_pdb::{RandomAccessFile, Stream};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;
use tracing::{debug, error, info, trace, trace_span, warn};
use zerocopy::{FromBytes, IntoBytes};

#[derive(clap::Parser, Debug)]
pub(crate) struct PdzEncodeOptions {
    /// Path to the input PDB file.
    pub input_pdb: String,

    /// Path to the output PDZ file.
    pub output_pdz: String,

    /// Pad the output PDZ file to a minimum of 16 KB. This is a workaround for a bug in the
    /// original MSVC implementation of the PDZ *decoder*.
    #[arg(long)]
    pub pad16k: bool,

    /// Compress the Stream Directory. Although the PDZ specification defines stream directory
    /// compression, some PDZ readers do not yet support reading compressed stream directories.
    #[arg(long)]
    pub compress_stream_dir: bool,

    /// If a file is not a PDB, then simply copy it to the destination, unchanged. This will
    /// copy Portable PDBs and PDZs to the output without changing them.
    #[arg(long)]
    pub copy_unrecognized: bool,

    /// The maximum _uncompressed_ size of each chunk, in bytes.  (You can specify a suffix of
    /// K for 1024 or M for 1048576.). Chunk contents are accumulated in a buffer with the specified
    /// size. When the buffer is full, the chunk is compressed and written to the output.
    /// The default is 4 MB.
    #[arg(long)]
    pub max_chunk_size: Option<String>,

    /// Place each chunk into its own compression chunk (or chunks, if the stream is large).
    #[arg(long)]
    pub one_stream_per_chunk: bool,

    /// After writing the PDZ file, close it, re-open it, and verify that its stream contents
    /// match the original PDB, byte for byte.
    #[arg(long)]
    pub verify: bool,
}

pub fn pdz_encode(options: PdzEncodeOptions) -> Result<()> {
    let _span = trace_span!("pdz_encode").entered();

    let input_file = RandomAccessFile::open(Path::new(&options.input_pdb))
        .with_context(|| format!("Failed to open input PDB: {}", options.input_pdb))?;

    use ms_pdb::taster::{Flavor, what_flavor};

    if let Ok(flavor) = what_flavor(&input_file) {
        if flavor != Some(Flavor::Pdb) {
            if options.copy_unrecognized {
                drop(input_file);
                std::fs::copy(&options.input_pdb, &options.output_pdz)?;
                return Ok(());
            } else {
                bail!("The input file is not a PDB: {}", options.input_pdb);
            }
        }
    }

    // Choose the maximum chunk size. We do this before creating the MsfzWriter so that we can
    // validate the input before creating an output file.
    let max_chunk_size: u32 = if let Some(s) = &options.max_chunk_size {
        parse_bytes(s)?
    } else {
        msfz::DEFAULT_CHUNK_THRESHOLD
    };

    let pdb = Msf::open(Path::new(&options.input_pdb))
        .with_context(|| format!("Failed to open input PDB: {}", options.input_pdb))?;

    let mut writer = MsfzWriter::create(Path::new(&options.output_pdz))
        .with_context(|| format!("Failed to open output PDZ: {}", options.output_pdz))?;

    writer.set_uncompressed_chunk_size_threshold(max_chunk_size);
    println!(
        "Using maximum chunk size: {n} ({n:#x})",
        n = writer.uncompressed_chunk_size_threshold()
    );

    let mut stream_data: Vec<u8> = Vec::new();
    let num_streams = pdb.num_streams();
    writer.reserve_num_streams(num_streams as usize);

    // The PDBI is a very important stream and we want to write its uncompressed form at the
    // very beginning of the output file, so that it can be easily found using the "initial read"
    // optimization.
    {
        let _span = trace_span!("transfer PDBI stream");
        pdb.read_stream_to_vec_mut(Stream::PDB.into(), &mut stream_data)?;
        let mut sw = writer.stream_writer(Stream::PDB.into())?;
        sw.set_compression_enabled(false);
        sw.write_all(&stream_data)?;
    }

    if options.one_stream_per_chunk {
        writer.end_chunk()?;
    }

    // Write the DBI stream (3).
    let dbi_header: Option<DbiStreamHeader>;
    {
        let _span = trace_span!("transfer DBI stream");
        pdb.read_stream_to_vec_mut(Stream::DBI.into(), &mut stream_data)?;
        let mut sw = writer.stream_writer(Stream::DBI.into())?;
        dbi_header = write_dbi(&mut sw, &stream_data)?;
        writer.end_chunk()?;
    }

    // Write the TPI (2).
    let tpi_header_opt: Option<tpi::TypeStreamHeader> = if pdb.is_stream_valid(Stream::TPI.into()) {
        let _span = trace_span!("transfer TPI stream");
        pdb.read_stream_to_vec_mut(Stream::TPI.into(), &mut stream_data)?;
        let mut sw = writer.stream_writer(Stream::TPI.into())?;
        write_tpi_or_ipi(&mut sw, &stream_data, max_chunk_size as usize)?
    } else {
        None
    };

    // Write the IPI (4)
    let ipi_header_opt: Option<tpi::TypeStreamHeader> = if pdb.is_stream_valid(Stream::IPI.into()) {
        let _span = trace_span!("transfer IPI stream");
        pdb.read_stream_to_vec_mut(Stream::IPI.into(), &mut stream_data)?;
        let mut sw = writer.stream_writer(Stream::IPI.into())?;
        write_tpi_or_ipi(&mut sw, &stream_data, max_chunk_size as usize)?
    } else {
        None
    };

    // Loop through the rest of the streams and do normal chunked compression.
    for stream_index in 1..num_streams {
        let _span = trace_span!("stream").entered();
        trace!(stream_index);

        if !pdb.is_stream_valid(stream_index) {
            // This is a nil stream. We don't need to do anything because the writer has already
            // reserved this stream slot.
            continue;
        }

        if stream_index == Stream::PDB.into()
            || stream_index == Stream::DBI.into()
            || stream_index == Stream::IPI.into()
            || stream_index == Stream::TPI.into()
        {
            // We have already processed these streams, above.
            continue;
        }

        // Custom encoding for the TPI Hash Stream
        if let Some(tpi_header) = &tpi_header_opt {
            if tpi_header.hash_stream_index.get() == Some(stream_index) {
                pdb.read_stream_to_vec_mut(stream_index, &mut stream_data)?;
                let mut sw = writer.stream_writer(stream_index)?;
                write_tpi_or_ipi_hash_stream(
                    &mut sw,
                    &stream_data,
                    max_chunk_size as usize,
                    &tpi_header,
                )?;
                writer.end_chunk()?;
                continue;
            }
        }

        // Custom encoding for the IPI Hash Stream
        if let Some(ipi_header) = &ipi_header_opt {
            if ipi_header.hash_stream_index.get() == Some(stream_index) {
                pdb.read_stream_to_vec_mut(stream_index, &mut stream_data)?;
                let mut sw = writer.stream_writer(stream_index)?;
                write_tpi_or_ipi_hash_stream(
                    &mut sw,
                    &stream_data,
                    max_chunk_size as usize,
                    &ipi_header,
                )?;
                writer.end_chunk()?;
                continue;
            }
        }

        // Custom encoding for the Global Symbol Stream.
        // The stream number for the Global Symbol Stream is found in the DBI Stream Header.
        match &dbi_header {
            Some(dbi_header) if dbi_header.global_symbol_stream.get() == Some(stream_index) => {
                let mut sw = writer.stream_writer(stream_index)?;
                pdb.read_stream_to_vec_mut(stream_index, &mut stream_data)?;
                write_global_symbols_stream(&mut sw, &stream_data, max_chunk_size as usize)?;
                writer.end_chunk()?;
                continue;
            }
            _ => {}
        }

        {
            let _span = trace_span!("read stream").entered();
            pdb.read_stream_to_vec_mut(stream_index, &mut stream_data)?;
            trace!(stream_size = stream_data.len());
        }

        {
            let _span = trace_span!("write stream").entered();
            let mut sw = writer.stream_writer(stream_index)?;
            sw.write_all(&stream_data)?;
        }

        if options.one_stream_per_chunk {
            writer.end_chunk()?;
        }
    }

    // Finish encoding the MSFZ file. This closes the session; don't append more data to the file
    // after this line. This writes the MSFZ Stream Directory, Chunk Table, and MSFZ File Header.
    let (summary, mut file) = {
        let _span = trace_span!("finish writing").entered();
        writer.finish_with_options(MsfzFinishOptions {
            min_file_size: if options.pad16k { MIN_FILE_SIZE_16K } else { 0 },
            stream_dir_compression: if options.compress_stream_dir {
                Some(msfz::Compression::Zstd)
            } else {
                None
            },
        })?
    };

    let out_file_size = file.seek(SeekFrom::End(0))?;

    match std::fs::metadata(&options.input_pdb) {
        Ok(pdb_metadata) => {
            show_comp_rate("PDB -> PDZ", pdb_metadata.len(), out_file_size);
        }
        Err(e) => {
            warn!("Failed to get metadata for input PDB: {e:?}");
        }
    }

    println!("{summary}");

    // Explicitly drop our output file handle so that we can re-open it for verification.
    drop(file);

    if options.verify {
        info!("Verifying PDZ encoding");
        let is_same = verify_pdz(&pdb, &options.output_pdz)?;
        if !is_same {
            bail!("PDZ encoding failed verification.");
        }
    }

    drop(pdb);

    Ok(())
}

/// Reads all of the data from two files and compares their contents.
///
/// The first file is a PDB (MSF) file and is already open.
/// The second file is a PDZ (MSFZ) file and its filename is given.
///
/// Returns `true` if their contents are identical.
fn verify_pdz(input_pdb: &Msf, output_pdz: &str) -> anyhow::Result<bool> {
    let output = msfz::Msfz::open(output_pdz)?;

    let input_num_streams = input_pdb.num_streams();
    let output_num_streams = output.num_streams();
    if input_num_streams != output_num_streams {
        error!(
            "The output file (PDZ) has the wrong number of streams. \
             Expected value: {input_num_streams}. \
             Actual value: {output_num_streams}"
        );
        bail!("Wrong number of streams");
    }

    let mut has_errors = false;

    let mut input_stream_data: Vec<u8> = Vec::new();
    let mut output_stream_data: Vec<u8> = Vec::new();

    for stream in 1..=input_num_streams {
        let input_stream_is_valid = input_pdb.is_stream_valid(stream);
        let output_stream_is_valid = output.is_stream_valid(stream);

        if input_stream_is_valid != output_stream_is_valid {
            error!(
                "Stream {stream} has wrong validity. \
                 Expected value: {input_stream_is_valid:?}. \
                 Actual value: {output_stream_is_valid:?}."
            );
            has_errors = true;
        }

        if !input_stream_is_valid {
            continue;
        }

        // Read stream data in the input file.
        {
            input_stream_data.clear();
            let mut sr = input_pdb.get_stream_reader(stream)?;
            sr.read_to_end(&mut input_stream_data)?;
        }

        // Read stream data in the output file.
        {
            output_stream_data.clear();
            let mut sr = output.get_stream_reader(stream)?;
            sr.read_to_end(&mut output_stream_data)?;
        }

        if input_stream_data.len() != output_stream_data.len() {
            error!(
                "Stream {stream} has wrong length. \
                    Expected value: {}. \
                    Actual value: {}.",
                input_stream_data.len(),
                output_stream_data.len()
            );
            has_errors = true;
            continue;
        }

        if let Some(byte_offset) =
            find_index_of_first_different_byte(&input_stream_data, &output_stream_data)
        {
            error!(
                "Stream {stream} has wrong (different) contents, at index {byte_offset} ({byte_offset:#x})"
            );
            has_errors = true;
            continue;
        }
    }

    if has_errors {
        return Ok(false);
    }

    info!("Verification succeeded.");

    Ok(true)
}

fn find_index_of_first_different_byte(mut a: &[u8], mut b: &[u8]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(0);
    }

    const BLOCK_SIZE: usize = 256;

    let mut skipped_len: usize = 0;
    loop {
        assert_eq!(a.len(), b.len());
        if a.len() < BLOCK_SIZE {
            break;
        }
        let block_a = &a[..BLOCK_SIZE];
        let block_b = &b[..BLOCK_SIZE];

        if block_a != block_b {
            break;
        }

        a = &a[BLOCK_SIZE..];
        b = &b[BLOCK_SIZE..];
        skipped_len += BLOCK_SIZE;
    }

    for i in 0..a.len() {
        if a[i] != b[i] {
            return Some(skipped_len + i);
        }
    }

    None
}

/// Write the DBI stream. Be smart about compression and compression chunk boundaries.
fn write_dbi(
    sw: &mut StreamWriter<'_, File>,
    stream_data: &[u8],
) -> Result<Option<DbiStreamHeader>> {
    // Avoid compressing data from the DBI stream with other chunks.
    sw.end_chunk()?;

    let Ok((dbi_header, mut rest_of_stream)) = DbiStreamHeader::read_from_prefix(stream_data)
    else {
        // Something is seriously wrong with this PDB. Pass the contents through without any
        // modification or compression.
        sw.set_compression_enabled(false);
        sw.write_all(stream_data)?;
        sw.end_chunk()?;
        return Ok(None);
    };

    // Write the DBI Stream Header uncompressed. This allows symbol.exe to read it.
    sw.set_compression_enabled(false);
    sw.write_all(dbi_header.as_bytes())?;
    sw.end_chunk()?;

    // The DBI consists of a header, followed by a set of substreams. The substreams contain
    // data with different sizes, compression characteristic, and different access patterns.
    //
    // We attempt to break up the rest of the stream data and handle each substream individually.
    // If we find a substream size that doesn't make sense (is negative or exceeds the size of
    // the remaining data in the stream) then we just bail and write the rest of the data without
    // doing any more chunking.

    'fallback: {
        macro_rules! get_next_substream {
            ($substream_len_field:ident) => {
                {
                    let Ok(len_u32) = u32::try_from(dbi_header.$substream_len_field.get()) else {
                        warn!("DBI stream is invalid; the substream {} has a negative length", stringify!($substream_len_field));
                        break 'fallback;
                    };
                    let len_usize = len_u32 as usize;
                    if rest_of_stream.len() < len_usize {
                        warn!("DBI stream is invalid; the substream {} has a length that exceeds the size of the remaining stream data.", stringify!($substream_len_field));
                        break 'fallback;
                    }
                    let (lo, hi) = rest_of_stream.split_at(len_usize);
                    rest_of_stream = hi;
                    lo
                }
            }
        }

        // Module Info
        let modules = get_next_substream!(mod_info_size);
        sw.set_compression_enabled(true);
        sw.write_all(modules)?;
        sw.end_chunk()?;

        // The "Section Contributions" substream is very large and is not often used, so we place
        // it in its own chunk, too.
        let section_contributions = get_next_substream!(section_contribution_size);
        sw.set_compression_enabled(true);
        sw.write_all(section_contributions)?;
        sw.end_chunk()?;

        // The "Section Map" is very small. We write it without compression.
        let section_map = get_next_substream!(section_map_size);
        sw.set_compression_enabled(false);
        sw.write_all(section_map)?;
        sw.end_chunk()?;

        // The "Sources" substream is very commonly accessed and medium-sized. We store it in its
        // own chunk.
        let sources = get_next_substream!(source_info_size);
        sw.set_compression_enabled(true);
        sw.write_all(sources)?;
        sw.end_chunk()?;

        // The "Type Server Map", "Optional Debug Headers", and "Edit-and-Continue" substreams are
        // typically very small and rarely accessed. We store them compressed.
    }

    // If we got here, then either something is wrong with the contents of the stream, or we just
    // reached the last few substreams, and we don't do anything special with them. Write the rest
    // of the data.
    sw.set_compression_enabled(true);
    sw.write_all(rest_of_stream)?;
    sw.end_chunk()?;

    Ok(Some(dbi_header))
}

/// This is the fallback path for writing complex streams.
///
/// If we find any problem in writing a complex stream, we fall back to compressing all of it.
#[inline(never)]
fn write_stream_fallback(sw: &mut StreamWriter<'_, File>, stream_data: &[u8]) -> Result<()> {
    sw.set_compression_enabled(true);
    sw.write_all(stream_data)?;
    sw.end_chunk()?;
    return Ok(());
}

fn write_global_symbols_stream(
    sw: &mut StreamWriter<'_, File>,
    stream_data: &[u8],
    mut max_chunk_len: usize,
) -> Result<()> {
    info!("Writing Global Symbol Stream");

    sw.end_chunk()?;

    // The code below assumes that you can always put at least one type record into a chunk.
    // To prevent a forward-progress failure, we require that max_chunk_len is larger than the
    // largest type record.
    const MIN_CHUNK_LEN: usize = 0x20000;
    if max_chunk_len < MIN_CHUNK_LEN {
        warn!(
            "max_chunk_len ({}) is way too small; promoting it",
            max_chunk_len
        );
        max_chunk_len = MIN_CHUNK_LEN;
    }

    let mut current_chunk_bytes: &[u8] = stream_data;
    let mut total_bytes_written: usize = 0;

    'top: while !current_chunk_bytes.is_empty() {
        let mut iter = SymIter::new(current_chunk_bytes);
        loop {
            let rest_len_before = iter.rest_len();
            if iter.next().is_none() {
                break 'top;
            }
            let rest_len_after = iter.rest_len();

            let chunk_len_with_this_record = current_chunk_bytes.len() - rest_len_after;
            if chunk_len_with_this_record > max_chunk_len {
                let chunk_len_without_this_record = current_chunk_bytes.len() - rest_len_before;
                let (committed_chunk_bytes, next_chunk_bytes) =
                    current_chunk_bytes.split_at(chunk_len_without_this_record);

                // TODO: This could be optimized to a single, non-buffered chunk write.
                sw.write_all(committed_chunk_bytes)?;
                sw.end_chunk()?;
                total_bytes_written += committed_chunk_bytes.len();

                // This will cause us to re-parse a record at the start of the next chunk.
                // That's ok, that's cheap.  But we do need to handle the case where a record
                // is larger than max_chunk_len.  We "handle" that by requiring that max_chunk_len
                // is at least 0x10004, since that is the maximum size for any record.  That's a
                // very silly lower bound for a chunk size, so we actually require it to be higher.
                current_chunk_bytes = next_chunk_bytes;
                continue 'top;
            }
        }
    }

    if !current_chunk_bytes.is_empty() {
        sw.write_all(current_chunk_bytes)?;
        sw.end_chunk()?;
        total_bytes_written += current_chunk_bytes.len();
    }

    assert_eq!(
        total_bytes_written,
        stream_data.len(),
        "expected to write same number of record bytes"
    );

    Ok(())
}

/// Write the TPI or IPI stream. Be smart about compression and compression chunk boundaries.
///
/// This function returns `Some(header)` if the TPI header was correctly parsed. This header can
/// be used to optimize the encoding of the associated Type Hash Stream.
fn write_tpi_or_ipi(
    sw: &mut StreamWriter<'_, File>,
    stream_data: &[u8],
    mut max_chunk_len: usize,
) -> Result<Option<tpi::TypeStreamHeader>> {
    debug!("write_tpi_or_ipi");
    sw.end_chunk()?;

    // The code below assumes that you can always put at least one type record into a chunk.
    // To prevent a forward-progress failure, we require that max_chunk_len is larger than the
    // largest type record.
    const MIN_CHUNK_LEN: usize = 0x20000;
    if max_chunk_len < MIN_CHUNK_LEN {
        warn!("max_chunk_len is way too small; promoting it");
        max_chunk_len = MIN_CHUNK_LEN;
    }

    // If the stream does not even contain a full header, then fall back to writing full contents.
    let Ok((tpi_header, after_header)) = tpi::TypeStreamHeader::read_from_prefix(stream_data)
    else {
        warn!("TPI or IPI stream was too short to contain a valid header");
        write_stream_fallback(sw, stream_data)?;
        return Ok(None);
    };

    // Find the slice of the type data. There can be data following the type data and we must
    // handle it correctly.
    let type_record_bytes_len = tpi_header.type_record_bytes.get() as usize;
    if after_header.len() < type_record_bytes_len {
        warn!(
            "TPI or IPI stream contained invalid header value (type_record_bytes exceeded bounds)"
        );
        write_stream_fallback(sw, stream_data)?;
        return Ok(None);
    }

    debug!(type_record_bytes_len, "encoding TPI/IPI.");

    // type_record_bytes contains the encoded type records
    // after_records contains unknown data (if any) after the type records
    let (type_record_bytes, after_records) = after_header.split_at(type_record_bytes_len);

    // Write the header, without compression.
    // TODO: Place this in the initial read section.
    sw.set_compression_enabled(false);
    sw.write_all(tpi_header.as_bytes())?;

    // Next, we are going to scan through the type records in the TPI. Our goal is to create chunk
    // boundaries that align with record boundaries, so that no type record is split across chunks.
    sw.set_compression_enabled(true);

    // current_chunk_bytes contains the type records that will be written into the next chunk.
    let mut current_chunk_bytes: &[u8] = type_record_bytes;

    let mut record_bytes_written: usize = 0;

    // This loop runs once per "chunk". Each iteration builds a single MSFZ chunk from a
    // sequence of contiguous type records. Type records never cross chunk boundaries.
    'top: while !current_chunk_bytes.is_empty() {
        let mut iter = TypesIter::new(current_chunk_bytes);
        loop {
            let rest_len_before = iter.rest_len();
            if iter.next().is_none() {
                break 'top;
            }

            let rest_len_after = iter.rest_len();
            let record_len = rest_len_before - rest_len_after;
            assert!(record_len <= current_chunk_bytes.len());

            // Would adding this record to the current chunk exceed our threshold?
            let chunk_len_with_this_record = current_chunk_bytes.len() - rest_len_after;
            if chunk_len_with_this_record > max_chunk_len {
                let chunk_len_without_this_record = current_chunk_bytes.len() - rest_len_before;
                let (committed_chunk_bytes, next_chunk_bytes) =
                    current_chunk_bytes.split_at(chunk_len_without_this_record);

                // TODO: This could be optimized to a single, non-buffered chunk write.
                sw.write_all(committed_chunk_bytes)?;
                sw.end_chunk()?;
                record_bytes_written += committed_chunk_bytes.len();

                // This will cause us to re-parse a record at the start of the next chunk.
                // That's ok, that's cheap.  But we do need to handle the case where a record
                // is larger than max_chunk_len.  We "handle" that by requiring that max_chunk_len
                // is at least 0x10004, since that is the maximum size for any record.  That's a
                // very silly lower bound for a chunk size, so we actually require it to be higher.
                current_chunk_bytes = next_chunk_bytes;
                continue 'top;
            }

            // Keep processing records.
        }
    }

    // If we got here, then the iterator stopped reporting records. That can happen for two
    // reasons: 1) the normal case where we reach the end of the types, or 2) we failed to
    // decode a type record.  In both cases, writing current_chunk_contents will write the
    // prefix of records that have been parsed (but have not triggered our threshold
    if !current_chunk_bytes.is_empty() {
        // TODO: optimize to a single, non-buffered chunk write.
        sw.write_all(current_chunk_bytes)?;
        record_bytes_written += current_chunk_bytes.len();
    }
    sw.end_chunk()?;
    assert_eq!(
        record_bytes_written,
        type_record_bytes.len(),
        "expected to write same number of record bytes"
    );

    if !after_records.is_empty() {
        debug!(
            after_records_len = after_records.len(),
            "TPI/IPI contains data after type stream"
        );
        sw.write_all(after_records)?;
    }

    sw.end_chunk()?;

    Ok(Some(tpi_header))
}

/// Write the "Type Hash Stream" associated with the TPI or IPI stream.
/// Be smart about compression and compression chunk boundaries.
///
/// This function requires the Type Stream Header from the original TPI or IPI stream.
/// That header describes the regions within the Type Stream Header.
///
/// The Type Hash Stream consists of three regions: 1) Hash Value Buffer, 2) Index Offset Buffer,
/// and 3) Hash Adjustment Buffer.  The size and location of each of these regions is specified
/// in fields in the Type Stream Header.
///
/// We use a simple algorithm. We build a list of the start and end locations of each of these
/// buffers (if they are non-zero length). Then we sort the list and de-dup it.  Then we traverse
/// the list, writing chunks for each region.
fn write_tpi_or_ipi_hash_stream(
    sw: &mut StreamWriter<'_, File>,
    stream_data: &[u8],
    max_chunk_len: usize,
    tpi_header: &tpi::TypeStreamHeader,
) -> Result<()> {
    sw.end_chunk()?;

    let stream_len_u32 = stream_data.len() as u32;

    let mut boundaries: Vec<usize> = Vec::with_capacity(8);

    boundaries.push(stream_data.len());

    let regions: [(i32, u32); 3] = [
        (
            tpi_header.hash_value_buffer_offset.get(),
            tpi_header.hash_value_buffer_length.get(),
        ),
        (
            tpi_header.index_offset_buffer_offset.get(),
            tpi_header.index_offset_buffer_length.get(),
        ),
        (
            tpi_header.hash_adj_buffer_offset.get(),
            tpi_header.hash_adj_buffer_length.get(),
        ),
    ];

    for &(start, length) in regions.iter() {
        if length == 0 {
            continue;
        }
        if start < 0 {
            warn!("Type Hash Stream has a negative offset for one of its regions");
            continue;
        }

        let start_u: u32 = start as u32;
        if start_u > stream_len_u32 {
            warn!("Type Hash Stream has a region whose start offset is out of bounds");
            continue;
        }

        let avail = stream_len_u32 - start_u;
        if length > avail {
            warn!("Type Hash Stream has a region whose end offset is out of bounds");
            continue;
        }

        let end: u32 = start_u + length;

        boundaries.push(start_u as usize);
        boundaries.push(end as usize);
    }

    boundaries.sort_unstable();
    boundaries.dedup();

    info!("Type Hash Stream offset boundaries: {:?}", boundaries);

    let mut pos: usize = 0;
    let mut total_bytes_written: usize = 0;

    for &boundary_offset in boundaries.iter() {
        assert!(boundary_offset >= pos);
        if boundary_offset == pos {
            continue;
        }

        let start = pos;
        let end = boundary_offset;
        pos = end;

        let data_between_boundaries = &stream_data[start..end];

        // We are going to _further_ chunk things, based on max_chunk_len.
        for chunk_data in data_between_boundaries.chunks(max_chunk_len) {
            sw.write_all(chunk_data)?;
            sw.end_chunk()?;
            total_bytes_written += chunk_data.len();
        }
    }

    assert_eq!(
        total_bytes_written,
        stream_data.len(),
        "expected to write the correct number of bytes"
    );

    Ok(())
}

fn parse_bytes(mut bytes_str: &str) -> anyhow::Result<u32> {
    let mut units: u32 = 1;

    if let Some(s) = bytes_str.strip_suffix(['k', 'K']) {
        units = 1024;
        bytes_str = s;
    }

    if let Some(s) = bytes_str.strip_suffix(['m', 'M']) {
        units = 1048576;
        bytes_str = s;
    }

    let n: u32 = bytes_str.parse()?;
    if let Some(n_scaled) = n.checked_mul(units) {
        Ok(n_scaled)
    } else {
        bail!("Size is too large")
    }
}
