use crate::pdz::util::*;
use anyhow::{bail, Context, Result};
use ms_pdb::dbi::{DbiStreamHeader, DBI_STREAM_HEADER_LEN};
use ms_pdb::msf::Msf;
use ms_pdb::msfz::{MsfzFinishOptions, MsfzWriter, StreamWriter, MIN_FILE_SIZE_16K};
use ms_pdb::{RandomAccessFile, Stream};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tracing::{trace, trace_span, warn};
use zerocopy::FromBytes;

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
}

pub fn pdz_encode(options: PdzEncodeOptions) -> Result<()> {
    let _span = trace_span!("pdz_encode").entered();

    let pdb_metadata = std::fs::metadata(&options.input_pdb).with_context(|| {
        format!(
            "Failed to get metadata for input PDB: {}",
            options.input_pdb
        )
    })?;

    let input_file = RandomAccessFile::open(Path::new(&options.input_pdb))
        .with_context(|| format!("Failed to open input PDB: {}", options.input_pdb))?;

    use ms_pdb::taster::{what_flavor, Flavor};

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

    let pdb = Msf::open(Path::new(&options.input_pdb))
        .with_context(|| format!("Failed to open input PDB: {}", options.input_pdb))?;

    let mut writer = MsfzWriter::create(Path::new(&options.output_pdz))
        .with_context(|| format!("Failed to open output PDZ: {}", options.output_pdz))?;

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

    // Next, write the DBI stream.
    {
        let _span = trace_span!("transfer DBI stream");
        pdb.read_stream_to_vec_mut(Stream::DBI.into(), &mut stream_data)?;
        let mut sw = writer.stream_writer(Stream::DBI.into())?;
        write_dbi(&mut sw, &stream_data)?;
    }

    // Loop through the rest of the streams and do normal chunked compression.
    for stream_index in 1..num_streams {
        if !pdb.is_stream_valid(stream_index) {
            // This is a nil stream. We don't need to do anything because the writer has already
            // reserved this stream slot.
            continue;
        }

        if stream_index == Stream::PDB.into() || stream_index == Stream::DBI.into() {
            // We have already processed these streams, above.
            continue;
        }

        let _span = trace_span!("stream").entered();
        trace!(stream_index);

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
    }

    // Get final size of the file. Don't append more data to the file after this line.
    let (summary, mut file) = {
        let _span = trace_span!("finish writing").entered();
        writer.finish_with_options(MsfzFinishOptions {
            min_file_size: if options.pad16k { MIN_FILE_SIZE_16K } else { 0 },
            stream_dir_compression: if options.compress_stream_dir {
                Some(ms_pdb::msfz::Compression::Zstd)
            } else {
                None
            },
        })?
    };

    let out_file_size = file.seek(SeekFrom::End(0))?;
    show_comp_rate("PDB -> PDZ", pdb_metadata.len(), out_file_size);

    println!("{}", summary);

    // Explicitly close our file handles so that the replace_file() call can succeed.
    drop(pdb);
    drop(file);

    Ok(())
}

/// Write the DBI stream. Be smart about compression and compression chunk boundaries.
fn write_dbi(sw: &mut StreamWriter<'_, File>, stream_data: &[u8]) -> Result<()> {
    // Avoid compressing data from the DBI stream with other chunks.
    sw.end_chunk()?;

    if stream_data.len() < DBI_STREAM_HEADER_LEN {
        // Something is seriously wrong with this PDB. Pass the contents through without any
        // modification or compression.
        sw.set_compression_enabled(false);
        sw.write_all(stream_data)?;
        return Ok(());
    }

    let (header_bytes, mut rest_of_stream) = stream_data.split_at(DBI_STREAM_HEADER_LEN);

    // This unwrap() cannot fail because we just tested the size, above.
    let dbi_header = DbiStreamHeader::ref_from_bytes(header_bytes).unwrap();

    // Write the DBI Stream Header uncompressed. This allows symbol.exe to read it.
    sw.set_compression_enabled(false);
    sw.write_all(header_bytes)?;

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

    Ok(())
}
