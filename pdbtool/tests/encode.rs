use ms_pdb::dbi::{DbiStreamHeader, DBI_STREAM_HEADER_LEN, DBI_STREAM_VERSION_V110};
use ms_pdb::msf::Msf;
use ms_pdb::msfz::Msfz;
use ms_pdb::{Stream, StreamIndexU16};
use std::io::Write;
use std::path::Path;
use std::process::Command;
use zerocopy::{FromBytes, IntoBytes};

const TMP_DIR: &str = env!("CARGO_TARGET_TMPDIR");
const PDBTOOL: &str = env!("CARGO_BIN_EXE_pdbtool");

#[track_caller]
fn run_command(mut cmd: Command) {
    let mut s = String::new();
    s.push_str(cmd.get_program().to_str().unwrap());
    for arg in cmd.get_args() {
        s.push(' ');
        s.push_str(arg.to_str().unwrap());
    }

    println!("Running: {s}");

    let status = cmd.status().expect("Failed to execute command");

    if !status.success() {
        panic!("Command failed: {}", status.code().unwrap());
    }

    println!();
}

#[test]
fn min_size_workaround() {
    let dir = Path::new(TMP_DIR).join("min_size_workaround");
    _ = std::fs::create_dir_all(&dir);

    let pdb_file_name = dir.join("test.pdb");
    let pdz_file_name = dir.join("test.pdz");

    // Create a very small test.pdb file
    {
        let mut pdb = Msf::create(&pdb_file_name, Default::default()).unwrap();
        let mut sw = pdb.write_stream(10).unwrap();
        sw.write_all_at_mut(b"Hello, world!", 0).unwrap();
        pdb.commit().unwrap();
    }

    // Directly read the PDB file
    {
        let pdb = Msf::open(&pdb_file_name).unwrap();
        let stream_contents = pdb.read_stream_to_vec(10).unwrap();
        assert_eq!(stream_contents.as_slice(), b"Hello, world!");
    }

    // Convert the PDB file to a PDZ file.
    {
        let mut cmd = Command::new(PDBTOOL);
        cmd.arg("pdz-encode");
        cmd.arg(&pdb_file_name);
        cmd.arg(&pdz_file_name);
        cmd.arg("--pad16k");
        assert!(cmd.status().unwrap().success());
    }

    // Read the PDZ file and verify the contents of the stream.
    {
        let pdz = Msfz::open(&pdz_file_name).unwrap();
        let stream_contents = pdz.read_stream(10).unwrap();
        assert_eq!(stream_contents.as_slice(), b"Hello, world!");
    }
}

fn make_good_dbi() -> Vec<u8> {
    // Make up some subsections.  The \0s are there to pad to multiples of 4.
    let module_info = b"I am the module info!\0\0\0";
    let section_contributions = b"This is totally a section contributions substream!\0\0";
    let section_map = b"What if this were a section map?";
    let source_info = b"It would be awesome if this was a source info substream.";
    let type_server_map = b"If I were a type server map, where would I be?\0\0";
    let optional_dbg_header = b"You get a debug header! And you get a debug header!\0";
    let edit_and_continue = b"I can barely edit at all, much less edit and continue!\0\0";

    // Make up a reasonable DBI stream header.
    let good_dbi_header = DbiStreamHeader {
        signature: (-1).into(),
        version: DBI_STREAM_VERSION_V110.into(),
        age: 1.into(),
        global_symbol_index_stream: StreamIndexU16::NIL,
        build_number: 0.into(),
        public_symbol_index_stream: StreamIndexU16::NIL,
        pdb_dll_version: 0.into(),
        global_symbol_stream: StreamIndexU16::NIL,
        pdb_dll_rbld: 0.into(),
        mod_info_size: (module_info.len() as i32).into(),
        section_contribution_size: (section_contributions.len() as i32).into(),
        section_map_size: (section_map.len() as i32).into(),
        source_info_size: (source_info.len() as i32).into(),
        type_server_map_size: (type_server_map.len() as i32).into(),
        mfc_type_server_index: 0.into(),
        optional_dbg_header_size: (optional_dbg_header.len() as i32).into(),
        edit_and_continue_size: (edit_and_continue.len() as i32).into(),
        flags: 0.into(),
        machine: 0.into(),
        padding: 0.into(),
    };

    let mut good_dbi: Vec<u8> = Vec::new();
    good_dbi.extend_from_slice(good_dbi_header.as_bytes());
    good_dbi.extend_from_slice(module_info);
    good_dbi.extend_from_slice(section_contributions);
    good_dbi.extend_from_slice(section_map);
    good_dbi.extend_from_slice(source_info);
    good_dbi.extend_from_slice(type_server_map);
    good_dbi.extend_from_slice(optional_dbg_header);
    good_dbi.extend_from_slice(edit_and_continue);

    good_dbi
}

