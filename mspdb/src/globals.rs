//! Global Symbols
//!
//! This module contains code for reading the public / global symbol streams. This is a
//! moderately-complicated set of data structures, and requires reading several streams and
//! correlating data between them.
//!
//! Global symbols are stored in several streams. The stream indexes are stored in the DBI
//! stream header; the stream indexes are not fixed.

pub mod gsi;
pub mod gss;
pub mod name_table;
pub mod psi;

#[cfg(test)]
mod tests;

use crate::parser::{Parse, ParserError};
use crate::syms::{self, Constant, OffsetSegment, Pub, SymIter, SymKind};
use crate::utils::iter::IteratorWithRangesExt;
use crate::ReadAt;
use anyhow::Context;
use bstr::BStr;
use log::{debug, warn};
use std::collections::HashMap;

#[cfg(doc)]
use crate::dbi::DbiStreamHeader;

impl<F: ReadAt> crate::Pdb<F> {
    /// Reads the Global Symbol Stream (GSS). This stream contains global symbol records.
    ///
    /// This function does not validate the contents of the stream.
    pub fn read_gss(&self) -> anyhow::Result<gss::GlobalSymbolStream> {
        if let Some(gss_stream) = self.dbi_header.global_symbol_stream.get() {
            let stream_data = self.read_stream_to_vec(gss_stream)?;
            Ok(gss::GlobalSymbolStream { stream_data })
        } else {
            Ok(gss::GlobalSymbolStream::empty())
        }
    }

    /// Reads the Global Symbol Index (GSI). This stream contains a name-to-symbol lookup table.
    /// It indexes many global symbols, such as `S_GPROCREF`, `S_CONSTANT`, etc.
    pub fn read_gsi(&self) -> anyhow::Result<gsi::GlobalSymbolIndex> {
        if let Some(gsi_stream) = self.dbi_header.global_symbol_index_stream.get() {
            let num_buckets = self.num_buckets_for_name_table();
            let gsi_stream_data = self.read_stream_to_vec(gsi_stream)?;
            gsi::GlobalSymbolIndex::parse(num_buckets, gsi_stream_data)
        } else {
            Ok(gsi::GlobalSymbolIndex::empty())
        }
    }

    /// Returns the number of buckets to use in `NameTable`, for use by the GSI and PSI.
    pub(crate) fn num_buckets_for_name_table(&self) -> usize {
        let minimal_dbg_info = self.mini_pdb();
        name_table::get_v1_default_bucket(minimal_dbg_info)
    }

    /// Reads the Public Symbol Index.
    pub fn read_psi(&self) -> anyhow::Result<psi::PublicSymbolIndex> {
        if let Ok(psi_stream) = self.dbi_header.public_stream_index() {
            let num_buckets = self.num_buckets_for_name_table();
            let public_stream_data = self.read_stream_to_vec(psi_stream)?;
            psi::PublicSymbolIndex::parse(num_buckets, public_stream_data)
        } else {
            Ok(psi::PublicSymbolIndex::empty())
        }
    }
}

/// If `kind` is a global symbol that should be indexed in the GSI or PSI, then this returns the
/// name of that global symbol (within `Some`).
///
/// A "global symbol" in this context is any symbol that can appear in the Global Symbol Stream
/// and be indexed in the Global Symbol Index or Public Symbol Index. The list of global symbols:
///
/// * `S_PUB32`
/// * `S_CONSTANT`
/// * `S_PROCREF`
/// * `S_LPROCREF`
/// * `S_DATAREF`
/// * `S_ANNOTATIONREF`
/// * `S_UDT`
/// * `S_LDATA32`
/// * `S_GDATA32`
/// * `S_LTHREAD32`
/// * `S_GTHREAD32`
pub fn get_global_symbol_name(kind: SymKind, data: &[u8]) -> Result<Option<&BStr>, ParserError> {
    match kind {
        SymKind::S_PUB32 => {
            let pub_data = Pub::parse(data)?;
            Ok(Some(pub_data.name))
        }

        SymKind::S_CONSTANT => {
            let constant_record = Constant::parse(data)?;
            Ok(Some(constant_record.name))
        }

        // These symbols have the same structure.
        SymKind::S_PROCREF
        | SymKind::S_LPROCREF
        | SymKind::S_DATAREF
        | SymKind::S_ANNOTATIONREF => {
            let ref_sym = syms::RefSym2::parse(data)?;
            Ok(Some(ref_sym.name))
        }

        SymKind::S_UDT => {
            let udt_data = syms::Udt::parse(data)?;
            Ok(Some(udt_data.name))
        }

        SymKind::S_LDATA32 | SymKind::S_GDATA32 | SymKind::S_LMANDATA | SymKind::S_GMANDATA => {
            let data = syms::Data::parse(data)?;
            Ok(Some(data.name))
        }

        SymKind::S_LTHREAD32 | SymKind::S_GTHREAD32 => {
            let thread_storage = syms::ThreadStorageData::parse(data)?;
            Ok(Some(thread_storage.name))
        }

        SymKind::S_LMANPROC | SymKind::S_GMANPROC => {
            let man_proc = syms::ManProcSym::parse(data)?;
            Ok(Some(man_proc.name))
        }

        // TODO
        SymKind::S_TOKENREF => Ok(None),

        _ => Ok(None),
    }
}

