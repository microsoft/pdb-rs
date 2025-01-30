//! Name-to-Symbol Lookup Table

#[cfg(test)]
mod tests;

use super::gss::GlobalSymbolStream;
use crate::parser::{Parser, ParserMut};
use crate::syms::Sym;
use anyhow::bail;
use bitvec::prelude::{BitSlice, Lsb0};
use bstr::BStr;
use std::mem::size_of;
use tracing::{debug, debug_span, error, info, trace, warn};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, I32, LE, U32};

/// This is the size used for calculating hash indices. It was the size of the in-memory form
/// of the hash records, on 32-bit machines. It is not the same as the length of the hash records
/// that are stored on disk (which is 8).
pub const HASH_RECORD_CALC_LEN: i32 = 12;

// section contribution structure
/// section contribution version, before V70 there was no section version
pub const GSI_HASH_SC_IMPV_V70: u32 = 0xeffe0000 + 19990810;

/// Signature value for `NameTableHeader::ver_signature`.
pub const GSI_HASH_HEADER_SIGNATURE: u32 = 0xffff_ffff;
/// The current value to use (implementation version) for the GSI Hash.
pub const GSI_HASH_HEADER_VERSION: u32 = GSI_HASH_SC_IMPV_V70;

/// This header is present at the start of the Name Table.
///
/// Immediately after this header follows the hash records, whose length is given by
/// `cb_hash_records`. This section contains an array of [`HashRecord`] structs.
///
/// Immediately after the hash records is the hash buckets, whose length is given by `cb_buckets`.
///
/// Called `GSIHashHdr` in C++.
#[derive(IntoBytes, FromBytes, Unaligned, KnownLayout, Immutable, Debug)]
#[repr(C)]
pub struct NameTableHeader {
    /// Constant which identifies this as a `NameTableHeader`. This value (0xffff_ffff) was chosen
    /// because the previous version of the `NameTable` did not use this header, and this
    /// signature value would never have occurred in the same position in the previous version.
    pub signature: U32<LE>,

    /// Version of the name hash table.
    pub version: U32<LE>,

    /// Size in bytes of the hash records. This should always be a multiple of
    /// `size_of::<HashRecord>()`, not `HASH_RECORD_CALC_LEN`.
    pub hash_records_size: U32<LE>,

    /// Size in bytes of the hash buckets.
    pub buckets_size: U32<LE>,
}

/// An entry in the GSI hash table. This is in the `cb_hash_records` region.
#[derive(IntoBytes, FromBytes, Unaligned, KnownLayout, Immutable, Debug, Clone)]
#[repr(C)]
pub struct HashRecord {
    /// The byte offset of a symbol record within the Global Symbol Stream, plus 1.
    /// It should point to the beginning of a global symbol record, e.g. `S_PUB32`.
    ///
    /// The value should never be negative or zero.
    pub offset: I32<LE>,

    /// This appears to be a reference-count field, use for the in-memory representation.
    /// On disk, the values are nearly always 1.
    pub c_ref: I32<LE>,
}

/// Contains a Name Table, as used in the GSI and PSI.
pub struct NameTable {
    /// Each value in this vector is an index into `hash_records`.
    /// There is an extra index at the end, to make range calculations easier.
    /// This is a "starts" vector that points into `hash_records`.
    ///
    /// `hash_buckets` contains a list of offsets into an array of `HROffsetCalc`
    /// structures, each of which is 12 bytes long. So all of the offsets should be a multiple
    /// of 12.  (Entries can also be -1, meaning there are no entries in that bucket.)
    ///
    /// But here's where it gets weird!  The value 12 was the size of a Win32 (32-bit) structure
    /// that contained memory pointers.  It is not the size of the structure that gets written
    /// to disk.  _That_ structure is 8 bytes!  So when you read these values, you must first
    /// divide them by 12, then multiply by 8, to get the actual byte offset into the on-disk
    /// data structure.
    hash_buckets: Vec<u32>,

