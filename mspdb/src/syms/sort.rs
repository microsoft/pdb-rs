//! Algorithm for sorting symbol records found in symbol streams.
//!
//! A symbol stream consists of a sequence of variable-length symbol records. Sorting a symbol
//! stream requires solving several design problems.
//!
//! Some symbols create a "scope" or "block" of symbols, which relates a contiguous sequence of
//! symbols to the parent symbol. For example, `S_GPROC32` records indicate the start of a procedure
//! definition; all symbols that follow `S_GPROC32` are part of the procedure definition, until
//! the `S_END` symbol is reached.  These nesting relationships must be preserved during sorting.
//!
//! Symbol scopes can be recursive, a `S_GPROC32` may contain an `S_INLINESITE`, which contains
//! another `S_INLINESITE`.
//!
//! Symbols that create a scope also contain pointers (byte offsets into the symbol stream)
//! that point to the symbol record that terminates the scope (usually an `S_END` symbol) and
//! to the parent record, if this record is itself nested.  These pointers are byte offsets
//! relative to the start of the symbol stream.  Because sorting changes the location of
//! records, these `end` and `parent` pointers must be updated after sorting.
//!
//! There are several streams that contain byte offsets that point to records in the Global Symbol
//! Stream. The GSI and PSI both contain such pointers. Module Streams contain pointers into
//! the GSS in the Global Refs section of the Module Stream.  If the GSS is sorted, then these
//! pointers from the GSI, PSI, and Module Global Refs must be updated.
//!
//! To summarize:
//!
//! * Symbol nesting relationships must be preserved.
//! * Symbol pointers within the symbol stream (`parent` and `end` pointers) must be updated.
//! * Symbol pointers in the GSI, PSI, and Module Global Refs must be updated.
//!
//! # Algorithm
//!
//! This algorithm goes through roughly these steps:
//!
//! * Find the locations of the "top-level scopes", which are the sequences of symbols that
//!   form a tree of nesting scopes, or are simply a single symbol record that does not contain any
//!   nesting. This builds the `scope_locations` vector.
//!
//! * Edit symbol records and set the `parent` and `end` pointers to 0. This is necessary for
//!   producing a deterministic sort order, because the `parent` and `end` pointers do not contain
//!   significant information and their values depend on record locations.
//!
//! * Produce a permutation vector for the scopes and sort it. This tells us the order of the
//!   scopes to write to the output.
//!
//! * Copy each top-level scope (each sequence of contiguous symbol records) into a temporary
//!   buffer.
//!
//!   This step also updates the `parent` and `end` pointers, since this is the step where we
//!   determine the new output location of each.
//!
//!   In this step, we also add entries to a remapping table. This remapping table maps the
//!   byte offsets of symbols (actually, scopes) in the old table to the byte offsets of symbols in
//!   the new table. This remapping table is used by code that remaps byte offsets (pointing into
//!   the GSS) in the GSI, PSI, and Module Global Refs.
//!
//! * Sort the remapping table so that it maps from old to new offsets.
//!
//! * Copy the temporary buffer back to the primary storage.

use super::SymData;
use crate::parser::{Parser, ParserMut};
use crate::sort_utils::identity_permutation_u32;
use crate::syms::{self, SymIter, SymIterMut, SymKind};
use crate::utils::iter::IteratorWithRangesExt;
use anyhow::bail;
use log::debug;
use std::cmp::Ordering;

/// Provides a mapping from old symbol record offsets to new symbol record offsets.
pub struct RemappedSymbolTable {
    /// Sorted by increasing `old_offset`. This allows for binary search to find an entry, then
    /// using the current entry and the next to find the record length, then new_offset to
    /// remap.
    pub vec: Vec<RemappedSymbolEntry>,
}

impl RemappedSymbolTable {
    /// Map from byte offsets in the old stream to byte offsets in the new stream.
    pub fn remap_with_delta(&self, old_offset: u32) -> Option<u32> {
        match self.vec.binary_search_by_key(&old_offset, |e| e.old_offset) {
            Ok(i) => Some(self.vec[i].new_offset),
            Err(i) => {
                if i == 0 {
                    // old_offset is lower than the first entry. No match is possible.
                    return None;
                }

                let previous = &self.vec[i - 1];
                assert!(old_offset > previous.old_offset);
                let delta_within_scope = old_offset - previous.old_offset;
                let new_offset = previous.new_offset + delta_within_scope;
                Some(new_offset)
            }
        }
    }

