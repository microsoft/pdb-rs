//! Global Symbol Index
//!
//! The Global Symbol Index (GSI) Stream provides a name-to-symbol lookup table for global symbols
//! that have a name.
//!
//! The GSI does not have a fixed stream number. The stream number is found in the DBI Stream
//! Header.
//!
//! The GSI contains entries only for the following symbol kinds:
//!
//! * `S_CONSTANT`
//! * `S_UDT`
//! * `S_LDATA32`
//! * `S_GDATA32`
//! * `S_LTHREAD32`
//! * `S_GTHREAD32`
//! * `S_LMANDATA`
//! * `S_GMANDATA`
//! * `S_PROCREF`
//! * `S_LPROCREF`
//! * `S_ANNOTATIONREF`
//! * `S_TOKENREF`
//!
//! Note that `S_PUB32` is not included in this list.  `S_PUB32` symbols are indexed in the PSI, not
//! the GSI.
//!
//! The GSI does not provide an address-to-name lookup table.

use super::name_table::*;
use crate::syms::Sym;
use crate::utils::is_aligned_4;
use bstr::BStr;
use std::mem::size_of;
use tracing::{debug, trace_span};

/// Contains the Global Symbol Index
pub struct GlobalSymbolIndex {
    name_table: NameTable,
}

impl GlobalSymbolIndex {
    /// Parses the Global Symbol Index from stream data. The caller must specify `num_buckets`
    /// because the value is not specified in the GSI itself.
    pub fn parse(num_buckets: usize, stream_data: Vec<u8>) -> anyhow::Result<GlobalSymbolIndex> {
        if stream_data.is_empty() {
            return Ok(Self::empty());
        }

        let name_table = NameTable::parse(num_buckets, 0, &stream_data)?;
        Ok(Self { name_table })
    }

    /// Constructs an empty instance of the GSI.
    pub fn empty() -> Self {
        Self {
            name_table: NameTable::empty(),
        }
    }

    /// Find a symbol within the GSI by name.
    pub fn find_symbol<'a, 'n>(
        &self,
        gss: &'a crate::globals::gss::GlobalSymbolStream,
        name: &BStr,
    ) -> anyhow::Result<Option<Sym<'a>>> {
        let name_raw: &BStr = name.into();
        self.name_table.find_symbol(gss, name_raw)
    }

    /// Gets direct access to the name-to-symbol table.
    pub fn names(&self) -> &NameTable {
        &self.name_table
    }
}

/// Builds the Global Symbol Index (GSI) table.
///
/// The GSI contains only a name table. It does not contain an address table.
pub fn build_gsi(sorted_hash_records: &mut NameTableBuilder) -> Vec<u8> {
    let _span = trace_span!("build_gsi").entered();

    let name_table_info = sorted_hash_records.prepare();
    let mut stream_data: Vec<u8> = vec![0; name_table_info.table_size_bytes];
    sorted_hash_records.encode(&name_table_info, &mut stream_data);

    // Make it easy to understand the output.
    {
        let mut pos = 0;
        let mut region = |name: &str, len: usize| {
            debug!("    {pos:08x} +{len:08x} : {name}");
            pos += len;
        };
        debug!("GSI Stream layout:");
        region("Name Table - Header", size_of::<NameTableHeader>());
        region(
            "Name Table - Hash Records",
            sorted_hash_records.num_names() * size_of::<HashRecord>(),
        );
        region(
            "Name Table - Buckets Bitmap",
            nonempty_bitmap_size_bytes(sorted_hash_records.num_buckets()),
        );
        region(
            "Name Table - Buckets",
            name_table_info.num_nonempty_buckets * 4,
        );
        region("(end)", 0);
        assert_eq!(pos, stream_data.len());
    }

    assert!(is_aligned_4(stream_data.len()));

    stream_data
}
