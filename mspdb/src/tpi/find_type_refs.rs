//! Support for walking the type reference graph within type streams and symbol streams.

use super::TypeStream;
use crate::names::NameIndex;
use crate::parser::ParserError;
use crate::tpi::TypeStreamKind;
use crate::types::visitor::{visit_type_indexes_in_record_slice, IndexVisitor};
use crate::types::{ItemId, TypeIndex, TypesIter};
use crate::utils::iter::IteratorWithRangesExt;
use anyhow::bail;
use dump_utils::HexDump;
use std::mem::size_of_val;
use std::ops::Range;
use tracing::{debug, error, warn};

/// State used to walk the type reference graph.
pub struct FindTypeRefsState {
    /// The type refs table that we are building. Contains `(t, u)` where `t --> u`.
    ///
    /// The values stored in this table start at the first type index.  That is, to convert from
    /// the values in this table to `TypeIndex`, you must add `type_index_begin` to them.
    ///
    /// For each pair `(t, u)`, `t` contains a pointer to `u`.  `t` should have a numerically
    /// higher value than `u`.
    pub type_refs: Vec<(u32, u32)>,

    /// The location of each (stored) type.
    ///
    /// `type_locations[0]` will have an offset equal to the size of the stream header.  Records
    /// are contiguous (do not have gaps between them), so `type_locations[i + 1] - type_locations[i]`
    /// can be used to find the length of a given type record.  There is an additional value pushed
    /// at the end, which is the stream length, to aid in just such calculations.
    pub type_locations: Vec<u32>,

    /// The rank of each type record.
    ///
    /// Type records that do not point to any other type record have a rank of 0.
    /// Type records which contain only pointers to other type records having a rank of 0, have a
    /// rank of 1. Etc.
    pub type_ranks: Vec<u32>,

    /// The number of type references that were misordered.
    pub num_misordered_type_refs: u64,

    /// The total number of type references (`TypeIndex` values) that were found.
    pub num_type_references_found: u64,

    /// The total number of types found.
    pub num_types_found: u64,

    /// The type index of the first value stored in the type stream.
    pub type_index_begin: TypeIndex,

    /// 1 + the type index of the last record stored in the type stream.
    pub type_index_end: TypeIndex,
}

impl FindTypeRefsState {
    /// Gets the range of bytes for a given type record.
    #[allow(dead_code)]
    pub fn get_type_record_range(&self, i: u32) -> Range<usize> {
        let start = self.type_locations[i as usize] as usize;
        let end = self.type_locations[i as usize + 1] as usize;
        start..end
    }
}

/// Find type references.
#[inline(never)]
pub fn find_the_type_records(
    stream_kind: TypeStreamKind,
    type_stream: &TypeStream<Vec<u8>>,
) -> Result<FindTypeRefsState, anyhow::Error> {
    let types_data = type_stream.type_records_bytes();
    let type_index_begin = type_stream.type_index_begin();
    let type_index_end = type_stream.type_index_end();
    find_the_type_records_core(stream_kind, types_data, type_index_begin, type_index_end)
}

