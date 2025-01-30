//! Public Symbol Index
//!
//! The Public Symbol Index (PSI) provides several look-up tables that accelerate finding
//! information in the Global Symbol Stream. The PSI indexes only `S_PUB32` symbols in the GSS; all
//! other symbol kinds are indexed in the GSI.
//!
//! The PSS does not have a fixed stream number. The stream number is found in the DBI Stream
//! Header.

use super::gss::*;
use super::name_table::*;
use crate::parser::{Parse, Parser, ParserMut};
use crate::syms::{OffsetSegment, Pub};
use crate::utils::is_aligned_4;
use anyhow::{bail, Context};
use bstr::BStr;
use std::mem::size_of;
use tracing::{debug, error, info};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, LE, U16, U32};

/// The header of the GSI stream.
///
/// See `PSGSIHDR` in `microsoft-pdb/PDB/dbi/gsi.h`.
#[repr(C)]
#[derive(IntoBytes, FromBytes, Unaligned, Immutable, KnownLayout, Clone, Debug)]
#[allow(missing_docs)]
pub struct PsiStreamHeader {
    /// Length in bytes of the symbol hash table.  This region immediately follows PSGSIHDR.
    pub name_table_size: U32<LE>,

    /// Length in bytes of the address map.  This region immediately follows the symbol hash.
    pub addr_table_size: U32<LE>,

    /// The number of thunk records.
    pub num_thunks: U32<LE>,
    /// Size in bytes of each thunk record.
    pub thunk_size: U32<LE>,
    pub thunk_table_section: U16<LE>,
    pub padding: U16<LE>,
    pub thunk_table_offset: U32<LE>,
    pub num_sections: U32<LE>,
}
static_assertions::const_assert_eq!(core::mem::size_of::<PsiStreamHeader>(), 28);

/// Contains the Public Symbol Index
///
/// The Public Symbol Index (PSI) contains a name-to-symbol lookup table and an address-to-symbol
/// lookup table.
pub struct PublicSymbolIndex {
    /// Allows name-to-symbol look for `S_PUB32` symbols.
    name_table: NameTable,

    /// Each entry in this table is a byte offset of one `S_PUB32` symbol in the GSS.
    /// All of the values are sorted by `(segment, offset)`, which allows binary search.
    addr_map: Vec<u32>,
}

impl PublicSymbolIndex {
    /// Parses the PSI from stream data. The caller must specify `num_buckets` because that value
    /// is not stored within the stream.
    pub fn parse(num_buckets: usize, public_stream_data: Vec<u8>) -> anyhow::Result<Self> {
        let mut p = Parser::new(&public_stream_data);
        if p.is_empty() {
            return Ok(Self::empty());
        }

        let psgsi_header: &PsiStreamHeader = p.get()?;
        debug!("PsiStreamHeader: {:#?}", psgsi_header);

        let sym_hash_size = psgsi_header.name_table_size.get() as usize;
        let addr_map_size = psgsi_header.addr_table_size.get() as usize;

        debug!("Size of symbol hash table: {} bytes", sym_hash_size);
        debug!("Size of address map: {} bytes", addr_map_size);

        let sym_hash_bytes = p
            .bytes(sym_hash_size)
            .with_context(|| "Failed to locate symbol hash table within Publics stream")?;
        let addr_map_bytes = p
            .bytes(addr_map_size)
            .with_context(|| "Failed to locate address map within Publics stream")?;

        let name_table =
            NameTable::parse(num_buckets, size_of::<PsiStreamHeader>(), sym_hash_bytes)?;

        // Load the address map. The address map is an array of u32 values, each of which is an
        // offset into the global symbol stream. I'm _guessing_ that the array is sorted by
        // [segment:offset].
        let addr_map: Vec<u32>;
        {
            let num_addrs = addr_map_bytes.len() / 4;
            info!("Number of entries in address map: {}", num_addrs);

            let mut addr_parser = Parser::new(addr_map_bytes);
            let addr_map_u32_slice: &[U32<LE>] = addr_parser.slice(num_addrs)?;

            addr_map = addr_map_u32_slice.iter().map(|i| i.get()).collect();
        }

        Ok(PublicSymbolIndex {
            name_table,
            addr_map,
        })
    }

    /// Constructs an empty instance of the PSI.
    pub fn empty() -> Self {
        Self {
            addr_map: vec![],
            name_table: NameTable::empty(),
        }
    }

    /// Check invariants for the PSI. This requires having access to the GSS, since the PSI
    /// points into the GSS.
    pub fn check_consistency(&self, gss: &GlobalSymbolStream) -> anyhow::Result<()> {
        // Verify that all entries in the address map are in non-decreasing order.
        let mut prev_sym: Option<Pub<'_>> = None;
        let mut num_bad_order: u32 = 0;
        for &offset in self.addr_map.iter() {
            let sym = gss.get_pub32_at(offset)?;
            if let Some(prev_sym) = &prev_sym {
                if prev_sym.offset_segment() > sym.offset_segment() {
                    if num_bad_order < 20 {
                        error!("found addr map in bad order");
                    }
                    num_bad_order += 1;
                }
            }
            prev_sym = Some(sym);
        }

        if num_bad_order != 0 {
            bail!(
                "Found {} address map entries that were out of order.",
                num_bad_order
            );
        }
        info!("All address map entries are correctly ordered.");

        Ok(())
    }

