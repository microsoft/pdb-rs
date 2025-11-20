use ms_pdb::Pdb;
use std::io::Write;
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct Options {
    /// The PDB to read.
    source_pdb: PathBuf,

    /// The PDB to write.
    dest_pdb: PathBuf,
}

pub fn copy_command(options: &Options) -> anyhow::Result<()> {
    if options.source_pdb.to_string_lossy().eq_ignore_ascii_case(&options.dest_pdb.to_string_lossy())
    {
        anyhow::bail!("`source_pdb` and `dest_pdb` must be different paths");
    }

    let src = Pdb::open(&options.source_pdb)?;
    let mut dst = ms_pdb::msf::Msf::create(&options.dest_pdb, Default::default())?;

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

#[test]
fn copy_errors_when_paths_differ_only_by_case() {
    let mut dir = std::env::temp_dir();
    dir.push("pdbtool_copy_test");
    let _ = std::fs::create_dir_all(&dir);

    let input = dir.join("same.pdb");
    let output = dir.join("Same.pdb");

    let opts = Options {
        source_pdb: PathBuf::from(&input),
        dest_pdb: PathBuf::from(&output),
    };

    let res = copy_command(&opts);
    assert!(
        res.is_err(),
        "expected error when paths differ only by case"
    );
    let msg = res.unwrap_err().to_string();
    assert!(
        msg.contains("must be different"),
        "unexpected error message: {}",
        msg
    );
}
