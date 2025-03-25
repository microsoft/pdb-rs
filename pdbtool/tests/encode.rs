use std::{path::Path, process::Command};

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const PDBTOOL: &str = env!("CARGO_BIN_EXE_pdbtool");

#[test]
fn min_size_workaround() {
    let dir = Path::new(TMP_DIR).join("min_size_workaround");
    _ = std::fs::create_dir_all(&dir);

    let pdb_file_name = dir.join("test.pdb");
    let pdz_file_name = dir.join("test.pdz");

    // Create a very small test.pdb file
    {
        let mut pdb = ms_pdb::msf::Msf::create(&pdb_file_name, Default::default()).unwrap();
        let mut sw = pdb.write_stream(10).unwrap();
        sw.write_all_at_mut(b"Hello, world!", 0).unwrap();
        pdb.commit().unwrap();
    }

    // Directly read the PDB file
    {
        let pdb = ms_pdb::msf::Msf::open(&pdb_file_name).unwrap();
        let stream_contents = pdb.read_stream_to_vec(10).unwrap();
        assert_eq!(stream_contents.as_slice(), b"Hello, world!");
    }

    // Convert the PDB file to a PDZ file.
    {
        let mut cmd = Command::new(PDBTOOL);
        cmd.arg("--verbose");
        cmd.arg("pdz-encode");
        cmd.arg(&pdb_file_name);
        cmd.arg(&pdz_file_name);
        cmd.arg("--pad16k");
        assert!(cmd.status().unwrap().success());
    }

    // Read the PDZ file and verify the contents of the stream.
    {
        let pdz = ms_pdb::msfz::Msfz::open(&pdz_file_name).unwrap();
        let stream_contents = pdz.read_stream(10).unwrap();
        assert_eq!(stream_contents.as_slice(), b"Hello, world!");
    }
}
