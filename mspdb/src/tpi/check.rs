//! Code for checking the invariants of a Type Stream

use super::{TypeStream, TypeStreamHeader};
use crate::diag::Diags;
use crate::names::NameIndex;
use crate::parser::{Parser, ParserError};
use crate::sort_utils::sort_map;
use crate::tpi::hash::hash_type_record;
use crate::types::visitor::{visit_type_indexes_in_record_slice, IndexVisitor};
use crate::types::{build_types_starts, ItemId, Leaf, TypeIndex, TypesIter};
use crate::utils::iter::IteratorWithRangesExt;
use crate::{Pdb, ReadAt, Stream};
use anyhow::bail;
use dump_utils::{HexDump, HexStr};
use std::collections::HashMap;
use std::fmt::Write;
use std::mem::size_of;
use tracing::{debug, error, info, warn};
use zerocopy::{AsBytes, LE, U32};

/// Verifies that all TypeIndex values within a Type Stream are well-ordered.
pub fn check_tpi_type_ordering(
    type_index_begin: TypeIndex,
    _type_index_end: TypeIndex,
    records_offset: usize,
    records: &[u8],
) -> anyhow::Result<()> {
    let mut errors: Vec<(u32, BadReason)> = Vec::new();

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum BadReason {
        ItemIdInWrongStream,
        TypeIndexForward,
    }

    let max_errors = 25;

    struct CheckTpiOrderVisitor<'a> {
        type_index_begin: TypeIndex,
        this_type_index: TypeIndex,
        errors: &'a mut Vec<(u32, BadReason)>,
    }

    impl<'a> IndexVisitor for CheckTpiOrderVisitor<'a> {
        fn item_id(&mut self, offset: usize, _value: ItemId) -> Result<(), ParserError> {
            self.errors
                .push((offset as u32, BadReason::ItemIdInWrongStream));
            Ok(())
        }

        fn type_index(&mut self, offset: usize, t: TypeIndex) -> Result<(), ParserError> {
            if t < self.type_index_begin {
                return Ok(());
            }

            if t < self.this_type_index {
                // good
            } else {
                self.errors
                    .push((offset as u32, BadReason::TypeIndexForward));
            }

            Ok(())
        }

        fn name_index(&mut self, _offset: usize, _value: NameIndex) -> Result<(), ParserError> {
            Ok(())
        }
    }

    for (i, (record_range, r)) in TypesIter::new(records).with_ranges().enumerate() {
        let this_type_index = TypeIndex(type_index_begin.0 + i as u32);
        let errors_len_before = errors.len();

        match visit_type_indexes_in_record_slice(
            r.kind,
            r.data,
            CheckTpiOrderVisitor {
                this_type_index,
                type_index_begin,
                errors: &mut errors,
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                bail!("failed to decode type record: {}", e);
            }
        }
        // add in the stream offsets
        for e in errors[errors_len_before..].iter_mut() {
            // +4 for stepping over the record header
            e.0 += records_offset as u32 + record_range.start as u32 + 4;
        }

        if errors.len() > max_errors {
            break;
        }
    }

    if !errors.is_empty() {
        let mut msg = String::new();
        writeln!(
            msg,
            "Found invalid TypeIndex and/or ItemId values within the IPI stream. Errors:"
        )
        .unwrap();
        for e in errors.iter() {
            writeln!(msg, "[{:08x}] {:?}", e.0, e.1).unwrap();
        }
        bail!("{}", msg);
    }

    Ok(())
}
/// Verifies that all `ItemId` values within an IPI Stream are well-ordered.
pub fn check_ipi_type_ordering(
    _type_index_begin: TypeIndex,
    type_index_end: TypeIndex,
    id_begin: ItemId,
    id_end: ItemId,
    records_offset: usize,
    records: &[u8],
) -> anyhow::Result<()> {
    let mut errors: Vec<(u32, ItemId, BadReason, u32)> = Vec::new();

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    enum BadReason {
        IdInvalidLow,
        IdInvalidForward,
        IdInvalidHigh,
        TypeIndexOutOfRange,
    }

    let max_errors = 25;

    struct CheckIpiOrderVisitor<'a> {
        type_index_end: TypeIndex,
        this_id: ItemId,
        id_begin: ItemId,
        id_end: ItemId,
        errors: &'a mut Vec<(u32, ItemId, BadReason, u32)>,
    }

    impl<'a> IndexVisitor for CheckIpiOrderVisitor<'a> {
        fn item_id(&mut self, offset: usize, id: ItemId) -> Result<(), ParserError> {
            // We are scanning the IPI.  Check the ordering of ItemId references that we find.

            if id == 0 {
                // It's a "null" ID. These are always acceptable.
            } else if id < self.id_begin {
                // It's in the range of what would be "primitive types" in the TPI stream.
                // But the IPI stream has no such concept, so this is an invalid index.
                self.errors
                    .push((offset as u32, self.this_id, BadReason::IdInvalidLow, id));
            } else if id < self.id_end {
                // It's within the range of valid indexes. Is it correctly ordered?
                if id < self.this_id {
                    // Yes, it's a valid (backward-pointing) index.
                } else {
                    // No, it is beyond the acceptable range, because it either points to the
                    // current record or points forward in the record stream.
                    self.errors.push((
                        offset as u32,
                        self.this_id,
                        BadReason::IdInvalidForward,
                        id,
                    ));
                }
            } else {
                // It's beyond the range of valid indexes.
                self.errors
                    .push((offset as u32, self.this_id, BadReason::IdInvalidHigh, id));
            }

            Ok(())
        }

        fn type_index(&mut self, offset: usize, t: TypeIndex) -> Result<(), ParserError> {
            // We are scanning the IPI, which means that TypeIndex values that we find point
            // into the TPI (a separate stream). There are no ordering constraints, but there is
            // a requirement that the TypeIndex is in a valid range.
            if t >= self.type_index_end {
                self.errors.push((
                    offset as u32,
                    self.this_id,
                    BadReason::TypeIndexOutOfRange,
                    t.0,
                ));
            }

            Ok(())
        }

        fn name_index(&mut self, _offset: usize, _value: NameIndex) -> Result<(), ParserError> {
            Ok(())
        }
    }

    for (i, (record_range, r)) in TypesIter::new(records).with_ranges().enumerate() {
        let this_id = id_begin + i as u32;
        let errors_len_before = errors.len();

        match visit_type_indexes_in_record_slice(
            r.kind,
            r.data,
            CheckIpiOrderVisitor {
                this_id,
                type_index_end,
                id_begin,
                id_end,
                errors: &mut errors,
            },
        ) {
            Ok(()) => {}
            Err(e) => {
                bail!("failed to decode type record: {}", e);
            }
        }

        // add in the stream offsets
        for e in errors[errors_len_before..].iter_mut() {
            // +4 for stepping over the record header
            e.0 += records_offset as u32 + record_range.start as u32 + 4;
        }

        if errors.len() > max_errors {
            break;
        }
    }

    if !errors.is_empty() {
        let mut msg = String::new();
        writeln!(
            msg,
            "Found invalid TypeIndex and/or ItemId values within the IPI stream. Errors:"
        )
        .unwrap();
        for e in errors.iter() {
            writeln!(msg, "[{:08x}] id: {:08x} - {:?} {:08x}", e.0, e.1, e.2, e.3).unwrap();
        }
        bail!("{}", msg);
    }

    Ok(())
}

