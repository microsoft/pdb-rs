//! Parses the Names Stream (`/names`).
//!
//! The Names Stream stores a set of unique strings (names). This allows other data structures to
//! refer to strings using an integer index ([`NameIndex`]), rather than storing copies of the same
//! string in many different places.
//!
//! The stream index for the Names Stream is found in the PDB Information Stream, in the Named
//! Streams section.  The key is "/names".
//!
//! The Names Stream begins with `NamesStreamHeader`, which specifies the size in bytes of the
//! string data substream. The string data substream immediately follows the stream header.
//! It consists of NUL-terminated UTF-8 strings.
//!
//! After the string data there is a hash table. The hash table is an array, one for each string
//! in the table. The value of each array entry is a byte offset that points into the string data.
//! The index of each array entry is chosen using a hash of the corresponding string value.
//!
//! Hash collisions are resolved using linear probing. That is, during table construction, the
//! hash table is allocated and initialized, with each entry pointing to nothing (nil). For each
//! string, we compute the hash of the string (modulo the size of the hash table). If the
//! corresponding entry in the hash table is empty, then we write the `NameIndex` value into that
//! slot. If that slot is already busy, then we check the next slot; if we reach the end of the
//! table then we wrap around to slot 0. For this reason, the number of hash entries must be
//! greater than or equal to the number of strings in the table.
//!
//! The overall organization of the stream is:
//!
//! name             | type                 | usage
//! -----------------|----------------------|------
//! `signature`      | `u32`                | should always be 0xEFFE_EFFE
//! `version`        | `u32`                | should be 1
//! `strings_size`   | `u32`                | size of the string data
//! `strings_data`   | `[u8; strings_size]` | contains the UTF-8 string data, with NUL terminators
//! `num_hashes`     | `u32`                | specifies the number of hash entries
//! `hashes`         | `[u32; num_hashes]`  | contains hash entries for all strings
//! `num_strings`    | `u32`                | number of non-empty strings in the table

#[cfg(test)]
mod tests;

use crate::ReadAt;
use crate::utils::align_4;
use anyhow::bail;
use bstr::BStr;
use ms_codeview::parser::{Parser, ParserMut};
use ms_codeview::{HasRestLen, IteratorWithRangesExt};
use std::ops::Range;
use tracing::{debug, trace, trace_span, warn};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, LE, U32, Unaligned};

/// The name of the `/names` stream. This identifies the stream in the Named Streams Table,
/// in the PDB Information Stream.
pub const NAMES_STREAM_NAME: &str = "/names";

/// A byte offset into the Names Stream.
///
/// This value does not include the size of the stream header, so the size of the stream header
/// must be added to it when dereferencing a string.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct NameIndex(pub u32);

impl std::fmt::Display for NameIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}

#[test]
fn name_index_display() {
    assert_eq!(format!("{}", NameIndex(42)), "42");
}

/// Represents a `NameIndex` value in LE byte order.
#[derive(
    Copy, Clone, Eq, PartialEq, Debug, IntoBytes, Immutable, KnownLayout, FromBytes, Unaligned,
)]
#[repr(transparent)]
pub struct NameIndexLe(pub U32<LE>);

impl NameIndexLe {
    /// Converts the value to the in-memory byte order.
    #[inline(always)]
    pub fn get(self) -> NameIndex {
        NameIndex(self.0.get())
    }
}

/// Value for `NamesStreamHeader::signature`.
pub const NAMES_STREAM_SIGNATURE: u32 = 0xEFFE_EFFE;

/// Value for `NamesStreamHeader::version`.
pub const NAMES_STREAM_VERSION_V1: u32 = 1;

/// The header of the Names Stream.
#[repr(C)]
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned)]
pub struct NamesStreamHeader {
    /// Signature identifies this as a Names Stream. Should always be `NAMES_STREAM_SIGNATURE`.
    pub signature: U32<LE>,
    /// Version of the Names Stream, which determines the hash function.
    pub version: U32<LE>,
    /// Size in bytes of the string data, which immediately follows this header.
    pub strings_size: U32<LE>,
}

/// Stream data for an empty Names stream.
pub static EMPTY_NAMES_STREAM_DATA: &[u8] = &[
    0xFE, 0xEF, 0xFE, 0xEF, // signature
    0x01, 0x00, 0x00, 0x00, // version
    0x04, 0x00, 0x00, 0x00, // strings_size
    0x00, 0x00, 0x00, 0x00, // string data
    0x01, 0x00, 0x00, 0x00, // num_hashes
    0x00, 0x00, 0x00, 0x00, // hash[0]
    0x00, 0x00, 0x00, 0x00, // num_strings
];