    /// Gets direct access to the name-to-symbol table.
    pub fn names(&self) -> &NameTable {
        &self.name_table
    }

    /// Searches for an `S_PUB32` symbol by name.
    pub fn find_symbol_by_name<'a>(
        &self,
        gss: &'a GlobalSymbolStream,
        name: &BStr,
    ) -> anyhow::Result<Option<Pub<'a>>> {
        if let Some(sym) = self.name_table.find_symbol(gss, name)? {
            Ok(Some(Pub::parse(sym.data)?))
        } else {
            Ok(None)
        }
    }

    /// Searches for an `S_PUB32` symbol by address.
    pub fn find_symbol_by_addr<'a>(
        &self,
        gss: &'a GlobalSymbolStream,
        segment: u16,
        offset: u32,
    ) -> anyhow::Result<Option<(Pub<'a>, u32)>> {
        use std::cmp::Ordering;

        let addr_map = self.addr_map.as_slice();

        let mut items = addr_map;
        while !items.is_empty() {
            let mid_index = items.len() / 2;
            let mid_rec = gss.get_pub32_at(items[mid_index])?;
            let mid_segment = mid_rec.fixed.offset_segment.segment();
            let mid_offset = mid_rec.fixed.offset_segment.offset();

            match segment.cmp(&mid_segment) {
                Ordering::Less => {
                    // info!("segment is less, moving low");
                    items = &items[..mid_index];
                    continue;
                }
                Ordering::Greater => {
                    // info!("segment is greater, moving high");
                    items = &items[mid_index + 1..];
                    continue;
                }
                Ordering::Equal => {}
            }

            // Same segment. Compare the offsets.

            if offset < mid_offset {
                // info!("offset is less, moving low");
                items = &items[..mid_index];
                continue;
            }

            if offset == mid_offset {
                // Bullseye!
                // info!("found it");
                return Ok(Some((mid_rec, 0)));
            }

            // info!("offset is greater, moving high");
            // The address we are looking for is higher than the address of the symbol that we are
            // currently looking at.
            // TODO: Implement best-so-far search.
            items = &items[mid_index + 1..];
            continue;
        }

        Ok(None)
    }
}

/// Sorts an address map slice.
#[inline(never)]
pub fn sort_address_records(addr_map: &mut [(u32, OffsetSegment)]) {
    addr_map.sort_unstable_by_key(|(record_offset, os)| (os.clone(), *record_offset));
}

/// Builds the Public Symbol Index (PSI).
///
/// The PSI contains both a name-to-symbol table and an address-to-symbol table.
pub fn build_psi(
    sorted_hash_records: &mut NameTableBuilder,
    sorted_addr_map: &[(u32, OffsetSegment)],
) -> Vec<u8> {
    assert_eq!(sorted_hash_records.num_names(), sorted_addr_map.len());

    debug!(
        "Number of entries in address table: {n} 0x{n:x}",
        n = sorted_addr_map.len()
    );
    debug!(
        "Size in bytes of address table: {s} 0x{s:x}",
        s = sorted_addr_map.len() * 4
    );

    let name_table_info = sorted_hash_records.prepare();
    let addr_map_size_bytes = sorted_addr_map.len() * size_of::<i32>();

    let stream_size_bytes =
        size_of::<PsiStreamHeader>() + name_table_info.table_size_bytes + addr_map_size_bytes;

    let mut stream_data: Vec<u8> = vec![0; stream_size_bytes];
    let mut p = ParserMut::new(&mut stream_data);

    let stream_header = PsiStreamHeader {
        name_table_size: U32::new(name_table_info.table_size_bytes as u32),
        addr_table_size: U32::new(addr_map_size_bytes as u32),
        num_thunks: U32::new(0), // TODO
        thunk_size: U32::new(0), // TODO
        padding: U16::new(0),
        thunk_table_section: U16::new(0), // TODO
        thunk_table_offset: U32::new(0),  // TODO
        num_sections: U32::new(0),        // TODO
    };
    *p.get_mut::<PsiStreamHeader>().unwrap() = stream_header;

    let name_table_bytes = p.bytes_mut(name_table_info.table_size_bytes).unwrap();
    sorted_hash_records.encode(&name_table_info, name_table_bytes);

    let addr_map_bytes = p.bytes_mut(addr_map_size_bytes).unwrap();
    let addr_map_output = <[U32<LE>]>::mut_from_bytes(addr_map_bytes).unwrap();
    // Write the address map. This converts from the array that we used for sorting, which contains
    // the symbol record byte offset and the segment:offset, to just the symbol record byte offset.
    {
        for (from, to) in sorted_addr_map.iter().zip(addr_map_output.iter_mut()) {
            *to = U32::new(from.0);
        }
    }

    assert!(p.is_empty());

    // Make it easy to understand the output.
    {
        let mut pos = 0;
        let mut region = |name: &str, len: usize| {
            debug!("    {pos:08x} +{len:08x} : {name}");
            pos += len;
        };
        debug!("PSI Stream layout:");
        region("PSI Stream Header", size_of::<PsiStreamHeader>());
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
        region("Address Table", addr_map_size_bytes);
        region("(end)", 0);
        assert_eq!(pos, stream_data.len());
    }

    assert!(is_aligned_4(stream_data.len()));

    stream_data
}