/// Thoroughly check invariants of the TPI Stream and its related TPI Hash Stream (if present).
pub fn check_tpi_stream<F: ReadAt>(
    pdb: &Pdb<F>,
    diags: &mut Diags,
) -> anyhow::Result<TypeStream<Vec<u8>>> {
    let tpi = pdb.read_type_stream()?;
    let Some(tpi_header) = tpi.header() else {
        return Ok(tpi);
    };

    check_type_stream(&tpi, diags);

    check_tpi_type_ordering(
        tpi.type_index_begin(),
        tpi.type_index_end(),
        tpi.type_records_range().start,
        tpi.type_records_bytes(),
    )?;

    let type_record_starts: Vec<u32> =
        build_types_starts(tpi.num_types() as usize, tpi.type_records_bytes());
    check_type_hash_stream_data(
        pdb,
        diags,
        Stream::TPI,
        tpi_header,
        tpi.type_records_bytes(),
        &type_record_starts,
    )?;

    Ok(tpi)
}

/// Thoroughly check invariants of the IPI Stream and its related TPI Hash Stream (if present).
pub fn check_ipi_stream<F: ReadAt>(
    pdb: &Pdb<F>,
    diags: &mut Diags,
    type_index_begin: TypeIndex,
    type_index_end: TypeIndex,
) -> anyhow::Result<()> {
    let ipi = pdb.read_ipi_stream()?;
    let Some(ipi_header) = ipi.header() else {
        return Ok(());
    };

    check_type_stream(&ipi, diags);

    check_ipi_type_ordering(
        type_index_begin,
        type_index_end,
        ipi.type_index_begin().0,
        ipi.type_index_end().0,
        ipi.type_records_range().start,
        ipi.type_records_bytes(),
    )?;

    let type_record_starts: Vec<u32> =
        build_types_starts(ipi.num_types() as usize, ipi.type_records_bytes());
    check_type_hash_stream_data(
        pdb,
        diags,
        Stream::IPI,
        ipi_header,
        ipi.type_records_bytes(),
        &type_record_starts,
    )?;

    Ok(())
}

