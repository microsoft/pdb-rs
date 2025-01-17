use mspdb::Pdb;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct Options {
    /// The PDB to read.
    source_pdb: PathBuf,

    /// The PDB to write.
    dest_pdb: PathBuf,
}

pub fn copy_command(options: &Options) -> anyhow::Result<()> {
    let src = Pdb::open(&options.source_pdb)?;
    let mut dst = mspdb::msf::Msf::create(&options.dest_pdb, Default::default())?;

    for stream_index in 1..src.num_streams() {
        if src.is_stream_valid(stream_index) {
            let stream_data = src.read_stream_to_vec(stream_index)?;
            let (_, mut s) = dst.new_stream()?;
            s.write_all(&stream_data)?;
        } else {
            let dst_stream_index = dst.nil_stream()?;
            assert_eq!(dst_stream_index, stream_index);
        }
    }

    dst.commit()?;

    Ok(())
}