#[test]
fn parse_empty_names_stream() {
    let names = NamesStream::parse(EMPTY_NAMES_STREAM_DATA).unwrap();
    assert_eq!(names.num_strings, 0);
    assert_eq!(names.num_hashes, 1);
}

/// The size of the Names Stream Header, in bytes.
pub const NAMES_STREAM_HEADER_LEN: usize = 12;

/// Reads the `/names` stream.
pub struct NamesStream<StreamData>
where
    StreamData: AsRef<[u8]>,
{
    /// Contains the stream data of the `/names` stream.
    pub stream_data: StreamData,

    /// The size of the string data. This value comes from the stream header.
    pub strings_size: usize,

    /// The number of entries in the hash table.
    pub num_hashes: usize,

    /// The byte offset within `stream_data` where the hash records begin. Each hash record
    /// contains a `NameIndex` value. The number of elements is `num_hashes`.
    pub hashes_offset: usize,

    /// The is the number of strings from the stream trailer. Nothing guarantees that this value
    /// correctly reflects the number of strings in the string data.
    pub num_strings: usize,
}

impl<StreamData> NamesStream<StreamData>
where
    StreamData: AsRef<[u8]>,
{
    /// Parses and validates the stream header.
    ///
    /// This function does not validate all of the strings in the table.
    /// The `check()` function performs extensive checks.
    pub fn parse(stream_data: StreamData) -> anyhow::Result<Self> {
        let stream_data_slice: &[u8] = stream_data.as_ref();
        let mut p = Parser::new(stream_data_slice);
        let header: &NamesStreamHeader = p.get()?;

        if header.signature.get() != NAMES_STREAM_SIGNATURE {
            bail!(
                "The `/names` stream has an invalid signature: 0x{:08x}.",
                header.signature.get()
            );
        }

        if header.version.get() != NAMES_STREAM_VERSION_V1 {
            bail!(
                "The `/names` stream is using an unsupported version: {}.",
                header.version.get()
            );
        }

        let strings_size = header.strings_size.get() as usize;
        let _string_data = p.bytes(strings_size)?;

        // Read the header of the hash table. The only value in the fixed-size portion is a u32
        // that specifies the number of hashes in the table.
        let num_hashes = p.u32()? as usize;

        let hashes_offset = stream_data_slice.len() - p.len();
        let _hashed_names: &[U32<LE>] = p.slice(num_hashes)?;

        // The last item is a u32 that specifies the number of strings in the table.
        let num_strings = p.u32()? as usize;

        Ok(Self {
            stream_data,
            strings_size,
            num_hashes,
            hashes_offset,
            num_strings,
        })
    }

    /// Returns the byte range within the stream of the string data.
    pub fn strings_range(&self) -> Range<usize> {
        NAMES_STREAM_HEADER_LEN..NAMES_STREAM_HEADER_LEN + self.strings_size
    }

    /// Gets the strings data
    pub fn strings_bytes(&self) -> &[u8] {
        &self.stream_data.as_ref()[self.strings_range()]
    }

    /// Gets the hash table. Each entry contains NameIndex, or 0.  The entries are arranged in
    /// the order of the hash of the strings.
    pub fn hashes(&self) -> &[U32<LE>] {
        let stream_data = self.stream_data.as_ref();
        <[U32<LE>]>::ref_from_prefix_with_elems(&stream_data[self.hashes_offset..], self.num_hashes)
            .unwrap()
            .0
    }

    /// Retrieves one string from the string table.
    pub fn get_string(&self, offset: NameIndex) -> anyhow::Result<&BStr> {
        let strings_bytes = self.strings_bytes();
        if let Some(s_bytes) = strings_bytes.get(offset.0 as usize..) {
            let mut p = Parser::new(s_bytes);
            let s = p.strz()?;
            trace!("found string at {offset:?} : {s:?}");
            Ok(s)
        } else {
            bail!("String offset {offset:?} is invalid (out of range)");
        }
    }

    /// Iterates the strings in the table, by reading the character data directly.
    ///
    /// By convention, the string table usually begins with the empty string. However, this is not
    /// a guarantee of this implementation.
    ///
    /// This iterator may iterate empty strings at the end of the sequence, due to alignment bytes
    /// at the end of the string data.
    pub fn iter(&self) -> IterNames<'_> {
        IterNames {
            rest: self.strings_bytes(),
        }
    }

    /// Sorts the Names Stream and removes duplicates. This also eliminates duplicate strings.
    ///
    /// Returns `(remapping_table, new_stream_data)`. The `remapping_table` contains tuples of
    /// `(old_offset, new_offset)` and is sorted by `old_offset`. The caller can use a binary
    /// search to remap entries.
    pub fn rebuild(&self) -> (NameIndexMapping, Vec<u8>) {
        let _span = trace_span!("NamesStream::rebuild").entered();

        let old_stream_data: &[u8] = self.stream_data.as_ref();
        // We verified the length of the stream in NamesStream::parse().
        let old_string_data = self.strings_bytes();

        // Check for the degenerate case of an empty names table, which does not even contain
        // the empty string. This should never happen, but protect against it anyway. Return
        // a copy of the current table, such as it is. The remapping_table is empty.
        if old_string_data.is_empty() {
            return (
                NameIndexMapping { table: Vec::new() },
                old_stream_data.to_vec(),
            );
        }

        // First pass, count the non-empty strings.
        let num_strings = self.iter().filter(|s| !s.is_empty()).count();
        debug!("Number of strings found: {num_strings}");

        // Second pass, build a string table.
        let mut strings: Vec<(Range<usize>, &BStr)> = Vec::with_capacity(num_strings);
        strings.extend(self.iter().with_ranges().filter(|(_, s)| !s.is_empty()));

        // Sort the strings.
        strings.sort_unstable_by_key(|i| i.1);
        strings.dedup_by_key(|i| i.1);

        let num_unique_strings = strings.len();
        if num_unique_strings != num_strings {
            debug!(
                "Removed {} duplicate strings.",
                num_strings - num_unique_strings
            );
        } else {
            debug!("Did not find duplicate strings.");
        }

        // Find the size of the new stream.
        // The 1+ at the start is for the empty string.
        let new_strings_len_unaligned = 1 + strings.iter().map(|(_, s)| s.len() + 1).sum::<usize>();
        let new_strings_len = align_4(new_strings_len_unaligned);

        // Choose the number of hashes.
        let num_hashes = num_unique_strings * 6 / 4;
        assert!(num_hashes >= num_unique_strings);
        debug!(
            "Using {} hashes for {} strings with linear probing.",
            num_hashes, num_unique_strings
        );

        let new_hash_size_bytes = 4   // for the num_hashes field
            + num_hashes * 4 // for the hashes array
            + 4; // for the num_strings field

        let new_stream_data_len = NAMES_STREAM_HEADER_LEN + new_strings_len + new_hash_size_bytes;
        debug!(
            "Old name stream size (strings only): {}",
            old_string_data.len()
        );
        debug!("New name stream size (strings only): {}", new_strings_len);

        let mut new_stream_data: Vec<u8> = vec![0; new_stream_data_len];
        let mut p = ParserMut::new(&mut new_stream_data);
        *p.get_mut().unwrap() = NamesStreamHeader {
            signature: U32::new(NAMES_STREAM_SIGNATURE),
            version: U32::new(NAMES_STREAM_VERSION_V1),
            strings_size: U32::new(new_strings_len as u32),
        };

        // Write the string data into the output table, and build the remapping table as we go.
        let mut remapping_table: Vec<(NameIndex, NameIndex)> = Vec::with_capacity(num_strings + 1);
        // Add mapping for empty
        remapping_table.push((NameIndex(0), NameIndex(0)));
        {
            let new_strings_data_with_alignment = p.bytes_mut(new_strings_len).unwrap();
            let out_bytes = &mut new_strings_data_with_alignment[..new_strings_len_unaligned];
            let out_bytes_len = out_bytes.len();
            let mut out_iter = out_bytes;

            // Write empty string.
            out_iter[0] = 0;
            out_iter = &mut out_iter[1..];

            for (old_range, s) in strings.iter() {
                let old_ni = NameIndex(old_range.start as u32);
                let new_ni = NameIndex((out_bytes_len - out_iter.len()) as u32);
                remapping_table.push((old_ni, new_ni));
                let sb: &[u8] = s;

                trace!(
                    "string: old_ni: 0x{old_ni:08x}, new_ni: 0x{new_ni:08x}, old_range: {:08x}..{:08x} s: {:?}",
                    old_range.start,
                    old_range.end,
                    s,
                    old_ni = old_ni.0,
                    new_ni = new_ni.0,
                );

                out_iter[..sb.len()].copy_from_slice(sb);
                out_iter = &mut out_iter[sb.len() + 1..]; // +1 for NUL
            }

            assert!(out_iter.is_empty());
            remapping_table.sort_unstable_by_key(|&(old, _)| old);
        }

        // Build the hash table. We rely on the table contain all zeroes before we begin writing.
        // We iterate through the strings, in the sorted order, and compute their hashes. Then we
        // insert the NameIndex into the table, using linear probing. If we get to the end, we
        // wrap around.
        let stream_offset_num_hashes = new_stream_data_len - p.len();
        *p.get_mut::<U32<LE>>().unwrap() = U32::new(num_hashes as u32);
        let stream_offset_hash_table = new_stream_data_len - p.len();

        {
            debug!("Building hash table, num_hashes = {}", num_hashes);
            let hash_table: &mut [U32<LE>] = p.slice_mut(num_hashes).unwrap();
            let mut new_ni: u32 = 1; // 1 is for empty string length
            for &(_, sb) in strings.iter() {
                let h = crate::hash::hash_mod_u32(sb, num_hashes as u32);
                trace!("ni {:08x}, hash {:08x}, {:?}", new_ni, h, sb);

                let mut hi = h;
                let mut wrapped = false;
                loop {
                    let slot = &mut hash_table[hi as usize];
                    if slot.get() == 0 {
                        *slot = U32::new(new_ni);
                        break;
                    }
                    hi += 1;
                    if hi as usize == hash_table.len() {
                        hi = 0;
                        assert!(!wrapped, "should not wrap around the table more than once");
                        wrapped = true;
                    }
                }

                new_ni += (sb.len() + 1) as u32;
            }
        }

        let stream_offset_num_strings = new_stream_data_len - p.len();
        *p.get_mut::<U32<LE>>().unwrap() = U32::new(strings.len() as u32);

        assert!(p.is_empty());

        debug!("Stream offsets:");
        debug!(
            "    [{:08x}] - Names Stream header",
            NAMES_STREAM_HEADER_LEN
        );
        debug!("    [{:08x}] - string data", NAMES_STREAM_HEADER_LEN);
        debug!(
            "    [{:08x}] - hash table header (num_hashes)",
            stream_offset_num_hashes
        );
        debug!(
            "    [{:08x}] - hash table, size in bytes = {}",
            stream_offset_hash_table,
            num_hashes * 4
        );
        debug!(
            "    [{:08x}] - num_strings field",
            stream_offset_num_strings
        );
        debug!("    [{:08x}] - (end)", new_stream_data_len);

        (
            NameIndexMapping {
                table: remapping_table,
            },
            new_stream_data,
        )
    }
}