/// Thoroughly check invariants of a Type Stream.
pub fn check_type_stream(tpi: &TypeStream<Vec<u8>>, diags: &mut Diags) {
    // It is legal for a type stream to be zero-length.
    // We see zero-length IPI streams, although we have never seen a zero-length TPI stream.
    if tpi.stream_data.is_empty() {
        return;
    }

    let mut p = Parser::new(tpi.stream_data.as_ref());
    let Ok(type_stream_header) = p.get::<TypeStreamHeader>() else {
        diags.error("The Type Stream is too small to contain a valid Type Stream Header.");
        return;
    };

    let header_size = type_stream_header.header_size.get() as usize;
    if header_size < size_of::<TypeStreamHeader>() {
        diags.error(format!("The Type Stream Header specifies a header_size ({header_size}) that is too small to be valid."));
        return;
    };

    // Verify that the Type Index range is valid.
    let type_index_begin = type_stream_header.type_index_begin.get();
    if type_index_begin < TypeIndex::MIN_BEGIN {
        diags.error(format!(
            "The Type Stream has an invalid value for type_index_begin ({type_index_begin:?}). \
             It is less than the minimum required value ({}).",
            TypeIndex::MIN_BEGIN.0
        ));
        return;
    }

    let type_index_end = type_stream_header.type_index_end.get();
    if type_index_end < type_index_begin {
        diags.error(format!(
            "The Type Stream has an invalid value for type_index_end ({type_index_end:?}). \
            It is less than type_index_begin ({type_index_begin:?})."
        ));
        return;
    }

    // Verify that the Types Record range is valid.

    if header_size > tpi.stream_data.len() {
        diags.error(format!(
            "The Type Stream Header specifies a header_size ({header_size}) that is larger than the \
            size of the stream."));
        return;
    }
    let max_types_size = tpi.stream_data.len() - header_size;

    let types_size = type_stream_header.type_record_bytes.get() as usize;
    if types_size > max_types_size {
        diags.error(format!(
            "The Type Stream Header specifies a types_size ({types_size}) that exceeds the \
             remaining data in the stream ({max_types_size})."
        ));
        return;
    }

    // It would be convient if type_sizes was always 4-byte aligned, but that is not the case.
    // Observationally, we see PDBs that have type_sizes values that are always multiples of 2,
    // but are not guaranteed to be multiples of 4.
    if types_size % 2 != 0 {
        diags.error(format!(
            "The Type Stream is invalid. The Type Stream Header specifies a types_size \
             ({types_size}) that is not aligned to 2-byte boundaries."
        ));
    }
}