    /// Map from byte offsets in the old stream to byte offsets in the new stream.
    ///
    /// This function requires that `old_offset` point to the start of a symbol that is a root
    /// scope. It will not find symbol records that are nested within other symbol records.
    pub fn remap_exact(&self, old_offset: u32) -> Option<u32> {
        match self.vec.binary_search_by_key(&old_offset, |e| e.old_offset) {
            Ok(i) => Some(self.vec[i].new_offset),
            Err(_) => None,
        }
    }
}

/// Represents one symbol scope (a hierarchy of nested symbols; potentially just one symbol record)
/// that has been remapped.
pub struct RemappedSymbolEntry {
    /// The byte offset in the symbol stream before records were sorted.
    pub old_offset: u32,
    /// The byte offset in the symbol stream after records were sorted.
    pub new_offset: u32,
}

/// Describes the relocation of the target of a a REFSYM symbol.
///
/// Several kinds of symbol records found in the GSS point into per-module symbol streams.
/// These include S_PROCREF, S_DATAREF, S_ANNOTATIONREF. All use the `REFSYM2` structure.
/// All contain a module index and a byte offset within that module's symbol stream.
///
/// When we reorder the symbol records within a module's symbol stream, we accumulate these records
/// in a vector. After module streams have been processed, we scan through the global symbol stream
/// again, and we use this vector to fix up references that point into per-module symbol streams.
#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct ModuleRefSymMapping {
    /// zero-based module index
    pub old_module: u16,
    /// old byte offset of this symbol from start of module symbol stream
    pub old_offset: u32,
    /// zero-based module index
    pub new_module: u16,
    /// new byte offset of this symbol from start of module symbol stream
    pub new_offset: u32,
}

/// Specifies sorting modes used by [`sort_module_syms`] and [`sort_global_syms`].
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum SymbolSortingMode {
    /// Sorts records by their encoded byte representation. This is efficient, but produces a
    /// sorting order that is difficult to read for humans.
    Bytes,

    /// Sorts records by their kind, then by their byte representation. This is nearly as fast as
    /// `Bytes` and makes debugging easier because of the grouping.
    KindBytes,

    /// Sorts symbols by symbol kind, then `unique_name` (if the symbol has a `unique_name`), then
    /// by the encoded byte representation of the record (as a fallback).
    ///
    /// This sorting order makes it easy to read the symbol dumps, since symbols of the same kind
    /// are grouped together and they are sorted by their unique name. It is slower than `Bytes`.
    KindName,
}

impl Default for SymbolSortingMode {
    fn default() -> Self {
        Self::KindName
    }
}

/// Specifies options for sorting module symbols
#[derive(Default, Clone)]
pub struct SortModuleSymsOptions {
    /// How to sort symbols
    pub sorting_mode: SymbolSortingMode,

    /// Remove `S_UDT` records from module symbol streams. If an `S_UDT` symbol is present in a
    /// module symbol stream, it is usually there because of an ODR violation.
    pub remove_udt: bool,
}

