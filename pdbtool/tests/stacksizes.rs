use ms_pdb::codeview::syms::builder::SymBuilder;
use ms_pdb::dbi::{DBI_STREAM_VERSION_V110, DbiStreamHeader, ModuleInfoFixed};
use ms_pdb::modi::CV_SIGNATURE_C13;
use ms_pdb::msf::Msf;
use ms_pdb::pdbi::{PDBI_VERSION_VC98, PdbiStream};
use ms_pdb::syms::SymKind;
use ms_pdb::{Stream, StreamIndexU16};
use std::path::{Path, PathBuf};
use std::process::Command;
use zerocopy::{FromZeros, IntoBytes};

fn module_symbols(kind: SymKind, name: &str, frame_size: u32) -> Vec<u8> {
    let mut builder = SymBuilder::new();
    builder
        .buffer
        .extend_from_slice(&CV_SIGNATURE_C13.to_le_bytes());
    let proc_start = builder.buffer.len();
    {
        let mut proc = builder.record(kind);
        for _ in 0..8 {
            proc.enc.u32(0);
        }
        proc.enc.u16(1);
        proc.enc.u8(0);
        proc.enc.strz(name.into());
    }
    {
        let mut frame = builder.record(SymKind::S_FRAMEPROC);
        frame.enc.u32(frame_size);
        for _ in 0..4 {
            frame.enc.u32(0);
        }
        frame.enc.u16(0);
        frame.enc.u32(0);
    }
    let end_offset = u32::try_from(builder.buffer.len()).unwrap();
    builder.buffer[proc_start + 8..proc_start + 12].copy_from_slice(&end_offset.to_le_bytes());
    drop(builder.record(SymKind::S_END));
    builder.finish()
}

fn create_pdb() -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join("stacksizes");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("test.pdb");
    let mut msf = Msf::create(&path, Default::default()).unwrap();

    let first_symbols = module_symbols(SymKind::S_GPROC32, "module_one", 512);
    let second_symbols = module_symbols(SymKind::S_LPROC32, "module_two", 1024);
    let empty_symbols = CV_SIGNATURE_C13.to_le_bytes();
    let modules = [
        (Some(5), first_symbols.as_slice()),
        (None, &[][..]),
        (Some(6), second_symbols.as_slice()),
        (Some(7), empty_symbols.as_slice()),
        (Some(8), &[][..]),
    ];
    let mut module_info = Vec::new();
    for (index, (stream, symbols)) in modules.into_iter().enumerate() {
        let mut header = ModuleInfoFixed::new_zeroed();
        header.stream = StreamIndexU16::try_from(stream).unwrap();
        header.sym_byte_size = u32::try_from(symbols.len()).unwrap().into();
        module_info.extend_from_slice(header.as_bytes());
        module_info.extend_from_slice(format!("module{index}\0module{index}.obj\0").as_bytes());
        module_info.resize(module_info.len().next_multiple_of(4), 0);
        if let Some(stream) = stream {
            msf.write_stream(stream)
                .unwrap()
                .write_all_at_mut(symbols, 0)
                .unwrap();
        }
    }

    let pdbi = PdbiStream {
        signature: 0,
        version: PDBI_VERSION_VC98,
        age: 1,
        unique_id: None,
        named_streams: Default::default(),
        features: Vec::new(),
    };
    msf.write_stream(Stream::PDB.into())
        .unwrap()
        .write_all_at_mut(&pdbi.to_bytes().unwrap(), 0)
        .unwrap();

    let mut dbi = DbiStreamHeader::new_zeroed();
    dbi.signature = (-1).into();
    dbi.version = DBI_STREAM_VERSION_V110.into();
    dbi.age = 1.into();
    dbi.global_symbol_index_stream = StreamIndexU16::NIL;
    dbi.public_symbol_index_stream = StreamIndexU16::NIL;
    dbi.global_symbol_stream = StreamIndexU16::NIL;
    dbi.mod_info_size = i32::try_from(module_info.len()).unwrap().into();
    let mut dbi_bytes = dbi.as_bytes().to_vec();
    dbi_bytes.extend_from_slice(&module_info);
    msf.write_stream(Stream::DBI.into())
        .unwrap()
        .write_all_at_mut(&dbi_bytes, 0)
        .unwrap();
    msf.commit().unwrap();
    path
}

fn run_stacksizes(path: &Path, args: &[&str]) -> String {
    let output = Command::new(env!("CARGO_BIN_EXE_pdbtool"))
        .arg("dump")
        .arg(path)
        .arg("stacksizes")
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stacksizes failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn stacksizes_scans_all_modules() {
    let path = create_pdb();
    let header = "  Frame size  Procedure\n";
    let all_rows = concat!(
        "  Frame size  Procedure\n",
        "        1024  module_two\n",
        "         512  module_one\n"
    );
    assert_eq!(run_stacksizes(&path, &[]), all_rows);
    assert_eq!(run_stacksizes(&path, &["--threshold", "0"]), all_rows);
    assert_eq!(
        run_stacksizes(&path, &["--threshold", "1024"]),
        format!("{header}        1024  module_two\n")
    );
    assert_eq!(run_stacksizes(&path, &["--threshold", "1025"]), header);
    std::fs::remove_file(path).unwrap();
}
