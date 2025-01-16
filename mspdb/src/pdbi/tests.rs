use dump_utils::HexDump;

use super::*;

static KERNELBASE_PDBI: &[u8] = include_bytes!("tests/kernelbase_pdbi.bin");
static DWRITECORE_PDBI: &[u8] = include_bytes!("tests/dwritecore_pdbi.bin");

#[test]
fn parse_kernelbase() {
    let pdbi = PdbiStream::parse(KERNELBASE_PDBI).unwrap();
    for (name, stream) in pdbi.named_streams().iter() {
        println!("stream: {:20} -> {:10}", name, stream);
    }

    assert_eq!(pdbi.named_streams.get("/names"), Some(7));

    assert_eq!(pdbi.named_streams.get("sourcelink$1"), Some(2344));
}

#[test]
fn parse_dwritecore() {
    let pdbi = PdbiStream::parse(DWRITECORE_PDBI).unwrap();
    for (name, stream) in pdbi.named_streams().iter() {
        println!("stream: {:20} -> {:10}", name, stream);
    }

    assert_eq!(pdbi.named_streams.get("/names"), Some(7));
    assert_eq!(pdbi.named_streams.get("sourcelink$1"), Some(3709));
}

fn names_build(names: &NamedStreams) {
    let mut bytes = Vec::new();
    names.to_bytes(&mut Encoder::new(&mut bytes));
    println!("\n{:?}", HexDump::new(&bytes));

    // Round-trip testing: Decode the stream that we just built.
    let mut p = Parser::new(&bytes);
    let rt_names =
        NamedStreams::parse(&mut p).expect("expected to successfully parse names stream");

    assert_eq!(names.map, rt_names.map);
    assert!(
        p.is_empty(),
        "found unparsed bytes at the end:\n{:?}",
        HexDump::new(p.peek_rest())
    );

    // Round-trip testing *again*.  Encode the round-trip table into bytes again, and verify that
    // we got the exact same bytes.
    let mut rt_bytes = Vec::new();
    names.to_bytes(&mut Encoder::new(&mut rt_bytes));
    assert_eq!(bytes, rt_bytes, "expected round-trip bytes to be the same");
}

#[test]
fn names_build_empty() {
    let names = NamedStreams::default();
    names_build(&names);
}

#[test]
fn names_build_simple() {
    let mut names = NamedStreams::default();
    names.map.insert("/foo".to_string(), 100);
    names.map.insert("/bar".to_string(), 200);
    names_build(&names);
}

#[test]
fn names_build_many() {
    let n = 100;
    let mut names = NamedStreams::default();
    for i in 0..n {
        names.map.insert(format!("/num/{i:04}"), 1000 + i as u32);
    }
    names_build(&names);
}

#[test]
fn names_rebuild_kernelbase() {
    rebuild_test(KERNELBASE_PDBI);
}

#[test]
fn names_rebuild_dwritecore() {
    rebuild_test(DWRITECORE_PDBI);
}

fn rebuild_test(stream_data: &[u8]) {
    println!("Input:\n{:?}", HexDump::new(stream_data));
    let pdbi = PdbiStream::parse(stream_data).unwrap();
    let bytes = pdbi.to_bytes().unwrap();
    println!("Output:\n{:?}", HexDump::new(&bytes));
}
