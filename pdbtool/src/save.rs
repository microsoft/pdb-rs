use anyhow::bail;
use mspdb::{names::NAMES_STREAM_NAME, Pdb, Stream};
use std::ops::Range;
use std::path::Path;

#[derive(clap::Parser)]
pub struct SaveStreamOptions {
    /// The PDB file to read.
    pdb: String,

    /// The index or name of the stream to save. Name can be one of: pdb, dbi, gsi, gss, tpi, ipi
    stream: String,

    /// The path to save the stream to.
    out: String,
}

pub fn save_stream(options: &SaveStreamOptions) -> anyhow::Result<()> {
    let reader = Pdb::open(Path::new(&options.pdb))?;

    // Support saving substreams of just a handful of streams. This could be made more general.
    if options.stream == "dbi/sources" {
        let dbi = reader.read_dbi_stream()?;
        std::fs::write(&options.out, dbi.source_info())?;
        return Ok(());
    }

    if options.stream == "dbi/section_contributions" {
        let dbi = reader.read_dbi_stream()?;
        std::fs::write(&options.out, dbi.section_contributions_bytes())?;
        return Ok(());
    }

    let (stream_index, stream_range_opt) = get_stream_index(&reader, &options.stream)?;
    let stream_data = reader.read_stream_to_vec(stream_index)?;
    let stream_slice = if let Some(stream_range) = stream_range_opt {
        if let Some(s) = stream_data.get(stream_range.clone()) {
            s
        } else {
            bail!(
                "The stream range 0x{:x}..0x{:x} is out of range. Stream length = 0x{:x}.",
                stream_range.start,
                stream_range.end,
                stream_data.len()
            );
        }
    } else {
        stream_data.as_slice()
    };
    std::fs::write(&options.out, stream_slice)?;
    Ok(())
}

pub fn get_stream_index(reader: &Pdb, name: &str) -> anyhow::Result<(u32, Option<Range<usize>>)> {
    if let Ok(stream_index) = name.parse::<u32>() {
        return Ok((stream_index, None));
    }

    if let Some(i) = get_fixed_stream(name) {
        return Ok((i, None));
    }

    let dbi_header = reader.dbi_header();

    if let Some(suffix) = name.strip_prefix("named:") {
        if let Some(s) = reader.named_streams().get(suffix) {
            return Ok((s, None));
        } else {
            bail!("There is no named stream with that name.");
        }
    }

    if let Some(suffix) = name.strip_prefix("mod:") {
        let index: usize = suffix.parse()?;
        let modules = reader.read_modules()?;
        if let Some(module) = modules.iter().nth(index) {
            if let Some(s) = module.stream() {
                return Ok((s, None));
            } else {
                bail!("Module {} does not have a stream.", index);
            }
        } else {
            bail!("Module index {} is out of valid range.", index);
        }
    }

    Ok(match name {
        "gss" => (dbi_header.sym_record_stream()?, None),
        "psi" => (dbi_header.public_stream_index()?, None),
        "gsi" => (dbi_header.global_stream_index()?, None),
        "names" => (reader.named_stream_err(NAMES_STREAM_NAME)?, None),
        "dbi/sources" => (Stream::DBI.into(), Some(dbi_header.sources_range()?)),
        "dbi/modules" => (Stream::DBI.into(), Some(dbi_header.modules_range()?)),
        _ => {
            if let Some(s) = reader.named_streams().get(name) {
                return Ok((s, None));
            }
            bail!("The name '{}' does not identify any known stream.", name);
        }
    })
}

fn get_fixed_stream(name: &str) -> Option<u32> {
    match name {
        "pdb" => Some(Stream::PDB.into()),
        "dbi" => Some(Stream::DBI.into()),
        "tpi" => Some(Stream::TPI.into()),
        "ipi" => Some(Stream::IPI.into()),
        _ => None,
    }
}