/// Find type references.
///
/// This is the first pass through the type table.  We scan all types and build a vector of type
/// references.  This table provides constraints on reordering types; for types `t` and `u`, if
/// type `t` refers to type `u`, then `t` _must_ come after `u` in the final output.
///
/// Scan the types stream (TPI stream) and find all of the places where one record points to another.
/// These pointers give us constraints; if type A points to type B, then (by the requirements
/// of the PDB spec) type A must come after type B in the type stream.  We use these to constrain
/// our sorting order.
#[inline(never)]
pub fn find_the_type_records_core(
    stream_kind: TypeStreamKind,
    records: &[u8],
    type_index_begin: TypeIndex,
    type_index_end: TypeIndex,
) -> Result<FindTypeRefsState, anyhow::Error> {
    if type_index_begin.0 == 0 || type_index_begin > type_index_end {
        bail!("TPI stream has invalid type_index_begin / type_index_end fields.");
    }
    let num_types = (type_index_end.0 - type_index_begin.0) as usize;

    if type_index_begin.0 != 0x1000 {
        warn!(
            "TPI stream uses a non-default value for type_index_begin: 0x{:x}",
            type_index_begin.0
        );
    }

    debug!(
        "TypeIndex range: 0x{:x} .. 0x{:x}",
        type_index_begin.0, type_index_end.0
    );
    debug!("Number of defined types: {num_types}");

    let mut state = FindTypeRefsState {
        type_locations: Vec::new(),
        num_misordered_type_refs: 0,
        type_refs: Vec::new(),
        num_types_found: 0,
        num_type_references_found: 0,
        type_index_begin,
        type_index_end,
        type_ranks: Vec::with_capacity(num_types),
    };

    let mut num_errors: u64 = 0;

    debug!("Scanning type stream for record-to-record references...");
    let original_buffer_len = records.len();
    let type_index_begin = state.type_index_begin;

    // The types referenced by the current type record that we are processing.
    // We build this list, then sort it and dedup it at the end of each type record that we process.
    // This is a "stored type index".
    let mut current_refs: Vec<u32> = Vec::with_capacity(256);

    let mut max_rank = 0;

    for (i, (record_range, record)) in TypesIter::new(records).with_ranges().enumerate() {
        if i == num_types {
            error!("The type stream contains more records than the TPI stream header specified.");
            // TODO: The length of the last record (in type_locations) will be inaccurate.
            break;
        }

        assert!(current_refs.is_empty());

        let type_index_relative = i as u32;
        let type_index = TypeIndex(type_index_begin.0 + type_index_relative);
        let type_record_location = record_range.start as u32;

        let mut this_type_rank: u32 = 0;

        struct FindViz<'a> {
            state: &'a mut FindTypeRefsState,
            type_index_begin: TypeIndex,
            current_refs: &'a mut Vec<u32>,
            this_type_rank: &'a mut u32,
            type_index: TypeIndex,
            stream_kind: TypeStreamKind,
        }

        impl<'a> FindViz<'a> {
            fn do_ref(&mut self, u: TypeIndex) {
                if u < self.type_index_begin {
                    // There is no need to store primitive type references.
                    return;
                }

                let u_rel = u.0 - self.type_index_begin.0;
                if u >= self.type_index {
                    // We found a misordered type reference.
                    self.state.num_misordered_type_refs += 1;
                    warn!("found misordered type reference");
                    return;
                }

                // What is the rank of u?
                let u_rank = self.state.type_ranks[u_rel as usize];
                let u_rank_plus_one = u_rank + 1;

                if u_rank_plus_one > *self.this_type_rank {
                    *self.this_type_rank = u_rank_plus_one;
                }

                self.current_refs.push(u_rel);
            }
        }

        impl<'a> IndexVisitor for FindViz<'a> {
            fn item_id(&mut self, _offset: usize, value: ItemId) -> Result<(), ParserError> {
                match self.stream_kind {
                    TypeStreamKind::TPI => {
                        // We are in the TPI and we found a record that contains ItemId.
                        // Those records should not be in the TPI.
                        error!("found an ItemID in the TPI; this should never happen");
                        Err(ParserError::new())
                    }

                    TypeStreamKind::IPI => {
                        // We are in the IPI and we found a pointer to another record in the IPI.
                        self.do_ref(TypeIndex(value));
                        Ok(())
                    }
                }
            }

            fn type_index(&mut self, _offset: usize, u: TypeIndex) -> Result<(), ParserError> {
                match self.stream_kind {
                    TypeStreamKind::TPI => {
                        // We're in the TPI and we found a TypeIndex.
                        self.do_ref(u);
                        Ok(())
                    }

                    TypeStreamKind::IPI => {
                        // We are in the IPI.  The IPI contains TypeIndex values, but we are not
                        // concerned with them in this context, because we only care about pointers
                        // between records within the same type stream.  When a record in the IPI
                        // contains a TypeIndex value, it points into a different stream.  There are
                        // no ordering constraints in that situation.
                        Ok(())
                    }
                }
            }

            fn name_index(&mut self, _offset: usize, _value: NameIndex) -> Result<(), ParserError> {
                Ok(())
            }
        }

        match visit_type_indexes_in_record_slice(
            record.kind,
            record.data,
            FindViz {
                current_refs: &mut current_refs,
                state: &mut state,
                type_index_begin,
                this_type_rank: &mut this_type_rank,
                type_index,
                stream_kind,
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                num_errors += 1;
                if num_errors <= 100 {
                    error!("failed to parse type record: (at offset {type_record_location}) {type_index:?} {:?}\n\
                        {e}\n\
                        {:#?}", record.kind,
                    HexDump::new(record.data));
                }
            }
        }

        // Now that we have accumulated the edges generated by this type record, sort them and
        // dedup them. Then copy them into the edge vector (state.type_refs).
        if !current_refs.is_empty() {
            state.num_type_references_found += current_refs.len() as u64;
            current_refs.sort_unstable();
            current_refs.dedup();

            for &u in current_refs.iter() {
                state.type_refs.push((type_index_relative, u));
            }
            current_refs.clear();
        }

        // Remember the location of this type record, so we can easily find it later during when
        // sorting type records.
        state.type_locations.push(type_record_location);

        state.type_ranks.push(this_type_rank);
        max_rank = max_rank.max(this_type_rank);

        state.num_types_found += 1;
    }

    if state.type_ranks.len() != num_types {
        bail!("Type stream did not contain the expected number of records");
    }

    assert_eq!(state.type_locations.len(), num_types);

    // Check whether the type_refs vector is sorted. We expect it to be, but let's verify.
    crate::sort_utils::assert_is_sorted_and_unique(&state.type_refs);

    debug!("Max rank: {}", max_rank);

    // Add an extra record to the type_locations table so that it is easy to compute the byte range
    // for a type record.
    state.type_locations.push(original_buffer_len as u32);

    if num_errors != 0 {
        error!("Total number of errors: {num_errors}");
    }

    debug!("Finished finding type refs.");
    debug!(
        "Number of types found:                    {:8}",
        state.num_types_found
    );
    debug!(
        "Number of type references found (all):    {:8}",
        state.num_type_references_found
    );
    debug!(
        "Number of type references found (unique): {:8}",
        state.type_refs.len()
    );

    if state.num_misordered_type_refs != 0 {
        error!(
            "Number of misordered type references: {}",
            state.num_misordered_type_refs
        );
    }

    debug!(
        "Size of type reference table: {:12} (bytes)",
        size_of_val(state.type_refs.as_slice())
    );
    debug!(
        "Size of type locations table: {:12} (bytes)",
        size_of_val(state.type_locations.as_slice())
    );

    Ok(state)
}