/// Output of `build_global_symbols_index`
pub struct BuildGlobalSymbolsIndexesOutput {
    /// The new GSI contents
    pub global_symbol_index_stream_data: Vec<u8>,
    /// The new PSI contents
    pub public_symbol_index_stream_data: Vec<u8>,
}

/// Reads a Global Symbol Stream and constructs a new Global Symbol Index (GSI) and
/// Public Symbol Index (PSI).
pub fn build_global_symbols_index(
    symbol_records: &[u8],
    num_buckets: usize,
) -> anyhow::Result<BuildGlobalSymbolsIndexesOutput> {
    debug!("Rebuilding Global Symbol Index (GSI) and Public Symbol Index (PSI)");

    let mut public_hash_records = name_table::NameTableBuilder::new(num_buckets);
    let mut global_hash_records = name_table::NameTableBuilder::new(num_buckets);

    // contains (byte offset in symbol stream, SegmentOffset)
    let mut public_addr_map: Vec<(u32, OffsetSegment)> = Vec::new();

    let mut unrecognized_symbols: HashMap<SymKind, u32> = HashMap::new();

    for (sym_range, sym) in SymIter::new(symbol_records).with_ranges() {
        let sym_offset = sym_range.start;

        // If the symbol is S_PUB32, then add an entry to both public_hash_records and
        // global_hash_records.
        if sym.kind == SymKind::S_PUB32 {
            let pub_data =
                Pub::parse(sym.data).with_context(|| "failed to parse S_PUB32 record")?;
            public_hash_records.push(pub_data.name, (sym_offset + 1) as i32);
            public_addr_map.push((sym_offset as u32, pub_data.offset_segment()));
            continue;
        }

        if matches!(sym.kind, SymKind::S_TOKENREF | SymKind::S_DATAREF) {
            continue;
        }

        if let Some(sym_name) = get_global_symbol_name(sym.kind, sym.data)? {
            global_hash_records.push(sym_name, (sym_offset + 1) as i32);
        } else {
            *unrecognized_symbols.entry(sym.kind).or_default() += 1;
        }
    }

    if !unrecognized_symbols.is_empty() {
        warn!(
            "Number of unrecognized symbol types found in Global Symbol Stream: {}",
            unrecognized_symbols.len()
        );
        let mut sorted_unrecognized: Vec<(SymKind, u32)> =
            unrecognized_symbols.iter().map(|(&k, &v)| (k, v)).collect();
        sorted_unrecognized.sort_unstable_by_key(|(k, _)| *k);
        for (kind, count) in sorted_unrecognized.iter() {
            warn!(
                "    {count:6} - [{raw_kind:04x}] {kind:?}",
                raw_kind = kind.0
            );
        }
    }

    psi::sort_address_records(&mut public_addr_map);

    debug!("Building Global Symbol Index (GSI)");
    let global_symbol_stream_data = gsi::build_gsi(&mut global_hash_records);

    debug!("Building Public Symbol Index (PSI)");
    let public_symbol_stream_data = psi::build_psi(&mut public_hash_records, &public_addr_map);

    Ok(BuildGlobalSymbolsIndexesOutput {
        global_symbol_index_stream_data: global_symbol_stream_data,
        public_symbol_index_stream_data: public_symbol_stream_data,
    })
}