fn check_type_hash_stream_data<F: ReadAt>(
    pdb: &Pdb<F>,
    diags: &mut Diags,
    stream: Stream,
    type_stream_header: &TypeStreamHeader,
    type_records_bytes: &[u8],
    type_record_starts: &[u32],
) -> anyhow::Result<()> {
    // Read the Type Hash Stream and validate its contents.
    if let Some(hash_stream_index) = type_stream_header.hash_stream_index.get() {
        debug!("Checking Type Hash Stream");

        let type_hash_stream_data = pdb.read_stream_to_vec(hash_stream_index)?;

        macro_rules! check_range {
            ($offset:ident, $length:ident) => {{
                let offset = type_stream_header.$offset.get() as usize;
                let length = type_stream_header.$length.get() as usize;
                if offset > type_hash_stream_data.len()
                    || length > type_hash_stream_data.len() - offset
                {
                    diags.error(format!(
                        "Type Stream Header specifies {} = {}, {} = {}, but this range is beyond \
                         the size of the Type Hash Stream ({}).",
                        stringify!($offset),
                        offset,
                        stringify!($length),
                        length,
                        type_hash_stream_data.len()
                    ));
                }

                debug!(
                    "0x{offset:08x} + 0x{length:08x} --> {}",
                    stringify!($offset)
                );

                &type_hash_stream_data[offset..offset + length]
            }};
        }

        let hash_value_buffer = check_range!(hash_value_buffer_offset, hash_value_buffer_length);
        let index_offset_buffer =
            check_range!(index_offset_buffer_offset, index_offset_buffer_length);
        let _hash_adj_buffer = check_range!(hash_adj_buffer_offset, hash_adj_buffer_length);

        check_hash_values_substream(type_stream_header, type_records_bytes, hash_value_buffer)?;

        check_hash_index_offset_buffer(
            stream,
            type_stream_header,
            index_offset_buffer,
            type_record_starts,
        )?;
    } else {
        debug!("This Type Stream does not have an associated Type Hash Stream.");

        // If there is no Type Hash Stream, then we expect all of the related values in the Type
        // Stream Header to be zero.
        // TODO: Reader, they are not zero.

        if false {
            macro_rules! check_zero {
                ($name:ident) => {
                    if type_stream_header.$name.get() != 0 {
                        diags.error(format!(
                            "The Type Stream Header does not have a Type Hash Stream, but it does \
                         specify a non-zero value for the related field '{}': {}",
                            stringify!($name),
                            type_stream_header.$name.get()
                        ));
                    }
                };
            }

            check_zero!(hash_key_size);
            check_zero!(num_hash_buckets);
            check_zero!(hash_value_buffer_offset);
            check_zero!(hash_value_buffer_length);
            check_zero!(index_offset_buffer_offset);
            check_zero!(index_offset_buffer_length);
            check_zero!(hash_adj_buffer_offset);
            check_zero!(hash_adj_buffer_length);
        }
    }

    Ok(())
}

