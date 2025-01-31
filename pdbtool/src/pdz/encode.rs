use crate::pdz::util::*;
use anyhow::{Context, Result};
use ms_pdb::msfz::MsfzWriter;
use ms_pdb::{Pdb, Stream};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use tracing::{trace, trace_span};

#[derive(clap::Parser)]
pub struct PdzEncodeOptions {
    /// Path to the input PDB file.
    pub input_pdb: String,

    /// Path to the output PDZ file.
    pub output_pdz: String,
}

pub fn pdz_encode(options: PdzEncodeOptions) -> Result<()> {
    let _span = trace_span!("pdz_encode").entered();

    let pdb_metadata = std::fs::metadata(&options.input_pdb).with_context(|| {
        format!(
            "Failed to get metadata for input PDB: {}",
            options.input_pdb
        )
    })?;
    let pdb = Pdb::open(Path::new(&options.input_pdb))
        .with_context(|| format!("Failed to open input PDB: {}", options.input_pdb))?;
    let out = File::create(&options.output_pdz)
        .with_context(|| format!("Failed to open output PDZ: {}", options.output_pdz))?;

    let mut writer = MsfzWriter::new(out)?;
    let mut stream_data: Vec<u8> = Vec::new();
    let num_streams = pdb.num_streams();
    writer.reserve_num_streams(num_streams as usize);

    for stream_index in 1..num_streams {
        if !pdb.is_stream_valid(stream_index) {
            // This is a nil stream. We don't need to do anything because the writer has already
            // reserved this stream slot.
            continue;
        }

        let _span = trace_span!("stream").entered();
        trace!(stream_index);

        stream_data.clear();
        {
            let _span = trace_span!("read stream").entered();
            pdb.read_stream_to_vec_mut(stream_index, &mut stream_data)?;
            trace!(stream_size = stream_data.len());
        }

        {
            let _span = trace_span!("write stream").entered();
            let mut sw = writer.stream_writer(stream_index)?;

            // Don't compress the PDBI. The PDBI is very small, so compression is useless, and this
            // exercises the non-compressed option. It also makes it possible to read the PDBI in
            // a hex dump.
            if stream_index == Stream::PDB.into() {
                sw.set_compression_enabled(false);
            }

            sw.write_all(&stream_data)?;
        }
    }

    // Get final size of the file. Don't append more data to the file after this line.
    let (summary, mut file) = {
        let _span = trace_span!("finish writing").entered();
        writer.finish()?
    };

    let out_file_size = file.seek(SeekFrom::End(0))?;
    show_comp_rate("PDB -> PDZ", pdb_metadata.len(), out_file_size);

    println!("{}", summary);

    // Explicitly close our file handles so that the replace_file() call can succeed.
    drop(pdb);
    drop(file);

    Ok(())
}