/// Sorts the records in `syms_data`. This can only be used with module symbol streams. It should
/// not be used with the Global Symbol stream.
///
/// Some symbols "contain" other symbols; that is, they form a hierarchy of scopes. The sorting
/// algorithm groups together symbols into scopes.  Reordering is performed only at the root level
/// of the hierarchy; symbol records within a scope hierarchy are not reordered.
///
/// Symbols at the root of the scopes tree are sorted. The exact sorting algorithm is not important,
/// but it is important that the sorting algorithm use information that is stable. We are still
/// experimenting with different sorting algorithms, balancing performance vs. diagnostics.
///
/// This function returns a mapping table that maps from byte offsets in the old symbols stream
/// to byte offsets in the new symbols stream. There is one record in this array for each root-level
/// symbol that starts a scope; this table does not contain record offsets for symbols that are
/// nested within those scopes.
///
pub fn sort_module_syms(
    options: &SortModuleSymsOptions,
    syms_data: &mut [u8], // This INCLUDES the 4-byte signature at the start
    old_module: u16,
    new_module: u16,
    module_refsym_remapping: &mut Vec<ModuleRefSymMapping>,
) -> anyhow::Result<Vec<u8>> {
    debug!("Sorting module symbol records");

    // scope_starts is a "starts" vector.
    let mut scope_starts: Vec<u32> = Vec::new();

    let mut sym_iter = SymIterMut::new(syms_data).with_ranges();

    // Module symbol streams begin with a 4-byte signature. Parse it now. This also has the
    // side-effect of ensuring that our symbol record byte offsets are correct, in the code
    // that queries them below.
    let signature: [u8; 4] = sym_iter.inner_mut().get_signature()?;

    // The current scope nesting depth.
    let mut depth: u32 = 0;

    for (sym_range, sym) in sym_iter {
        let kind = sym.kind;
        let sym_pos = sym_range.start as u32;

        // We occasionally see S_UDT symbols in module streams. There is some evidence that
        // they are only present when the linker sees ODR violations, with conflicting
        // definitions for UDTs from different modules.  If allowed, we remove these.
        if options.remove_udt && sym.kind == SymKind::S_UDT {
            continue;
        }

        if depth == 0 {
            scope_starts.push(sym_pos);
        }

        if kind.starts_block() {
            let mut p = ParserMut::new(sym.data);
            let block: &mut syms::BlockHeader = p.get_mut()?;

            // Convert the p_end field to a relative offset. This allows sorting to work, because
            // otherwise we would be sorting position-dependent information, which would defeat
            // sorting. After sorting is finished, we will convert these relative offsets back to
            // absolute offsets, but using a different base (the sorted stream position).
            let p_end = block.p_end.get();
            if p_end < sym_pos {
                bail!("Found sybmol kind {kind:?} whose p_end value ({p_end}) is less than the position of the current symbol ({sym_pos}).");
            }
            block.p_end.set(p_end - sym_pos);

            // Similarly, fix up the parent pointer (convert it to a relative pointer).
            // Be careful not to change the value for top-level blocks.
            let p_parent = block.p_parent.get();
            if depth != 0 {
                if p_parent >= sym_pos {
                    bail!("Found symbol {kind:?} whose p_parent value ({p_parent}) is later than the current symbol position ({sym_pos}).");
                }
                block.p_parent.set(sym_pos - p_parent);
            } else {
                // If a symbol starts a root symbol scope, then its parent pointer should be zero.
                if p_parent != 0 {
                    bail!("Found symbol {kind:?} whose p_parent value ({p_parent}) is non-zero, which is illegal because this is a top-level symbol.");
                }
            }

            depth += 1;
        }

        if sym.kind.ends_scope() {
            if depth == 0 {
                bail!("sym record at pos {sym_pos} ends a scope, but we're not inside a scope");
            };
            depth -= 1;
        }
    }

    if depth != 0 {
        bail!("Module symbol stream ended without all symbol scopes being closed.");
    }

    // num_scopes counts the number of top-level scopes found
    let num_scopes = scope_starts.len();
    scope_starts.push(syms_data.len() as u32);

    let scope_data = |scope_index: u32| -> &[u8] {
        &syms_data[scope_starts[scope_index as usize] as usize
            ..scope_starts[scope_index as usize + 1] as usize]
    };

    // Create permutation vector and sort it.
    debug!("Sorting top-level symbols scopes");
    let mut scope_order: Vec<u32> = identity_permutation_u32(num_scopes);

    match options.sorting_mode {
        SymbolSortingMode::Bytes => {
            scope_order.sort_unstable_by(|&a, &b| {
                let scope_data_a = scope_data(a);
                let scope_data_b = scope_data(b);
                scope_data_a.cmp(scope_data_b)
            });
        }

        SymbolSortingMode::KindBytes => {
            scope_order.sort_unstable_by(|&a, &b| {
                let scope_data_a = scope_data(a);
                let scope_data_b = scope_data(b);
                let sym_a = SymIter::one(scope_data_a).unwrap();
                let sym_b = SymIter::one(scope_data_b).unwrap();
                match Ord::cmp(&sym_a.kind, &sym_b.kind) {
                    Ordering::Equal => scope_data_a.cmp(scope_data_b),
                    c => c,
                }
            });
        }

        SymbolSortingMode::KindName => {
            // scope_sort_keys contains the keys that we will use for sorting symbols.
            // The first value is the symbol kind.
            // The second value is the "unique name" of a symbol, if it has one, or &[] if not.
            // The third value is the encoded bytes of the symbol (payload), which are the fallback.
            let mut scope_sort_keys: Vec<(u16, &[u8], &[u8])> = Vec::with_capacity(num_scopes);
            for w in scope_starts.windows(2) {
                let this_scope_data = &syms_data[w[0] as usize..w[1] as usize];
                let sym = SymIter::new(this_scope_data).next().unwrap();
                let sym_data = SymData::parse(sym.kind, sym.data)?;
                let name: &[u8] = if let Some(n) = sym_data.name() {
                    n
                } else {
                    &[]
                };
                scope_sort_keys.push((sym.kind.0, name, sym.data));
            }

            scope_order.sort_unstable_by(|&a, &b| {
                let scope_key_a = &scope_sort_keys[a as usize];
                let scope_key_b = &scope_sort_keys[b as usize];
                Ord::cmp(scope_key_a, scope_key_b)
            });
        }
    }

    // Build the new stream data.
    let mut new_sym_data: Vec<u8> = Vec::with_capacity(syms_data.len());

    // Write signature field.
    new_sym_data.extend_from_slice(&signature);

    for &scope_index in scope_order.iter() {
        let source_data = scope_data(scope_index);
        let old_scope_offset = scope_starts[scope_index as usize];
        let new_scope_offset = new_sym_data.len() as u32;

        // Copy this scope to the output vector.
        new_sym_data.extend_from_slice(source_data);

        // Fix up the pointers inside the symbol records within this scope.

        let scope_out = &mut new_sym_data[new_scope_offset as usize..]; // get the scope we just wrote
        let mut depth: u32 = 0;

        for (sym_range_within_scope, sym) in SymIterMut::new(scope_out).with_ranges() {
            // the offset of this symbol in the output stream
            let old_offset = old_scope_offset + sym_range_within_scope.start as u32;
            let new_offset = new_scope_offset + sym_range_within_scope.start as u32;

            if sym.kind.is_refsym_target() {
                module_refsym_remapping.push(ModuleRefSymMapping {
                    new_module,
                    old_module,
                    old_offset,
                    new_offset,
                });
            }

            if sym.kind.starts_block() {
                let mut p = ParserMut::new(sym.data);
                let block: &mut syms::BlockHeader = p.get_mut()?;

                // Fix p_parent
                if depth != 0 {
                    let p_parent_relative = block.p_parent.get();
                    block.p_parent.set(new_offset - p_parent_relative);
                }

                // Fix p_end
                let relative_p_end = block.p_end.get();
                block.p_end.set(new_offset + relative_p_end);

                depth += 1;
            } else if sym.kind.ends_scope() {
                // This should have been caught above, during the first pass.
                assert!(depth > 0);
                depth -= 1;
            }
        }
    }

    Ok(new_sym_data)
}

