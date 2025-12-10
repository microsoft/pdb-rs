//! PDB Info Stream (aka the PDB Stream)
//!
//! # References
//! * <https://llvm.org/docs/PDB/PdbStream.html>

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use super::*;
use crate::guid::GuidLe;
use anyhow::bail;
use bitvec::prelude::{BitSlice, Lsb0};
use bstr::ByteSlice;
use ms_codeview::encoder::Encoder;
use ms_codeview::parser::Parser;
use tracing::{trace, trace_span, warn};
use uuid::Uuid;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, LE, U32, Unaligned};

/// Contains the PDB Information Stream.
///
/// This implementation reads all of the data from the PDBI Stream and converts it to in-memory
/// data structures. This is not typical for most of the data within the PDB. We do this because
/// the PDBI is fairly small, is needed for reading most PDBs, and will often need to be edited
/// for generating or rebuilding PDBs.
#[allow(missing_docs)]
#[derive(Clone)]
pub struct PdbiStream {
    pub signature: u32,
    pub version: u32,
    pub age: u32,
    pub unique_id: Option<Uuid>,
    pub named_streams: NamedStreams,
    pub features: Vec<FeatureCode>,
}

impl PdbiStream {
    /// Parses the stream.
    pub fn parse(stream_data: &[u8]) -> anyhow::Result<Self> {
        let mut p = Parser::new(stream_data);

        let header: &PdbiStreamHeader = p.get()?;
        let version = header.version.get();

        // Older PDBs (pre-VC7, i.e. before 2000) do not contain a GUID.
        let unique_id = if pdbi_has_unique_id(version) {
            // Check that the stream data is large enough to contain the unique ID.
            // We use slices, below, relying on bounds checking here.
            Some(p.get::<GuidLe>()?.get())
        } else {
            None
        };

        let named_streams = NamedStreams::parse(&mut p)?;

        // The last part of the PDBI stream is a list of "features". Features are u32 values, and
        // the feature values are defined as constants. If a feature is present in this list, then
        // that feature is enabled.
        let mut features: Vec<FeatureCode> = Vec::with_capacity(p.len() / 4);
        while p.len() >= 4 {
            let feature = FeatureCode(p.u32()?);
            features.push(feature);
        }

        Ok(Self {
            signature: header.signature.get(),
            version,
            age: header.age.get(),
            unique_id,
            named_streams,
            features,
        })
    }

    /// Serializes this to a stream.
    pub fn to_bytes(&self) -> anyhow::Result<Vec<u8>> {
        let mut out = Vec::new();

        let mut e = Encoder::new(&mut out);

        let header = PdbiStreamHeader {
            signature: U32::new(self.signature),
            version: U32::new(self.version),
            age: U32::new(self.age),
        };

        e.t(&header);
        if pdbi_has_unique_id(self.version) {
            if let Some(unique_id) = &self.unique_id {
                e.uuid(unique_id);
            } else {
                bail!("The PDBI version requires a unique ID, but none has been provided.");
            }
        } else if self.unique_id.is_some() {
            warn!(
                "PDBI version is too old to have a unique ID, but this PdbiStream has a unique ID. It will be ignored."
            );
        }

        self.named_streams.to_bytes(&mut e);

        // Write the features.
        for &feature in self.features.iter() {
            e.u32(feature.0);
        }

        Ok(out)
    }

    /// Gets the 'age' value of the PDB. This links the PDB with the executable; a PDB must have
    /// the same age as its related executable.
    pub fn age(&self) -> u32 {
        self.age
    }

    /// Version from the PDBI header, e.g. [`PDBI_VERSION_VC110`].
    pub fn version(&self) -> u32 {
        self.version
    }

    /// The binding key that associates this PDB with a given PE executable.
    pub fn binding_key(&self) -> BindingKey {
        BindingKey {
            guid: self.unique_id.unwrap_or(Uuid::nil()),
            age: self.age,
        }
    }

    /// Provides access to the named streams table.
    pub fn named_streams(&self) -> &NamedStreams {
        &self.named_streams
    }

    /// Provides mutable access to the named streams table.
    pub fn named_streams_mut(&mut self) -> &mut NamedStreams {
        &mut self.named_streams
    }

    /// Checks whether this PDB has a given feature enabled.
    pub fn has_feature(&self, feature_code: FeatureCode) -> bool {
        self.features.contains(&feature_code)
    }
}