    /// Each record in this table corresponds to one symbol record (e.g. `S_PUB32`). It contains
    /// the offset in bytes into the global symbol stream, plus 1.  The value 0 is reserved.
    hash_records: Vec<HashRecord>,
}

impl NameTable {
    /// Parses a `NameTable`. For the GSI, the entire GSI stream contains only a `NameTable`.
    /// For the `PSI, the PSI stream begins with a `PsiStreamHeader`, followed by a `NameTable`.
    pub fn parse(
        num_buckets: usize,
        stream_offset: usize,
        sym_hash_bytes: &[u8],
    ) -> anyhow::Result<Self> {
        // Read the hash table from the stream data. The hash table may be in one of two forms:
        // "large" or "small".  The "large" format was the original format. The "small" format was
        // added later.
        //
        // The hash records are the same for the small and large hash table formats. They are
        // different in how the hash buckets are stored.

        let _span = debug_span!("NameTable::parse").entered();

        let original_len = stream_offset + sym_hash_bytes.len();

        // See gsi.cpp, GSI1::readHash()
        let hash_records: Vec<HashRecord>;
        let hash_buckets: Vec<u32>;
        {
            let mut p = Parser::new(sym_hash_bytes);
            let stream_offset_table_header = original_len - p.len();
            let hash_header: &NameTableHeader = p.get()?;

            if hash_header.signature.get() == GSI_HASH_HEADER_SIGNATURE
                && hash_header.version.get() == GSI_HASH_HEADER_VERSION
            {
                debug!(
                    hash_records_size = hash_header.hash_records_size.get(),
                    buckets_size = hash_header.buckets_size.get(),
                    "Hash table format: small buckets"
                );

                let hash_records_size = hash_header.hash_records_size.get() as usize;
                let buckets_size = hash_header.buckets_size.get() as usize;

                let stream_offset_hash_records = original_len - p.len();
                let hash_records_bytes = p.bytes(hash_records_size)?;
                let stream_offset_hash_buckets = original_len - p.len();
                let buckets_bytes = p.bytes(buckets_size)?;
                let stream_offset_end = original_len - p.len();

                // We expect that the size of the hash table is equal to the size of the header +
                // cb_hash_records + cb_buckets.
                if !p.is_empty() {
                    warn!("Found extra bytes in hash table, len = {}", p.len());
                }

                if hash_records_size % size_of::<HashRecord>() != 0 {
                    warn!("GSI/PSI name table contains hash table with a length that is not a multiple of the hash record size.");
                }
                let num_hash_records = hash_records_size / size_of::<HashRecord>();
                let hash_records_slice: &[HashRecord] =
                    Parser::new(hash_records_bytes).slice(num_hash_records)?;

                debug!(num_hash_records, "Number of records in name table");

                hash_records = hash_records_slice.to_vec();

                debug!(num_buckets, "Number of hash buckets in name table");

                debug!("[........] Stream offsets:");
                debug!("[{:08x}] : NameTableHeader", stream_offset_table_header);
                debug!("[{:08x}] : hash records", stream_offset_hash_records);
                debug!("[{:08x}] : hash buckets", stream_offset_hash_buckets);
                debug!("[{:08x}] : (end)", stream_offset_end);

                hash_buckets = expand_buckets(
                    buckets_bytes,
                    num_buckets,
                    num_hash_records,
                    stream_offset_hash_buckets,
                )?;
            } else {
                // We did not find a GsiHashHeader, so this is an old-style hash.
                error!("Hash table format: old-style normal buckets");
                bail!("Old-style hash table is not supported");
            }
        }

        Ok(Self {
            hash_buckets,
            hash_records,
        })
    }

    /// Constructs an empty instance
    pub fn empty() -> Self {
        Self {
            hash_buckets: vec![0],
            hash_records: vec![],
        }
    }

