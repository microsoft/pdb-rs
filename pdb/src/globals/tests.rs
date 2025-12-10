use pretty_hex::PrettyHex;

use super::build_global_symbols_index;
use super::gsi::GlobalSymbolIndex;
use super::gss::GlobalSymbolStream;
use super::psi::PublicSymbolIndex;
use crate::syms::SymKind;
use crate::types::TypeIndex;
use ms_codeview::encoder::Encoder;

const NUM_BUCKETS: usize = 0x1000;

#[derive(Default)]
struct SymBuilder {
    buffer: Vec<u8>,
    record_start: usize,
}

impl SymBuilder {
    fn finish(self) -> GlobalSymbolStream {
        GlobalSymbolStream::new(self.buffer)
    }

    /// Starts adding a new record to the builder.
    fn start_record(&mut self, kind: SymKind) -> Encoder<'_> {
        self.record_start = self.buffer.len();
        self.buffer.extend_from_slice(&[0, 0]); // placeholder for record length
        self.buffer.extend_from_slice(&kind.0.to_le_bytes());
        Encoder::new(&mut self.buffer)
    }

    /// Finishes adding a record to the builder.
    fn end_record(&mut self) {
        match self.buffer.len() & 3 {
            1 => self.buffer.push(0xf1),
            2 => self.buffer.extend_from_slice(&[0xf1, 0xf2]),
            3 => self.buffer.extend_from_slice(&[0xf1, 0xf2, 0xf3]),
            _ => {}
        }

        let record_len = self.buffer.len() - self.record_start - 2;

        let record_field = &mut self.buffer[self.record_start..];
        record_field[0] = record_len as u8;
        record_field[1] = (record_len >> 8) as u8;
    }

    /// Adds an `S_UDT` record.
    fn udt(&mut self, ty: TypeIndex, name: &str) {
        let mut e = self.start_record(SymKind::S_UDT);
        e.u32(ty.0);
        e.strz(name.into());
        self.end_record();
    }

    /// Adds an `S_PUB32` record.
    fn pub32(&mut self, flags: u32, offset: u32, segment: u16, name: &str) {
        let mut e = self.start_record(SymKind::S_PUB32);
        e.u32(flags);
        e.u32(offset);
        e.u16(segment);
        e.strz(name.into());
        self.end_record();
    }
}

/// Builds a GSS with some example records
fn build_test_gss() -> Vec<u8> {
    let mut sb = SymBuilder::default();

    sb.udt(TypeIndex(0x1001), "FOO");
    sb.udt(TypeIndex(0x1001), "BAR");
    sb.udt(
        TypeIndex(0x1002),
        "AugmentedMultiThreadedSymbolExpanderServiceProviderSingletonAbstractBaseFacet",
    );

    // Add some S_PUB32 records. Put records out-of-order, with respect to their segment:offset,
    // so that we test the sorting code.
    sb.pub32(0, 100, 1, "main");
    sb.pub32(0, 200, 1, "memset");
    sb.pub32(0, 40, 1, "memcpy");
    sb.pub32(0, 30, 1, "CreateWindowEx");

    sb.buffer
}

#[test]
fn build_and_search_globals() {
    println!();

    let gss = GlobalSymbolStream::new(build_test_gss());
    println!("GSS:\n{:?}", &gss.stream_data.hex_dump());

    let indexes = build_global_symbols_index(&gss.stream_data, NUM_BUCKETS).unwrap();

    {
        let gsi =
            GlobalSymbolIndex::parse(NUM_BUCKETS, indexes.global_symbol_index_stream_data).unwrap();

        // Check consistency of name hashes
        gsi.names().check_hashes(&gss).unwrap();

        println!("Dumping names from GSI:");
        for name_sym in gsi.names().iter(&gss) {
            println!("{name_sym:?}");
        }

        let gsi_names = gsi.names();
        assert!(gsi_names
            .find_symbol(&gss, "bad_name_not_found".into())
            .unwrap()
            .is_none());
        assert!(gsi_names.find_symbol(&gss, "FOO".into()).unwrap().is_some());
        assert!(gsi_names.find_symbol(&gss, "BAR".into()).unwrap().is_some());
        assert!(gsi_names
            .find_symbol(
                &gss,
                "AugmentedMultiThreadedSymbolExpanderServiceProviderSingletonAbstractBaseFacet"
                    .into()
            )
            .unwrap()
            .is_some());
    }

    {
        let psi =
            PublicSymbolIndex::parse(NUM_BUCKETS, indexes.public_symbol_index_stream_data).unwrap();

        // Check consistency of name hashes
        psi.names().check_hashes(&gss).unwrap();

        psi.check_consistency(&gss).unwrap();

        assert!(psi
            .find_symbol_by_name(&gss, "bad_name_not_found".into())
            .unwrap()
            .is_none());

        {
            let memset = psi
                .find_symbol_by_name(&gss, "memset".into())
                .unwrap()
                .unwrap();
            assert_eq!(memset.name, "memset");
            assert_eq!(memset.fixed.offset_segment.offset(), 200);
        }

        {
            let memcpy = psi.find_symbol_by_addr(&gss, 1, 40).unwrap().unwrap();
            assert_eq!(memcpy.0.name, "memcpy");
        }
    }
}

#[test]
fn empty_psi() {
    let gss = GlobalSymbolStream::empty();
    let psi = PublicSymbolIndex::parse(NUM_BUCKETS, Vec::new()).unwrap();
    psi.check_consistency(&gss).unwrap();
}

#[test]
fn empty_gsi() {
    let gss = GlobalSymbolStream::empty();
    let gsi = GlobalSymbolIndex::parse(NUM_BUCKETS, Vec::new()).unwrap();
    assert!(gsi
        .find_symbol(&gss, bstr::BStr::new("none"))
        .unwrap()
        .is_none());

    // Check bad offset: Outside of bounds
    assert!(gss.get_sym_at(0xbadbad).is_err());

    // Check bad offset: The slice operation succeeds, but the symbol cannot be decoded.
    assert!(gss.get_sym_at(0).is_err());

    assert_eq!(gss.iter_syms().count(), 0);
}

#[test]
fn gss_get_pub32_wrong_type() {
    let mut sb = SymBuilder::default();
    sb.udt(TypeIndex(0x1001), "FOO");

    let gss = sb.finish();

    // Symbol exists, but has wrong type.
    assert!(gss.get_pub32_at(0).is_err());
}

#[test]
fn gss_get_pub32_invalid_symbol() {
    let mut sb = SymBuilder::default();

    // S_PUB32 record with invalid contents
    let _e = sb.start_record(SymKind::S_PUB32);
    sb.end_record();

    let gss = sb.finish();

    // Found record at offset, but it could not be decoded as S_PUB32 because its contents are bogus.
    assert!(gss.get_pub32_at(0).is_err());
}