#[allow(missing_docs)]
pub const PDBI_VERSION_VC2: u32 = 19941610;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC4: u32 = 19950623;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC41: u32 = 19950814;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC50: u32 = 19960307;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC98: u32 = 19970604;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC70_DEPRECATED: u32 = 19990604; // deprecated vc70 implementation version
#[allow(missing_docs)]
pub const PDBI_VERSION_VC70: u32 = 20000404; // <-- first version that has unique id
#[allow(missing_docs)]
pub const PDBI_VERSION_VC80: u32 = 20030901;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC110: u32 = 20091201;
#[allow(missing_docs)]
pub const PDBI_VERSION_VC140: u32 = 20140508;

fn pdbi_has_unique_id(version: u32) -> bool {
    version > PDBI_VERSION_VC70_DEPRECATED
}

/// The header of the PDB Info stream.
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct PdbiStreamHeader {
    pub version: U32<LE>,
    pub signature: U32<LE>,
    pub age: U32<LE>,
    // This is only present if the version number is higher than impvVC70Dep.
    // pub unique_id: GuidLe,
}

#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct HashTableHeader {
    pub size: U32<LE>,
    pub capacity: U32<LE>,
    // present bit vector
    // deleted bit vector
    // (key, value) pairs
}

#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Debug)]
#[repr(C)]
#[allow(missing_docs)]
pub struct HashEntry {
    pub key: U32<LE>,
    pub value: U32<LE>,
}

/// Provides access to the Named Streams Table.
#[derive(Default, Clone)]
pub struct NamedStreams {
    /// If true, the named streams set has been modified since it was loaded.
    pub(crate) modified: bool,

    /// Stores the mapping.
    ///
    /// We use `BTreeMap` so that the names are ordered.
    map: BTreeMap<String, u32>,
}