    /// Check all of the hash records in the name table and verify that the hash of the name for
    /// this record matches the hash bucket that the hash record is in.
    pub fn check_hashes(&self, global_symbols: &GlobalSymbolStream) -> anyhow::Result<()> {
        // Verify that hash buckets point to symbol records, and that the hashes of the symbol
        // names in those symbol records matches the hash code for this bucket.

        let mut num_hashes_incorrect: u32 = 0;

        for (bucket_index, hash_index_window) in self.hash_buckets.windows(2).enumerate() {
            trace!(
                "Checking hash bucket #{bucket_index}, hash records at {} .. {}",
                hash_index_window[0],
                hash_index_window[1]
            );
            let Some(bucket_hash_records) = self
                .hash_records
                .get(hash_index_window[0] as usize..hash_index_window[1] as usize)
            else {
                error!("hash record range is invalid");
                continue;
            };

            for hash_record in bucket_hash_records.iter() {
                let hash_record_offset = hash_record.offset.get();

                // The hash record offset should always be positive.
                if hash_record_offset <= 0 {
                    continue;
                }

                let record_offset_in_symbol_stream = hash_record_offset - 1;
                let pub_sym =
                    match global_symbols.get_pub32_at(record_offset_in_symbol_stream as u32) {
                        Err(e) => {
                            error!("{e}");
                            continue;
                        }
                        Ok(s) => s,
                    };

                let name_hash =
                    crate::hash::hash_mod_u32(pub_sym.name, self.hash_buckets.len() as u32 - 1);

                if name_hash != bucket_index as u32 {
                    if num_hashes_incorrect < 50 {
                        error!(
                            "bucket #{} has symbol {} with wrong hash code {}",
                            bucket_index, pub_sym.name, name_hash
                        );
                    }
                    num_hashes_incorrect += 1;
                }
            }
        }

        if num_hashes_incorrect != 0 {
            error!("Found {num_hashes_incorrect} hash records with the wrong hash value");
        } else {
            info!("All name hashes are correct.");
        }

        Ok(())
    }

    /// Gets the hash records for a specific bucket index. The caller is responsible for using
    /// a valid bucket index.
    pub fn hash_records_for_bucket(&self, bucket: usize) -> &[HashRecord] {
        let start = self.hash_buckets[bucket] as usize;
        let end = self.hash_buckets[bucket + 1] as usize;
        &self.hash_records[start..end]
    }

    /// Gets the hash records that might contain `name`.
    pub fn hash_records_for_name<'a>(&'a self, name: &BStr) -> &'a [HashRecord] {
        // If hash_buckets is empty (i.e. has a length of 1, because it is a "starts" table),
        // then the table is empty and the hash modulus is zero. We need to avoid dividing by zero.
        if self.hash_buckets.len() <= 1 {
            return &[];
        }

        let name_hash = crate::hash::hash_mod_u32(name, self.hash_buckets.len() as u32 - 1);
        self.hash_records_for_bucket(name_hash as usize)
    }

    /// Searches for `name` in the Name Table. The caller must provide access to the GSS.
    pub fn find_symbol<'a>(
        &self,
        gss: &'a GlobalSymbolStream,
        name: &BStr,
    ) -> anyhow::Result<Option<Sym<'a>>> {
        let bucket_entries = self.hash_records_for_name(name);
        for entry in bucket_entries.iter() {
            let entry_offset = entry.offset.get();
            if entry_offset <= 0 {
                warn!("found invalid hash record; entry.offset <= 0");
                continue;
            }

            let sym = gss.get_sym_at(entry_offset as u32 - 1)?;

            if let Some(sym_name) = super::get_global_symbol_name(sym.kind, sym.data)? {
                if sym_name.eq_ignore_ascii_case(name) {
                    return Ok(Some(sym));
                }
            }
        }

        Ok(None)
    }

    /// Iterates the names stored within a `NameTable`.
    pub fn iter<'i, 'a: 'i>(&'a self, gss: &'a GlobalSymbolStream) -> NameTableIter<'i> {
        NameTableIter {
            hash_record_iter: self.hash_records.iter(),
            gss,
        }
    }
}

