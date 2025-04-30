//! Global Symbol Stream
//!
//! The Global Symbol Stream contains a sequence of variable-length symbol records. This stream does
//! not have a header of any kind; all of the stream data consists of CodeView symbol records.
//!
//! The GSS does not have a fixed stream number. The stream number is found in the DBI Stream
//! Header.
//!
//! Many other parts of the PDB contain pointers (byte offsets) that point into the GSS:
//! * PSI: Contains lookup tables for `S_PUB32` symbols
//! * GSI: Contains a lookup table for all other named global symbols
//! * Module Streams: Contains a Global Refs section that points to entries in the GSS that are
//!   referenced by that module.

use crate::syms::{Pub, SymIter, SymKind};
use anyhow::bail;
use ms_codeview::parser::Parse;

/// Contains the Global Symbol Stream (GSS). This contains symbol records.
///
/// The GSI and the PSI both point into this stream.
pub struct GlobalSymbolStream {
    /// Contains the stream data.
    pub stream_data: Vec<u8>,
}

impl GlobalSymbolStream {
    /// Constructor. This does not validate the contents.
    pub fn new(stream_data: Vec<u8>) -> Self {
        Self { stream_data }
    }

    /// Constructs an empty GSS.
    pub fn empty() -> Self {
        Self {
            stream_data: vec![],
        }
    }

    /// Gets a reference to a symbol at a given record offset.
    ///
    /// This function validates `record_offset`. If it is out of range, this function will return
    /// `Err` instead of panicking.
    pub fn get_sym_at(&self, record_offset: u32) -> anyhow::Result<crate::syms::Sym<'_>> {
        let Some(record_bytes) = self.stream_data.get(record_offset as usize..) else {
            bail!("Invalid record offset into GSS: {record_offset}.  Out of range for the GSS.");
        };

        let mut sym_iter = SymIter::new(record_bytes);
        let Some(sym) = sym_iter.next() else {
            bail!("Invalid record offset into GSS: {record_offset}. Failed to decode symbol data at that offset.");
        };

        Ok(sym)
    }

    /// Gets a reference to a symbol at a given record offset, and then parses it as an `S_PUB32`
    /// record.
    ///
    /// This function validates `record_offset`. If it is out of range, this function will return
    /// `Err` instead of panicking.
    ///
    /// If the symbol at `record_offset` is not an `S_PUB32` symbol, this function returns `Err`.
    pub fn get_pub32_at(&self, record_offset: u32) -> anyhow::Result<Pub<'_>> {
        let sym = self.get_sym_at(record_offset)?;

        if sym.kind != SymKind::S_PUB32 {
            bail!("Invalid record offset into GSS: {record_offset}. Found a symbol with the wrong type.  Expected S_PUB32, found {:?}",
                sym.kind
            );
        };

        let Ok(pub_sym) = crate::syms::Pub::parse(sym.data) else {
            bail!(
                "Invalid record offset into GSS: {record_offset}. Failed to decode S_PUB32 record."
            );
        };

        Ok(pub_sym)
    }

    /// Iterates the symbol records in the Global Symbol Stream.
    pub fn iter_syms(&self) -> SymIter<'_> {
        SymIter::new(&self.stream_data)
    }
}