/// Parses each symbol so that we can find the last valid byte, then writes zeroes to the bytes
/// within the record data that are not covered by the symbol parser.
///
/// This is provided for experimentation, such as comparing the effectiveness of the determinism
/// algorithm. It should not be performed on production symbol streams.
pub fn clean_trailing_bytes(symbols: &mut [u8]) {
    for sym in SymIterMut::new(symbols) {
        let mut p = Parser::new(sym.data);

        // We parse the symbol data, just so that we can find the unused bytes at the end.
        match SymData::from_parser(sym.kind, &mut p) {
            Ok(_sym_data) => {
                let sym_len = sym.data.len();
                let unused_len = p.len();
                for b in sym.data[sym_len - unused_len..].iter_mut() {
                    *b = 0;
                }
            }
            Err(_) => {}
        }
    }
}

/// Sorts symbol records in the Global Symbol Stream (GSS).
///
/// There are no symbol records in the GSS that start symbol scopes. This makes sorting the GSS
/// easier than sorting module symbol streams.
///
/// We often see many duplicate records in the GSS, such as duplicates of `S_PUB32` records.
/// This function can optionally de-duplicate symbol records. Deduplicating is simplified by not
/// needing to deal with symbol scopes, which is one reason why we only apply deduplication the
/// the GSS and not to module streams.
///
/// This function returns a mapping table that maps from byte offsets in the old symbols stream
/// to byte offsets in the new symbols stream. There is one record in this array for each root-level
/// symbol that starts a scope; this table does not contain record offsets for symbols that are
/// nested within those scopes.
pub fn sort_global_syms(
    sorting_mode: SymbolSortingMode,
    syms_data: &[u8],
) -> anyhow::Result<(Vec<u8>, RemappedSymbolTable)> {
    // sym_locations is a "starts" vector.
    let mut sym_locations: Vec<u32> = Vec::new();

    for (sym_range, _sym) in SymIter::new(syms_data).with_ranges() {
        sym_locations.push(sym_range.start as u32);
    }

    // num_syms counts the number of top-level scopes found
    let num_syms = sym_locations.len();
    sym_locations.push(syms_data.len() as u32);

    let record_data = |i: u32| -> &[u8] {
        &syms_data[sym_locations[i as usize] as usize..sym_locations[i as usize + 1] as usize]
    };

    // Create permutation vector and sort it.
    let mut order: Vec<u32> = identity_permutation_u32(num_syms);

    match sorting_mode {
        SymbolSortingMode::Bytes => {
            order.sort_unstable_by(|&a, &b| {
                let record_data_a = record_data(a);
                let record_data_b = record_data(b);
                record_data_a.cmp(record_data_b)
            });
        }

        SymbolSortingMode::KindBytes => {
            order.sort_unstable_by(|&a, &b| {
                let record_data_a = record_data(a);
                let record_data_b = record_data(b);
                let sym_a = SymIter::new(record_data_a).next().unwrap();
                let sym_b = SymIter::new(record_data_b).next().unwrap();
                match Ord::cmp(&sym_a.kind, &sym_b.kind) {
                    Ordering::Equal => record_data_a.cmp(record_data_b),
                    c => c,
                }
            });
        }

        SymbolSortingMode::KindName => {
            // scope_sort_keys contains the keys that we will use for sorting symbols.
            // The first value is the symbol kind.
            // The second value is the "unique name" of a symbol, if it has one, or &[] if not.
            // The third value is the encoded bytes of the symbol (payload), which are the fallback.
            let mut sorting_keys: Vec<(u16, &[u8], &[u8])> = Vec::with_capacity(num_syms);
            for sym in SymIter::new(syms_data) {
                let sym_data = SymData::parse(sym.kind, sym.data)?;
                let name: &[u8] = if let Some(n) = sym_data.name() {
                    n
                } else {
                    &[]
                };
                sorting_keys.push((sym.kind.0, name, sym.data));
            }

            order.sort_unstable_by(|&a, &b| {
                let scope_key_a = &sorting_keys[a as usize];
                let scope_key_b = &sorting_keys[b as usize];
                Ord::cmp(scope_key_a, scope_key_b)
            });
        }
    }

    // Build the new stream data.
    let new_sym_data_len = syms_data.len();
    let mut new_sym_data: Vec<u8> = Vec::with_capacity(new_sym_data_len);
    let mut remapping: Vec<RemappedSymbolEntry> = Vec::with_capacity(num_syms);

    // Iterate through the records in the new record order.
    // Compare adjacent records. If we find an identical record, remove the record.
    // Write remapping entries as we go, including remapping duplicate records that were removed.

    let mut last_record_written: Option<&[u8]> = None;
    let mut last_record_offset: u32 = 0xffff_ffff;
    let mut num_dups_removed: u32 = 0;
    let mut size_dups_removed: usize = 0;

    for &old_index in order.iter() {
        let old_start = sym_locations[old_index as usize] as usize;
        let old_end = sym_locations[old_index as usize + 1] as usize;
        let record_data = &syms_data[old_start..old_end];

        // Is this record equivalent to the last record that we wrote to the output?
        if let Some(last) = last_record_written {
            if record_data == last {
                // Found a duplicate record.
                remapping.push(RemappedSymbolEntry {
                    old_offset: old_start as u32,
                    new_offset: last_record_offset,
                });
                num_dups_removed += 1;
                size_dups_removed += record_data.len();
                continue;
            }
        }

        let new_offset = new_sym_data.len() as u32;
        remapping.push(RemappedSymbolEntry {
            old_offset: old_start as u32,
            new_offset,
        });

        last_record_written = Some(record_data);
        last_record_offset = new_offset;

        // Copy the record to the output.
        new_sym_data.extend_from_slice(record_data);
    }

    if num_dups_removed != 0 {
        debug!(
            "Number of duplicate symbols removed from Global Symbol Stream: {num_dups_removed}, {size_dups_removed} bytes"
        );
    }

    // Sort our remapping table by old offset, so that users of this remapping can query it for
    // forward mappings (old --> new). We added these entries in "new" order, which is why we need
    // to sort it.
    remapping.sort_unstable_by_key(|e| e.old_offset);

    Ok((new_sym_data, RemappedSymbolTable { vec: remapping }))
}