/// Iterator state for iterating the names within a `NameTable`.
pub struct NameTableIter<'a> {
    hash_record_iter: std::slice::Iter<'a, HashRecord>,
    gss: &'a GlobalSymbolStream,
}

impl<'a> Iterator for NameTableIter<'a> {
    type Item = Sym<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let hash_record = self.hash_record_iter.next()?;

            let sym_offset = hash_record.offset.get();
            if sym_offset <= 0 {
                // Whatever, just ignore it.
                continue;
            }

            if let Ok(sym) = self.gss.get_sym_at((sym_offset - 1) as u32) {
                return Some(sym);
            } else {
                error!("failed to decode symbol in GSS at offset {sym_offset}");
                return None;
            }
        }
    }
}

/// Gets the default number of buckets to use.
///
/// The values are hard-coded. 0x1000 is used for normal PDBs and 0x3ffff is used for "mini PDBs".
/// Mini PDBs are produced using the `/DEBUG:FASTLINK` linker option.
pub fn get_v1_default_bucket(minimal_dbg_info: bool) -> usize {
    if minimal_dbg_info {
        0x3ffff
    } else {
        0x1000
    }
}

/// Expands a compressed bucket. Returns a vector of offsets.
///
/// The input contains a bitmap, followed by an array of offsets. The bitmap determines how many
/// items there are in the array of offsets. The length of the bitmap is specified by num_buckets.
///
/// This function returns a vector that contains hash indices. The hash records for a given hash
/// bucket can be found as:
///
/// ```text
/// let buckets = expand_buckets(...)?;
/// let bucket_index = 10;
/// let hash_records: &[HashRecord] = &hash_records[buckets[bucket_index]..buckets[bucket_index + 1]];
/// ```
fn expand_buckets(
    buckets_bytes: &[u8],
    num_buckets: usize,
    num_hash_records: usize,
    stream_offset: usize, // offset of this data structure in stream; for diagnostics only
) -> anyhow::Result<Vec<u32>> {
    let _span = debug_span!("expand_buckets").entered();

    trace!(num_buckets, num_hash_records, "expanding buckets");

    let original_len = stream_offset + buckets_bytes.len();
    let mut p = Parser::new(buckets_bytes);

    let output_len = num_buckets + 1;

    let bitmap_len_in_bytes = nonempty_bitmap_size_bytes(num_buckets);
    let bitmap_bytes = p.bytes(bitmap_len_in_bytes)?;
    let bv: &BitSlice<u8, Lsb0> = BitSlice::from_slice(bitmap_bytes);
    trace!(bitmap_bytes, bitmap_len_in_bytes);

    // Count the number of 1 bits set in the non-empty bucket bitmap.
    // Use min(num_buckets) so that we ignore any extra bits in the bitmap.
    let num_nonempty_buckets = bv.count_ones().min(num_buckets);
    trace!(num_nonempty_buckets);

    let nonempty_pointers_stream_offset = original_len - p.len();
    let nonempty_pointers: &[I32<LE>] = p.slice(num_nonempty_buckets)?;

    trace!(
        nonempty_pointers_stream_offset,
        non_empty_pointers = nonempty_pointers.as_bytes(),
        "non-empty pointers"
    );

    let mut nonempty_pointers_iter = nonempty_pointers.iter();

    let mut hash_buckets = Vec::with_capacity(output_len);
    for bucket_index in bv.iter_ones() {
        // Be careful to avoid processing 1 bits in the bitmap. It is possible for the bitmap
        // to contain more bits than there are buckets, due to bit alignment.
        if bucket_index >= num_buckets {
            break;
        }

        // The unwrap() cannot fail because we computed the slice length (num_nonempty_buckets)
        // from the number of 1 bits in the non-empty mask (bv).
        let offset_x12 = nonempty_pointers_iter.next().unwrap().get();
        if offset_x12 < 0 {
            bail!("found a negative offset in hash buckets");
        }
        if offset_x12 % HASH_RECORD_CALC_LEN != 0 {
            bail!("hash record offset {offset_x12} is not a multiple of 12 (as required)");
        }
        let offset = (offset_x12 / HASH_RECORD_CALC_LEN) as u32;

        // It would be strange for offset to be equal to num_hash_records because that would
        // imply an empty "non-empty" hash bucket.
        if offset as usize >= num_hash_records {
            bail!("hash record offset {offset_x12} is beyond range of hash records table");
        }

        // Record offsets must be non-decreasing. They should actually be strictly increasing,
        // but we tolerate repeated values.
        if let Some(&prev_offset) = hash_buckets.last() {
            if offset < prev_offset {
                bail!("hash record offset {offset} is less than previous offset {prev_offset}");
            }
        } else if offset != 0 {
            bail!("First hash record offset should be zero, but instead it is: 0x{offset:x}");
        }

        // Add offsets for previous buckets, which were all empty.
        if hash_buckets.len() < bucket_index {
            trace!("    bucket: 0x{:08x} .. 0x{bucket_index:08x} : range is empty, pushing offset: 0x{offset:8x} {offset:10}", hash_buckets.len());
            hash_buckets.resize(bucket_index, offset);
        }
        trace!(
            "    bucket: 0x{b:08x} --> offset: 0x{offset:08x}",
            b = hash_buckets.len()
        );
        hash_buckets.push(offset);
        assert!(hash_buckets.len() <= num_buckets);
    }

    // Fill in the offsets for the remaining empty buckets (if any), and push an extra offset for
    // the end of the hash records array.
    assert!(hash_buckets.len() <= num_buckets);
    hash_buckets.resize(num_buckets + 1, num_hash_records as u32);

    trace!(
        "hash bucket offsets: {:?}",
        &hash_buckets[..hash_buckets.len().min(100)]
    );

    if !nonempty_pointers_iter.as_slice().is_empty() {
        warn!(
            num_extra_bytes = p.len(),
            rest = p.peek_rest(),
            "Compressed hash buckets table contains extra byte(s)"
        );
    }

    if tracing::event_enabled!(tracing::Level::TRACE) {
        trace!("Non-empty buckets: (within expand_buckets)");
        for i in 0..hash_buckets.len() - 1 {
            let start = hash_buckets[i];
            let end = hash_buckets[i + 1];
            if start != end {
                trace!(i, start, end);
            }
        }
    }

    Ok(hash_buckets)
}