/// Contains a mapping from old `NameIndex` to new `NameIndex. The mapping is sparse.
#[derive(Default)]
pub struct NameIndexMapping {
    /// the mapping table; use binary search for it
    ///
    /// This always starts with `(0, 0)`.
    pub table: Vec<(NameIndex, NameIndex)>,
}

impl NameIndexMapping {
    /// Looks up `name` in the mapping table and returns the mapping for it.
    pub fn map_old_to_new(&self, name: NameIndex) -> anyhow::Result<NameIndex> {
        // Perf optimization: Avoid the binary search for 0, which is never remapped.
        if name.0 == 0 {
            return Ok(name);
        }

        let table = self.table.as_slice();
        match table.binary_search_by_key(&name, |(old, _)| *old) {
            Ok(i) => Ok(table[i].1),
            Err(_) => bail!(
                "The NameIndex value 0x{:x} cannot be remapped because it was not present in the old Names stream.",
                name.0
            ),
        }
    }
}

/// Given an index `i` into a hash table `hashes`, where `hashes[i]` is already known to be used
/// (non-empty), find the range or ranges of contiguous non-empty entries in `hashes` that cover `i`.
///
/// The reason this function can return two ranges is that linear probing wraps around at the end
/// of the hash table. We have to account for wrap-around at both the start and end of `hashes`.
/// The unit tests (below) illustrate this.
///
/// We use the ranges returned from this function to verify that a given hash entry is at a legal
/// index within the hash table. The hash table may place hash entries adjacent to each other either
/// because the hash functions were numerically 1 different from each other (e.g. `foo` hashes to
/// 42 and `bar` hashes to 43) or because a hash collision occurred. This function does not
/// (cannot) distinguish between those two cases, because it does not have the original strings.
/// Instead, it just computes the places where a given string could legally be. The caller then
/// verifies that each hash entry is in a range that is valid for it.
#[allow(dead_code)]
fn find_collision_ranges(hashes: &[U32<LE>], i: usize) -> (Range<usize>, Range<usize>) {
    assert!(i < hashes.len());
    assert!(hashes[i].get() != 0);

    let mut start = i;
    while start > 0 && hashes[start - 1].get() != 0 {
        start -= 1;
    }

    let mut end = i + 1;
    while end < hashes.len() && hashes[end].get() != 0 {
        end += 1;
    }

    if start == 0 {
        // Special case: The entire hash table is one collision range.
        // We check for this because there are no unused slots in the table.
        if end == hashes.len() {
            return (start..end, 0..0);
        }

        let mut r2_start = hashes.len();
        while r2_start > 0 && hashes[r2_start - 1].get() != 0 {
            r2_start -= 1;
            assert!(r2_start > end); // prevent infinite loops
        }
        if r2_start != hashes.len() {
            (start..end, r2_start..hashes.len())
        } else {
            (start..end, 0..0)
        }
    } else if end == hashes.len() {
        // The end of the main range is aligned at the end of the buffer.
        // Wrap around to the beginning and find the range at the beginning, if any.
        let mut r2_end = 0;
        while r2_end < hashes.len() && hashes[r2_end].get() != 0 {
            assert!(r2_end < start); // prevent infinite loops
            r2_end += 1;
        }

        (start..end, 0..r2_end)
    } else {
        (start..end, 0..0)
    }
}