// Test roundtrip with a PDB with a reasonable DBI stream
#[test]
fn dbi_good() {
    let dbi = make_good_dbi();
    dbi_case("good", &dbi);
}

// Test roundtrip with a DBI that is too small to be valid.
#[test]
fn dbi_header_too_small() {
    dbi_case("dbi_header_too_small", b"this is bad");
}

// Test roundtrip with a DBI that has a valid header, but is cut off within the Module Info substream.
#[test]
fn dbi_cut_off_in_module_info() {
    let dbi = make_good_dbi();
    dbi_case(
        "dbi_cut_off_in_module_info",
        &dbi[..DBI_STREAM_HEADER_LEN + 10],
    );
}

#[test]
fn dbi_cut_off_in_section_contributions() {
    let dbi = make_good_dbi();
    let header = DbiStreamHeader::ref_from_prefix(&dbi).unwrap().0;
    dbi_case(
        "dbi_cut_off_in_section_contributions",
        &dbi[..DBI_STREAM_HEADER_LEN + header.mod_info_size.get() as usize + 4],
    );
}

#[test]
fn dbi_cut_off_in_section_map() {
    let dbi = make_good_dbi();
    let header = DbiStreamHeader::ref_from_prefix(&dbi).unwrap().0;
    dbi_case(
        "dbi_cut_off_in_section_map",
        &dbi[..DBI_STREAM_HEADER_LEN
            + header.mod_info_size.get() as usize
            + header.section_contribution_size.get() as usize
            + 4],
    );
}

// Test roundtrip with only a header
#[test]
fn dbi_only_header() {
    let dbi = make_good_dbi();
    dbi_case("dbi_only_header", &dbi[..DBI_STREAM_HEADER_LEN]);
}

// Poke some bogus values into length fields
#[test]
fn dbi_negative_module_info() {
    let mut dbi = make_good_dbi();
    let header = DbiStreamHeader::mut_from_prefix(&mut dbi).unwrap().0;
    header.mod_info_size = (-4).into();
    dbi_case("dbi_negative_module_info", &dbi);
}

/// Write out a PDB with the DBI contents provided, convert it to PDZ, read the data back,
/// verify we got the same data.
#[inline(never)]
#[track_caller]
fn dbi_case(name: &str, original_dbi_data: &[u8]) {
    println!(
        "dbi_case: DBI stream length: {0} 0x{0:x}",
        original_dbi_data.len()
    );

    let dir = Path::new(TMP_DIR).join("dbi_chunking");
    _ = std::fs::create_dir_all(&dir);

    let pdb_file_name = dir.join(format!("{name}.pdb"));
    let pdz_file_name = dir.join(format!("{name}.pdz"));

    {
        let mut pdb = Msf::create(&pdb_file_name, Default::default()).unwrap();
        let mut sw = pdb.write_stream(Stream::DBI.into()).unwrap();
        sw.write_all(original_dbi_data).unwrap();
        pdb.commit().unwrap();
    }

    // Convert it to a PDZ
    let mut cmd = Command::new(PDBTOOL);
    cmd.arg("pdz-encode");
    cmd.arg(&pdb_file_name);
    cmd.arg(&pdz_file_name);
    run_command(cmd);

    // Open the PDZ and verify a few things
    let pdz = Msfz::open(&pdz_file_name).unwrap();
    let readback_dbi_data = pdz.read_stream(Stream::DBI.into()).unwrap();

    // Don't use assert_eq!() here; the data is too large and the debug display is not useful.
    assert!(readback_dbi_data.as_slice() == original_dbi_data);
}