/// Used when rebuilding a Name Table
///
/// The order of the fields is significant because it is used for sorting.
#[derive(Eq, PartialEq, PartialOrd, Ord)]
pub struct HashEntry {
    /// computed hash code
    pub hash: u32,
    /// Symbol offset within the GSS. This value has a bias of +1. It will never be 0.
    pub symbol_offset: i32,
}

/// Scan hash_records and figure out how many hash buckets are _not_ empty. Because hash_records
/// is sorted by hash, we can do a single scan through it and find all of the "edges" (places where
/// the `hash` value changes).
pub fn count_nonempty_buckets(sorted_hash_records: &[HashEntry]) -> usize {
    iter_nonempty_buckets(sorted_hash_records).count()
}

/// Compute the size in bytes of the bitmap of non-empty buckets.
pub fn nonempty_bitmap_size_bytes(num_buckets: usize) -> usize {
    let compressed_bitvec_size_u32s = (num_buckets + 1 + 31) / 32;
    compressed_bitvec_size_u32s * 4
}

/// Compute the size in bytes of the name hash table. This includes the header.
pub fn compute_hash_table_size_bytes(
    num_hash_records: usize,
    num_buckets: usize,
    num_nonempty_buckets: usize,
) -> usize {
    size_of::<NameTableHeader>()
        + num_hash_records * size_of::<HashRecord>()
        + nonempty_bitmap_size_bytes(num_buckets)
        + num_nonempty_buckets * size_of::<i32>()
}