fn check_hash_values_substream(
    type_stream_header: &TypeStreamHeader,
    type_records_bytes: &[u8],
    hash_value_buffer: &[u8],
) -> anyhow::Result<()> {
    debug!("Checking Hash Value Substream");
    debug!("Hash Value Buffer size = {}", hash_value_buffer.len());
    debug!("    hash_key_size = {}", type_stream_header.hash_key_size);
    debug!(
        "    num_hash_buckets (from header) = 0x{:x}",
        type_stream_header.num_hash_buckets
    );

    let type_index_begin = type_stream_header.type_index_begin.get();
    let header_size = type_stream_header.header_size.get() as usize;

    if hash_value_buffer.is_empty() {
        debug!("The Hash Value Substream is empty.");
        return Ok(());
    }

    let num_types =
        type_stream_header.type_index_end.get().0 - type_stream_header.type_index_begin.get().0;

    const EXPECTED_HASH_KEY_SIZE: usize = 4;

    let hash_key_size = type_stream_header.hash_key_size.get();
    if hash_key_size as usize != EXPECTED_HASH_KEY_SIZE {
        bail!(
            "The Type Stream Header specifies hash_key_size = {hash_key_size}, which is not \
                supported. The only supported value is {EXPECTED_HASH_KEY_SIZE}."
        );
    }

    let num_hash_buckets = type_stream_header.num_hash_buckets.get();
    if num_hash_buckets == 0 {
        bail!("Type Stream Header specifies num_hash_buckets == 0, which is not allowed.");
    }

    if hash_value_buffer.len() % EXPECTED_HASH_KEY_SIZE != 0 {
        bail!("The Hash Value Substream (within the Type Hash Stream) has a size ({}) that is not a multiple of hash_key_size ({hash_key_size}).",
            hash_value_buffer.len());
    }

    let num_hashes = hash_value_buffer.len() / EXPECTED_HASH_KEY_SIZE;
    debug!("Number of hashes: {num_hashes}");

    if num_hashes != num_types as usize {
        bail!(
            "The number of hashes in the Hash Value Substream (in the Type Hash Stream) is
                 {num_hashes}, which does not match the number of type records {num_types}.",
        );
    }

    // We just checked the length/alignment, above, so this unwrap() should succeed.
    let hashes: &[U32<LE>] = zerocopy::Ref::new_slice_unaligned(hash_value_buffer)
        .unwrap()
        .into_slice();

    let report_error_details = false;

    let mut num_bad_shown: u32 = 0;

    // Show some of the hashes, with the matching records.
    let mut bad_record_offsets: Vec<usize> = Vec::new(); // stream offsets of bad records.
    let mut num_bad_records: u32 = 0;
    let mut num_records_with_alignment_suffix: u32 = 0;
    let mut record_counts_by_kind: HashMap<Leaf, (u32, u32)> = HashMap::new(); // (all, bad)

    let mut num_no_name: u32 = 0;

    for (i, ((record_range, record), hash_from_file_le)) in TypesIter::new(type_records_bytes)
        .with_ranges()
        .zip(hashes.iter())
        .enumerate()
    {
        let record_count = record_counts_by_kind.entry(record.kind).or_default();
        record_count.0 += 1;

        let hash_from_file = hash_from_file_le.get();
        let entire_record_bytes = &type_records_bytes[record_range.clone()];

        let record_data = record.parse()?;
        let Some(sym_name) = record_data.name() else {
            // no name at all
            num_no_name += 1;
            continue;
        };

        // let computed_hash_mod_u32 = hash_mod_u32(sym_name.as_bytes(), num_hash_buckets);
        let computed_hash =
            hash_type_record(record.kind, entire_record_bytes, record.data)? % num_hash_buckets;

        let type_index = TypeIndex(type_index_begin.0 + i as u32);

        let mut is_interesting = false;
        if computed_hash != hash_from_file && num_bad_shown < 5 {
            num_bad_shown += 1;
            is_interesting = true;
        }

        if is_interesting {
            info!(
                "Showing a hash record:
Offset in Type Stream: 0x{pos_in_stream:x}
Offset in Hash Stream: 0x{pos_in_hash_stream:x}
Type Index:            0x{type_index:08x}
Hash stored on disk:   0x{hash_from_file:08x}  {hash_from_file_bytes:?}
Hash computed:         0x{computed_hash:08x}
Record kind:           {kind:?}
Record size:           0x{record_len:x}
sym_name:              {sym_name:?}
sym_name hex:          {sym_name_hex:?}
{data:?}",
                type_index = type_index.0,
                record_len = entire_record_bytes.len(),
                pos_in_stream = header_size + record_range.start,
                pos_in_hash_stream = i * hash_key_size as usize,
                kind = record.kind,
                data = HexDump::new(entire_record_bytes).max(256),
                sym_name_hex = HexStr::new(sym_name.as_bytes()),
                hash_from_file_bytes = HexStr::new(hash_from_file_le.as_bytes()),
            );
        }

        if hash_from_file != computed_hash {
            if has_alignment_suffix(entire_record_bytes) {
                num_records_with_alignment_suffix += 1;
            } else {
                if report_error_details {
                    if num_bad_records < 5 {
                        error!(
                            "Invalid hash value found.
Offset in Type Stream: 0x{pos_in_stream:x}
Type Index:            0x{type_index:08x}
Hash stored on disk:   0x{hash_from_file:08x}
Hash computed:         0x{computed_hash:08x}
Record kind:           {kind:?}
Record size:           0x{record_len:x}
{data:?}",
                            type_index = type_index_begin.0 + i as u32,
                            record_len = entire_record_bytes.len(),
                            pos_in_stream = header_size + record_range.start,
                            kind = record.kind,
                            data = HexDump::new(entire_record_bytes).max(256)
                        );
                    }
                }

                num_bad_records += 1;

                if bad_record_offsets.len() < 20 {
                    bad_record_offsets.push(record_range.start + header_size);
                }

                record_count.1 += 1;
            }
        }
    }

    if num_records_with_alignment_suffix != 0 {
        warn!("Number of records found that have an alignment suffix and a mis-matching hash: {num_records_with_alignment_suffix}");
    }

    if num_bad_records != 0 {
        // TODO: Figure out why the hashes are not matching, then upgrade this to bail!().
        warn!(
            "The Type Hash Substream contains {num_bad_records} records whose hash values \
                do not match the hash of the corresponding record. \
                Some stream offsets of bad records: {:x?}",
            bad_record_offsets
        );
    }

    if num_no_name != 0 {
        debug!("Number of records that could not be hashed because we don't know their name: {num_no_name}");
    }

    if num_records_with_alignment_suffix != 0 || num_bad_records != 0 {
        for (kind, (all, bad)) in sort_map(&record_counts_by_kind).into_iter() {
            warn!("{all:8} total, {bad:8} bad : {kind:?}");
        }
    }

    debug!("Checked {} hash values.", num_types);

    Ok(())
}