impl NamedStreams {
    /// Iterates the named streams.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &u32)> {
        self.map.iter()
    }

    /// Searches the list of named strings for `name`. If found, returns the stream index.
    ///
    /// This does _not_ use a hash function. It just sequentially searches.
    /// This uses a case-sensitive comparison.
    pub fn get(&self, name: &str) -> Option<u32> {
        self.map.get(name).copied()
    }

    /// Searches the list of named strings for `name`. If found, returns the stream index.
    /// If not found, returns a descriptive error.
    ///
    /// This does _not_ use a hash function. It just sequentially searches.
    /// This uses a case-sensitive comparison.
    pub fn get_err(&self, name: &str) -> anyhow::Result<u32> {
        if let Some(&stream) = self.map.get(name) {
            Ok(stream)
        } else {
            bail!("Failed to find a named stream {:?}", name);
        }
    }

    /// Parses a `NamedStreams` table.
    pub fn parse(p: &mut Parser) -> anyhow::Result<Self> {
        let names_size = p.u32()?;
        let names_data = p.bytes(names_size as usize)?;

        // This is the "cdr" (cardinality) field in pdb.cpp.
        let name_count = p.u32()?;
        let _name_hash_size = p.u32()?;

        let present_u32_count = p.u32()?;
        let present_mask = p.bytes(present_u32_count as usize * 4)?;
        let present_num_items: u32 = present_mask.iter().map(|&b| b.count_ones()).sum();

        let deleted_u32_count = p.u32()?;
        let deleted_mask = p.bytes(deleted_u32_count as usize * 4)?;
        let _deleted_num_items: u32 = deleted_mask.iter().map(|&b| b.count_ones()).sum();

        if present_num_items != name_count {
            bail!(
                "The PDBI name table contains inconsistent values.  Name count is {}, but present bitmap count is {}.",
                name_count,
                present_num_items
            );
        }

        let items: &[HashEntry] = p.slice(name_count as usize)?;

        let mut names: BTreeMap<String, u32> = BTreeMap::new();

        for item in items.iter() {
            let key = item.key.get();
            let stream = item.value.get();
            // Key is a byte offset into names_data.
            // Value is a stream index.

            let mut kp = Parser::new(names_data);
            kp.skip(key as usize)?;
            let name = kp.strz()?.to_str_lossy();

            if let Some(existing_stream) = names.get(&*name) {
                warn!(
                    "The PDBI contains more than one stream with the same name {:?}: stream {} vs stream {}",
                    name, existing_stream, stream
                );
                continue;
            }

            names.insert(name.to_string(), stream);
        }

        // Parse the "number of NameIndex" values at the end (niMac).
        let num_name_index = p.u32()?;
        if num_name_index != 0 {
            warn!(
                "The Named Streams table contains a non-zero value for the 'niMac' field. This is not supported"
            );
        }

        Ok(Self {
            modified: false,
            map: names,
        })
    }

    /// Inserts a new named stream.
    ///
    /// Returns `true` if the mapping was inserted.
    ///
    /// Returns `false` if there was already a mapping with the given name. In this case, the
    /// named stream table is not modified.
    pub fn insert(&mut self, name: &str, value: u32) -> bool {
        if self.map.contains_key(name) {
            false
        } else {
            self.modified = true;
            self.map.insert(name.to_string(), value);
            true
        }
    }

    /// Removes all entries from the named stream map.
    pub fn clear(&mut self) {
        self.modified = true;
        self.map.clear();
    }

    /// Encode this table to a byte stream
    pub fn to_bytes(&self, e: &mut Encoder) {
        let _span = trace_span!("NamedStreams::to_bytes").entered();

        // Sort the names in the table, so that we have a deterministic order.
        let mut sorted_names: Vec<(&String, u32)> = Vec::with_capacity(self.map.len());
        for (name, stream) in self.map.iter() {
            sorted_names.push((name, *stream));
        }
        sorted_names.sort_unstable();
        let num_names = sorted_names.len();

        // Find the size of the string data table and find the position of every string in that
        // table. We have to do this after sorting the strings.
        let mut strings_len: usize = 0;
        let name_offsets: Vec<u32> = sorted_names
            .iter()
            .map(|(name, _)| {
                let this_pos = strings_len;
                strings_len += name.len() + 1;
                this_pos as u32
            })
            .collect();

        // Write the string data. This is prefixed by the length of the string data.
        e.u32(strings_len as u32);
        for &(name, _) in sorted_names.iter() {
            e.strz(BStr::new(name));
        }

        // We are going to encode this hash table using the format defined by PDBI.  This format
        // is a hash table that uses linear probing.  We choose a load factor of 2x, then hash all
        // the items and place them in the table.
        //
        // Choose a hash size that is larger than our list of names.
        let hash_size = if sorted_names.is_empty() {
            10
        } else {
            sorted_names.len() * 2
        };

        // Find the size of the "present" and "deleted" bitmaps. These bitmaps have the same size.
        let bitmap_size_u32s = hash_size.div_ceil(32);
        let mut present_bitmap_bytes: Vec<u8> = vec![0; bitmap_size_u32s * 4];
        let present_bitmap: &mut BitSlice<u8, Lsb0> =
            BitSlice::from_slice_mut(present_bitmap_bytes.as_mut_slice());

        // hash_slots contains (string_index, stream)
        let mut hash_slots: Vec<Option<(u32, u32)>> = Vec::new();
        hash_slots.resize_with(hash_size, Default::default);

        trace!(num_names, hash_size);

        // Assign all strings to hash slots.
        for (i, &(name, stream)) in sorted_names.iter().enumerate() {
            let name_offset = name_offsets[i];
            let h = crate::hash::hash_mod_u16(name.as_bytes(), 0xffff_ffff) as usize % hash_size;
            let mut slot = h;
            loop {
                if hash_slots[slot].is_none() {
                    hash_slots[slot] = Some((name_offset, stream));
                    present_bitmap.set(slot, true);
                    trace!(
                        assigned_name = name,
                        hash = h,
                        slot = slot,
                        name_offset,
                        stream
                    );
                    break;
                }
                slot += 1;
                assert_ne!(
                    slot, h,
                    "linear probing should not wrap around to starting slot"
                );
                if slot == hash_slots.len() {
                    slot = 0;
                }
            }
        }

        // Write the "cardinality" (number of elements in the table) field.
        e.u32(num_names as u32);

        // Write the number of hashes field.
        e.u32(hash_size as u32);

        // Write the "present" bitmap.
        e.u32(bitmap_size_u32s as u32);
        e.bytes(&present_bitmap_bytes);

        // Write the "deleted" bitmap.
        e.u32(bitmap_size_u32s as u32);
        for _ in 0..bitmap_size_u32s {
            e.u32(0);
        }

        // Write the entries from the hash table that are present.
        for slot in hash_slots.iter() {
            if let Some(slot) = slot {
                e.u32(slot.0);
                e.u32(slot.1);
            }
        }

        // Write the "number of NameIndex values" (niMac).
        e.u32(0);
    }
}

/// A feature code is a `u32` value that indicates that an optional feature is enabled for a given PDB.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Ord, PartialOrd)]
pub struct FeatureCode(pub u32);

impl FeatureCode {
    /// Indicates that this PDB is a "mini PDB", produced by using the `/DEBUG:FASTLINK` parameter.
    ///
    /// See: <https://learn.microsoft.com/en-us/cpp/build/reference/debug-generate-debug-info?view=msvc-170>
    pub const MINI_PDB: FeatureCode = FeatureCode(0x494E494D);
}