#[test]
fn test_find_collision_range() {
    const EMPTY: U32<LE> = U32::from_bytes([0; 4]);
    const BUSY: U32<LE> = U32::from_bytes([0xff; 4]);

    let hashes_full: Vec<U32<LE>> = vec![BUSY, BUSY, BUSY, BUSY, BUSY];
    assert_eq!(find_collision_ranges(&hashes_full, 0), (0..5, 0..0));
    assert_eq!(find_collision_ranges(&hashes_full, 2), (0..5, 0..0));

    {
        let hashes_2 = vec![
            BUSY,  // 0 - wraps around
            EMPTY, // 1
            BUSY,  // 2
            EMPTY, // 3
            EMPTY, // 4
            BUSY,  // 5
            BUSY,  // 6 - wraps around
        ];
        assert_eq!(find_collision_ranges(&hashes_2, 0), (0..1, 5..7));
        assert_eq!(find_collision_ranges(&hashes_2, 2), (2..3, 0..0));
        assert_eq!(find_collision_ranges(&hashes_2, 5), (5..7, 0..1));
    }

    {
        let hashes_3 = vec![
            BUSY,  // 0 - wraps around
            EMPTY, // 1
            BUSY,  // 2
            EMPTY, // 3
            EMPTY, // 4
            BUSY,  // 5
            EMPTY, // 6 - no wrap around
        ];
        assert_eq!(find_collision_ranges(&hashes_3, 0), (0..1, 0..0));
        assert_eq!(find_collision_ranges(&hashes_3, 2), (2..3, 0..0));
        assert_eq!(find_collision_ranges(&hashes_3, 5), (5..6, 0..0));
    }
    {
        let hashes_4 = vec![
            EMPTY, // 0 - no wrap around
            EMPTY, // 1
            BUSY,  // 2
            EMPTY, // 3
            EMPTY, // 4
            BUSY,  // 5
            BUSY,  // 6 - wraps around
        ];
        assert_eq!(find_collision_ranges(&hashes_4, 2), (2..3, 0..0));
        assert_eq!(find_collision_ranges(&hashes_4, 5), (5..7, 0..0));
        assert_eq!(find_collision_ranges(&hashes_4, 6), (5..7, 0..0));
    }
}

/// Iterator state
pub struct IterNames<'a> {
    rest: &'a [u8],
}

impl<'a> HasRestLen for IterNames<'a> {
    fn rest_len(&self) -> usize {
        self.rest.len()
    }
}

impl<'a> Iterator for IterNames<'a> {
    type Item = &'a BStr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.rest);
        let Ok(s) = p.strz() else {
            warn!(
                rest_len = self.rest.len(),
                "Found malformed string in /names stream"
            );
            return None;
        };

        self.rest = p.into_rest();
        Some(s)
    }
}

impl NamesStream<Vec<u8>> {
    /// Reads the Names Stream and parses its header.
    pub fn load_and_parse<F: ReadAt>(
        pdb: &crate::msf::Msf<F>,
        named_streams: &crate::pdbi::NamedStreams,
    ) -> anyhow::Result<Self> {
        let named_stream_index = named_streams.get_err(NAMES_STREAM_NAME)?;
        let named_stream_data = pdb.read_stream_to_vec(named_stream_index)?;
        Self::parse(named_stream_data)
    }
}