fn has_alignment_suffix(b: &[u8]) -> bool {
    if let Some(last) = b.last() {
        (0xf1..=0xf3).contains(last)
    } else {
        false
    }
}

/// Checks the Hash Index Offset Buffer / Substream
fn check_hash_index_offset_buffer(
    stream: Stream,
    type_stream_header: &TypeStreamHeader,
    hash_index_buffer: &[u8],
    type_record_starts: &[u32], // as usual, this includes an entry at the end
) -> anyhow::Result<()> {
    debug!("Checking Hash Index Offsets Substream");

    if hash_index_buffer.is_empty() {
        debug!("The Hash Index Offsets Substream is empty.");
        return Ok(());
    }

    let index_offset_buffer_offset = type_stream_header.index_offset_buffer_offset.get();

    debug!(
        "Stream offset of table: 0x{:08x}",
        index_offset_buffer_offset
    );
    debug!("Size in bytes of table: 0x{:08x}", hash_index_buffer.len());
    debug!(
        "Start of buffer:\n{:?}",
        HexDump::new(hash_index_buffer).max(512)
    );

    // LayoutVerified verifies that the input slice is a multiple of the element size.
    let pairs: &[super::HashIndexPair] = if let Some(lv) =
        zerocopy::Ref::new_slice_unaligned(hash_index_buffer)
    {
        lv.into_slice()
    } else {
        bail!("The Hash Index Substream has an invalid size ({}). It is required to be a multiple of 8 (the record size), but it is not.",
            hash_index_buffer.len());
    };

    debug!("Number of index pairs in stream: {}", pairs.len());

    if pairs[0].type_index.get() != type_stream_header.type_index_begin.get() {
        bail!("The first entry in the Hash Index Substream is required to have its type_index equal to the type_index_begin value from the Type Stream Header.");
    }
    if pairs[0].offset.get() != 0 {
        bail!(
            "The first entry in the Hash Index Substream is required to have a stream offset of 0."
        );
    }

    // Verify that the type index values and buffer offsets are both strictly increasing.
    let mut num_misordered: u32 = 0;
    for (i, w) in pairs.windows(2).enumerate() {
        let w0_type_index = w[0].type_index.get();
        let w1_type_index = w[1].type_index.get();
        let w0_offset = w[0].offset.get();
        let w1_offset = w[1].offset.get();

        if i < 10 {
            debug!(
                "pair # {i}: {w0_type_index:?} (Δ 0x{ti_delta:04x}) at 0x{w0_offset:08x} (Δ 0x{offset_delta:08x})",
                ti_delta = w1_type_index.0.wrapping_sub(w0_type_index.0),
                offset_delta = w1_offset.wrapping_sub(w0_offset)
            );
        }

        if w0_type_index >= w1_type_index || w0_offset >= w1_offset {
            if num_misordered == 0 {
                error!("[in stream {stream}]");
            }
            if num_misordered < 20 {
                error!(
                    "[at {stream_offset:08x}] misordered pair at # {i} : \
                    T{w0_type_index:08x} at 0x{w0_offset:08x} vs. \
                    T{w1_type_index:08x} at 0x{w1_offset:08x}",
                    stream_offset = index_offset_buffer_offset as usize + i * 8,
                    w0_type_index = w0_type_index.0,
                    w1_type_index = w1_type_index.0
                );
            }

            num_misordered += 1;
        }
    }

    // Check whether the record offsets are valid.
    let mut num_invalid_offset: u32 = 0;
    for (i, pair) in pairs.iter().enumerate() {
        let offset = pair.offset.get();
        if type_record_starts.binary_search(&offset).is_err() {
            if num_invalid_offset < 20 {
                error!(
                    "[at {stream_offset:08x}] record has invalid offset (does not point to valid record).  offset: 0x{offset:08x}",
                    stream_offset = index_offset_buffer_offset as usize + i * 8
                );
            }
            num_invalid_offset += 1;
        }
    }

    if num_misordered != 0 {
        bail!("The Hash Index Offset Substream contains {num_misordered} misordered entries.");
    } else {
        debug!("All entries in Hash Index Offset Substream verified.");
    }

    Ok(())
}