/// Output of `build_name_table_prepare`
pub struct NameTableInfo {
    /// The number of buckets to use. This is an input parameter for the table building code, but
    /// it is preserved here to simplify control flow.
    pub num_buckets: usize,
    /// Number of non-empty buckets.  Always less than or equal to `num_buckets.
    pub num_nonempty_buckets: usize,
    /// Size of the entire table, in bytes.
    pub table_size_bytes: usize,
}

impl NameTableBuilder {
    /// Reads a set of sorted hash records and computes the number of non-empty hash buckets and the
    /// total size of the Name Table in bytes.
    pub fn prepare(&mut self) -> NameTableInfo {
        self.sort();

        let num_nonempty_buckets = count_nonempty_buckets(&self.hash_records);
        let table_size_bytes = compute_hash_table_size_bytes(
            self.hash_records.len(),
            self.num_buckets,
            num_nonempty_buckets,
        );
        NameTableInfo {
            num_buckets: self.num_buckets,
            num_nonempty_buckets,
            table_size_bytes,
        }
    }

    /// The number of names inserted into this builder.
    pub fn num_names(&self) -> usize {
        self.hash_records.len()
    }

    /// Writes the hash records, the hash header, and the buckets to output. The output size must be
    /// equal to the expected size.
    pub fn encode(&self, prepared_info: &NameTableInfo, output: &mut [u8]) {
        debug!(
            "Number of symbols found (in name table): {n} 0x{n:x}",
            n = self.hash_records.len()
        );
        debug!(
            "Size in bytes of hash records (in name table): {s} 0x{s:x}",
            s = self.hash_records.len() * 8,
        );

        let compressed_bitvec_size_bytes = nonempty_bitmap_size_bytes(self.num_buckets);
        let num_nonempty_buckets = prepared_info.num_nonempty_buckets;
        let hash_buckets_size_bytes =
            compressed_bitvec_size_bytes + num_nonempty_buckets * size_of::<i32>();
        debug!(
            "hash_buckets_size_bytes = 0x{0:x} {0}",
            hash_buckets_size_bytes
        );

        // Build the name-to-symbol hash map.
        // The structure of the name-to-symbol hash map is:
        //
        //      GsiHashHeader (fixed size)
        //      [HashRecord; num_hash_records]
        //      u32-aligned bitmap of num_buckets + 1 length (in bits), indicating whether a given bucket is non-empty.
        //      [i32; num_non_empty_buckets]
        {
            let mut hash_output_cursor = ParserMut::new(output);

            // Write the hash header.
            *hash_output_cursor.get_mut().unwrap() = NameTableHeader {
                signature: U32::new(GSI_HASH_HEADER_SIGNATURE),
                version: U32::new(GSI_HASH_HEADER_VERSION),
                buckets_size: U32::new(hash_buckets_size_bytes as u32),
                hash_records_size: U32::new(
                    (self.hash_records.len() * size_of::<HashRecord>()) as u32,
                ),
            };

            // Write the hash records.
            {
                let output_slice: &mut [HashRecord] = hash_output_cursor
                    .slice_mut(self.hash_records.len())
                    .unwrap();
                for (from, to) in self.hash_records.iter().zip(output_slice.iter_mut()) {
                    *to = HashRecord {
                        offset: I32::new(from.symbol_offset),
                        c_ref: I32::new(1),
                    };
                }
            }

            // Set all bits in the presence bitmap.
            {
                let sym_hash_bitvec_bytes = hash_output_cursor
                    .bytes_mut(compressed_bitvec_size_bytes)
                    .unwrap();
                let bv: &mut BitSlice<u8, Lsb0> = BitSlice::from_slice_mut(sym_hash_bitvec_bytes);

                for (_record_index, bucket_hashes) in iter_nonempty_buckets(&self.hash_records) {
                    let hash = bucket_hashes[0].hash;
                    bv.set(hash as usize, true);
                }

                assert_eq!(bv.count_ones(), num_nonempty_buckets);
            }

            // Write the hash record offsets of each of the non-empty hash buckets.
            // These values are non-decreasing. The end of each hash bucket is the beginning of the
            // next hash bucket.
            {
                let mut num_nonempty_written = 0;
                let output_offsets_slice: &mut [U32<LE>] =
                    hash_output_cursor.slice_mut(num_nonempty_buckets).unwrap();
                let mut output_iter = output_offsets_slice.iter_mut();

                for (record_index, _hashes) in iter_nonempty_buckets(&self.hash_records) {
                    *output_iter.next().unwrap() = U32::new((record_index as u32) * 12);
                    num_nonempty_written += 1;
                }

                assert_eq!(num_nonempty_written, num_nonempty_buckets);
                assert!(output_iter.as_slice().is_empty());
            }

            assert!(hash_output_cursor.is_empty());
        }
    }
}

