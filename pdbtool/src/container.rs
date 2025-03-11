use anyhow::Result;
use std::path::Path;

#[derive(clap::Parser)]
pub struct ContainerOptions {
    /// The PDB file or PDZ file to read.
    pdb: String,
}

pub fn container_command(options: &ContainerOptions) -> Result<()> {
    let pdb = ms_pdb::Pdb::open(Path::new(&options.pdb))?;

    let container = pdb.container();
    match container {
        ms_pdb::Container::Msf(msf) => {
            println!("Container format: MSF (uncompressed)");
            println!("  Number of streams:           {:8}", pdb.num_streams());
            println!(
                "  Page size:                   {:8} bytes per page",
                u32::from(msf.page_size())
            );
            println!("  Number of pages:             {:8}", msf.nominal_size());
            println!("  Number of free pages:        {:8}", msf.num_free_pages());
        }

        ms_pdb::Container::Msfz(msfz) => {
            println!("Container format: MSFZ (compressed)");
            println!("  Number of streams:           {:8}", pdb.num_streams());
            println!("  Number of compressed chunks: {:8}", msfz.num_chunks());
            println!("  Number of stream fragments:  {:8}", msfz.num_fragments());
        }
    }

    Ok(())
}
