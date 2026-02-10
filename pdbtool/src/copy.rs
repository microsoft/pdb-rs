use ms_pdb::Pdb;
use ms_pdb::msf::{CreateOptions, PageSize};
use std::io::Write;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Options {
    /// The PDB to read.
    source_pdb: PathBuf,

    /// The PDB to write.
    dest_pdb: PathBuf,

    /// The size in bytes of the pages to use. Must be a power of two.
    #[arg(long)]
    page_size: Option<u32>,
}

pub fn copy_command(options: &Options) -> anyhow::Result<()> {
    let src = Pdb::open(&options.source_pdb)?;
    let mut create_options = CreateOptions::default();

    if let Some(page_size) = options.page_size {
        create_options.page_size = PageSize::try_from(page_size)
            .map_err(|_| anyhow::anyhow!("The page size must be a power of 2."))?;
    }

    let mut dst = ms_pdb::msf::Msf::create(&options.dest_pdb, create_options)?;

    for stream_index in 1..src.num_streams() {
        if src.is_stream_valid(stream_index) {
            let stream_data = src.read_stream_to_vec(stream_index)?;
            let mut s = dst.write_stream(stream_index)?;
            s.write_all(&stream_data)?;
        }
    }

    dst.commit()?;

    Ok(())
}