/// Sorts hash records
#[inline(never)]
pub fn sort_hash_records(hash_records: &mut [HashEntry]) {
    // This fully determines the order, since we are comparing all fields.
    hash_records.sort_unstable();
}

/// Iterates non-empty buckets. Each item is `(record_index, bucket)`, where `record_index` is
/// the index within `hashes` (the input of this function) where `bucket` begins.  The iterated
/// `bucket` value will always contain values (will not be empty).
fn iter_nonempty_buckets(hashes: &[HashEntry]) -> IterNonEmptyBuckets<'_> {
    IterNonEmptyBuckets {
        record_index: 0,
        hashes,
    }
}

struct IterNonEmptyBuckets<'a> {
    record_index: usize,
    hashes: &'a [HashEntry],
}

impl<'a> Iterator for IterNonEmptyBuckets<'a> {
    // (index into hash records, slice of hash records in bucket)
    type Item = (usize, &'a [HashEntry]);

    fn next(&mut self) -> Option<Self::Item> {
        if self.hashes.is_empty() {
            return None;
        }

        let record_index = self.record_index;

        let hash = self.hashes[0].hash;
        let mut end = 1;
        while end < self.hashes.len() && self.hashes[end].hash == hash {
            end += 1;
        }

        let (lo, hi) = self.hashes.split_at(end);
        self.record_index += end;
        self.hashes = hi;
        Some((record_index, lo))
    }
}

/// This type constructs a new name table, for the GSI/PSI.
///
/// Example:
///
/// ```
/// # use mspdb::globals::name_table::NameTableBuilder;
/// # use bstr::BStr;
/// let mut builder = NameTableBuilder::new(0x1000);
/// builder.push("hello".into(), 1);
/// builder.push("world".into(), 2);
/// let prepared_info = builder.prepare();
/// let mut encoded_bytes: Vec<u8> = vec![0; prepared_info.table_size_bytes];
/// builder.encode(&prepared_info, &mut encoded_bytes);
/// ```
pub struct NameTableBuilder {
    num_buckets: usize,
    hash_records: Vec<HashEntry>,
}

impl NameTableBuilder {
    /// Starts building a new name table.  Specify the number of hash buckets to use.
    pub fn new(num_buckets: usize) -> Self {
        assert!(num_buckets > 0);
        Self {
            num_buckets,
            hash_records: Vec::new(),
        }
    }

    /// The number of hash buckets.
    pub fn num_buckets(&self) -> usize {
        self.num_buckets
    }

    /// Adds a new string entry to the builder. Specify the hash of the string and its symbol
    /// offset. The hash _must_ already have been computed and the remainder taken using
    /// `num_buckets()`.
    pub fn push_hash(&mut self, hash: u32, symbol_offset: i32) {
        assert!((hash as usize) < self.num_buckets);
        self.hash_records.push(HashEntry {
            hash,
            symbol_offset,
        });
    }

    /// Hashes a string and adds it to the table. The contents of the string are not retained.
    pub fn push(&mut self, name: &BStr, symbol_offset: i32) {
        let name_hash = crate::hash::hash_mod_u32(name, self.num_buckets as u32);
        self.push_hash(name_hash, symbol_offset);
    }

    fn sort(&mut self) {
        sort_hash_records(&mut self.hash_records);
    }
}
