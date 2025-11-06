//! Type Information Stream (TPI)
//!
//! Layout of a Type Stream:
//!
//! * `TypeStreamHeader` - specifies lots of important parameters
//! * Type Record Data
//!
//! Each Type Stream may also have an associated Type Hash Stream. The Type Hash Stream contains
//! indexing information that helps find records within the main Type Stream. The Type Stream
//! Header specifies several parameters that are needed for finding and decoding the Type Hash
//! Stream.
//!
//! The Type Hash Stream contains:
//!
//! * Hash Value Buffer: Contains a list of hash values, one for each Type Record in the
//!   Type Stream.
//!
//!   The offset and size of the Hash Value Buffer is specified in the `TypeStreamHeader`, in the
//!   `hash_value_buffer_offset` and `hash_value_buffer_length` fields, respectively.
//!
//!   It should be assumed that there are either 0 hash values, or a number equal to the number of
//!   type records in the TPI stream (`type_index_end - type_end_begin`). Thus, if
//!   `hash_value_buffer_length` is not equal to `(type_index_end - type_end_begin) * hash_key_size`
//!   we can consider the PDB malformed.
//!
//! * Type Index Offset Buffer - A list of pairs of `u32` values where the first is a Type Index
//!   and the second is the offset within Type Record Data of the type with this index.
//!   This enables a binary search to find a given Type Index record.
//!
//!   The offset and size of the Type Index Offset Buffer is specified in the `TypeStreamHeader`,
//!   in the `index_offset_buffer_offset` and `index_offset_buffer_length` fields, respectively.
//!
//! * Hash Adjustment Buffer - A hash table whose keys are the hash values in the hash value
//!   buffer and whose values are type indices.
//!
//!   The offset and size of the Type Index Offset BUffer is specified in the `TypeStreamHeader`,
//!   in the `index_offset_buffer_offset` and `index_offset_buffer_length` fields, respectively.

pub mod hash;

use super::*;
use crate::types::fields::{Field, IterFields};
use crate::types::{build_types_starts, TypeData, TypeIndex, TypeIndexLe, TypeRecord, TypesIter};
use anyhow::bail;
use ms_codeview::parser::Parser;
use std::fmt::Debug;
use std::mem::size_of;
use std::ops::Range;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, I32, LE, U32};

/// The header of the TPI stream.
#[allow(missing_docs)]
#[derive(Clone, Eq, PartialEq, IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Debug)]
#[repr(C)]
pub struct TypeStreamHeader {
    pub version: U32<LE>,
    pub header_size: U32<LE>,
    pub type_index_begin: TypeIndexLe,
    pub type_index_end: TypeIndexLe,
    /// The number of bytes of type record data following the `TypeStreamHeader`.
    pub type_record_bytes: U32<LE>,

    pub hash_stream_index: StreamIndexU16,
    pub hash_aux_stream_index: StreamIndexU16,

    /// The size of each hash key in the Hash Value Substream. For the current version of TPI,
    /// this value should always be 4.
    pub hash_key_size: U32<LE>,
    /// The number of hash buckets. This is used when calculating the record hashes. Each hash
    /// is computed, and then it is divided by num_hash_buckets and the remainder becomes the
    /// final hash.
    ///
    /// If `hash_value_buffer_length` is non-zero, then `num_hash_buckets` must also be non-zero.
    pub num_hash_buckets: U32<LE>,
    pub hash_value_buffer_offset: I32<LE>,
    pub hash_value_buffer_length: U32<LE>,

    pub index_offset_buffer_offset: I32<LE>,
    pub index_offset_buffer_length: U32<LE>,

    pub hash_adj_buffer_offset: I32<LE>,
    pub hash_adj_buffer_length: U32<LE>,
}

impl TypeStreamHeader {
    /// Makes an empty one
    pub fn empty() -> Self {
        Self {
            version: Default::default(),
            header_size: U32::new(size_of::<TypeStreamHeader>() as u32),
            type_index_begin: TypeIndexLe(U32::new(TypeIndex::MIN_BEGIN.0)),
            type_index_end: TypeIndexLe(U32::new(TypeIndex::MIN_BEGIN.0)),
            type_record_bytes: Default::default(),
            hash_stream_index: StreamIndexU16::NIL,
            hash_aux_stream_index: StreamIndexU16::NIL,
            hash_key_size: Default::default(),
            num_hash_buckets: Default::default(),
            hash_value_buffer_offset: Default::default(),
            hash_value_buffer_length: Default::default(),
            index_offset_buffer_offset: Default::default(),
            index_offset_buffer_length: Default::default(),
            hash_adj_buffer_offset: Default::default(),
            hash_adj_buffer_length: Default::default(),
        }
    }
}

/// The size of the `TpiStreamHeader` structure.
pub const TPI_STREAM_HEADER_LEN: usize = size_of::<TypeStreamHeader>();

/// The expected value of `TypeStreamHeader::version`.
pub const TYPE_STREAM_VERSION_2004: u32 = 20040203;

/// Contains a TPI Stream or IPI Stream.
pub struct TypeStream<StreamData>
where
    StreamData: AsRef<[u8]>,
{
    /// The stream data. This contains the entire type stream, including header and type records.
    pub stream_data: StreamData,

    type_index_begin: TypeIndex,
    type_index_end: TypeIndex,

    /// A starts vector for type record offsets. This is created on-demand, since many users of
    /// `TypeStream` do not need this.
    record_starts: OnceCell<Vec<u32>>,
}

/// Distinguishes the TPI and IPI streams.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum TypeStreamKind {
    /// The primary type stream
    TPI,
    /// The ID stream
    IPI,
}

impl TypeStreamKind {
    /// Get the stream index. Fortunately, the stream indexes are fixed.
    pub fn stream(self) -> Stream {
        match self {
            Self::IPI => Stream::IPI,
            Self::TPI => Stream::TPI,
        }
    }
}

/// Represents an entry in the Hash Index Offset Substream.
#[repr(C)]
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Debug)]
pub struct HashIndexPair {
    /// The type index at the start of this range.
    pub type_index: TypeIndexLe,
    /// The offset within the Type Records Substream (not the entire Type Stream) where this
    /// record begins.
    pub offset: U32<LE>,
}

impl<StreamData> TypeStream<StreamData>
where
    StreamData: AsRef<[u8]>,
{
    /// Gets a reference to the stream header
    pub fn header(&self) -> Option<&TypeStreamHeader> {
        let stream_data: &[u8] = self.stream_data.as_ref();
        let (header, _) = TypeStreamHeader::ref_from_prefix(stream_data).ok()?;
        Some(header)
    }

    /// Returns the version of the stream, or `TYPE_STREAM_VERSION_2004` if this is an empty stream.
    pub fn version(&self) -> u32 {
        if let Some(header) = self.header() {
            header.version.get()
        } else {
            TYPE_STREAM_VERSION_2004
        }
    }

    /// Returns the stream index of the related hash stream, if any.
    pub fn hash_stream(&self) -> Option<u32> {
        self.header()?.hash_stream_index.get()
    }

    /// Checks whether this is a degenerate empty stream.
    pub fn is_empty(&self) -> bool {
        self.stream_data.as_ref().is_empty()
    }

    /// Gets a mutable reference to the stream header
    pub fn header_mut(&mut self) -> Option<&mut TypeStreamHeader>
    where
        StreamData: AsMut<[u8]>,
    {
        let (header, _) = TypeStreamHeader::mut_from_prefix(self.stream_data.as_mut()).ok()?;
        Some(header)
    }

    /// The type index of the first type record.
    pub fn type_index_begin(&self) -> TypeIndex {
        self.type_index_begin
    }

    /// The type index of the last type record, plus 1.
    pub fn type_index_end(&self) -> TypeIndex {
        self.type_index_end
    }

    /// The number of types defined in the type stream.
    pub fn num_types(&self) -> u32 {
        self.type_index_end.0 - self.type_index_begin.0
    }

    /// Gets the byte offset within the stream of the record data.
    pub fn records_offset(&self) -> usize {
        if let Some(header) = self.header() {
            header.header_size.get() as usize
        } else {
            0
        }
    }

    /// Returns the encoded type records found in the TPI or IPI stream.
    ///
    /// The type records immediately follow the type stream. The length is given by the
    /// header field type_record_bytes. The values in the header were validated in
    /// read_tpi_or_ipi_stream(), so we do not need to check them again, here.
    pub fn type_records_bytes(&self) -> &[u8] {
        let records_range = self.type_records_range();
        if records_range.is_empty() {
            &[]
        } else {
            &self.stream_data.as_ref()[records_range]
        }
    }

    /// Returns the encoded type records found in the type stream.
    pub fn type_records_bytes_mut(&mut self) -> &mut [u8]
    where
        StreamData: AsMut<[u8]>,
    {
        let records_range = self.type_records_range();
        if records_range.is_empty() {
            &mut []
        } else {
            &mut self.stream_data.as_mut()[records_range]
        }
    }

    /// Returns the byte range of the encoded type records found in the type stream.
    pub fn type_records_range(&self) -> std::ops::Range<usize> {
        if let Some(header) = self.header() {
            let size = header.type_record_bytes.get();
            if size == 0 {
                return 0..0;
            }
            let type_records_start = header.header_size.get();
            let type_records_end = type_records_start + size;
            type_records_start as usize..type_records_end as usize
        } else {
            0..0
        }
    }

    /// Iterates the types contained within this type stream.
    pub fn iter_type_records(&self) -> TypesIter<'_> {
        TypesIter::new(self.type_records_bytes())
    }

    /// Parses the header of a Type Stream and validates it.
    pub fn parse(stream_index: Stream, stream_data: StreamData) -> anyhow::Result<Self> {
        let stream_bytes: &[u8] = stream_data.as_ref();

        if stream_bytes.is_empty() {
            return Ok(Self {
                stream_data,
                type_index_begin: TypeIndex::MIN_BEGIN,
                type_index_end: TypeIndex::MIN_BEGIN,
                record_starts: OnceCell::new(),
            });
        }

        let mut p = Parser::new(stream_bytes);
        let tpi_stream_header: TypeStreamHeader = p.copy()?;

        let type_index_begin = tpi_stream_header.type_index_begin.get();
        let type_index_end = tpi_stream_header.type_index_end.get();
        if type_index_end < type_index_begin {
            bail!(
                "Type stream (stream {stream_index}) has invalid values in header.  \
                 The type_index_begin field is greater than the type_index_end field."
            );
        }

        if type_index_begin < TypeIndex::MIN_BEGIN {
            bail!(
                "The Type Stream has an invalid value for type_index_begin ({type_index_begin:?}). \
                 It is less than the minimum required value ({}).",
                TypeIndex::MIN_BEGIN.0
            );
        }

        let type_data_start = tpi_stream_header.header_size.get();
        if type_data_start < TPI_STREAM_HEADER_LEN as u32 {
            bail!(
                "Type stream (stream {stream_index}) has invalid values in header.  \
                 The header_size field is smaller than the definition of the actual header."
            );
        }

        let type_data_end = type_data_start + tpi_stream_header.type_record_bytes.get();
        if type_data_end > stream_bytes.len() as u32 {
            bail!(
                "Type stream (stream {stream_index}) has invalid values in header.  \
                   The header_size and type_record_bytes fields exceed the size of the stream."
            );
        }

        Ok(TypeStream {
            stream_data,
            type_index_begin,
            type_index_end,
            record_starts: OnceCell::new(),
        })
    }

    /// Builds a "starts" table that gives the starting location of each type record.
    pub fn build_types_starts(&self) -> TypeIndexMap {
        let starts =
            crate::types::build_types_starts(self.num_types() as usize, self.type_records_bytes());

        TypeIndexMap {
            type_index_begin: self.type_index_begin,
            type_index_end: self.type_index_end,
            starts,
        }
    }

    /// Creates a new `TypeStream` that referenced the stream data of this `TypeStream`.
    /// This is typically used for temporarily creating a `TypeStream<&[u8]>` from a
    /// `TypeStream<Vec<u8>>`.
    pub fn to_ref(&self) -> TypeStream<&[u8]> {
        TypeStream {
            stream_data: self.stream_data.as_ref(),
            type_index_begin: self.type_index_begin,
            type_index_end: self.type_index_end,
            record_starts: OnceCell::new(),
        }
    }

    /// Gets the "starts" vector for the byte offsets of the records in this `TypeStream`.
    ///
    /// This function will create the starts vector on-demand.
    pub fn record_starts(&self) -> &[u32] {
        self.record_starts.get_or_init(|| {
            let type_records = self.type_records_bytes();
            build_types_starts(self.num_types() as usize, type_records)
        })
    }

    /// Returns `true` if `type_index` refers to a primitive type.
    pub fn is_primitive(&self, type_index: TypeIndex) -> bool {
        type_index < self.type_index_begin
    }

    /// Retrieves the type record identified by `type_index`.
    ///
    /// This should only be used for non-primitive `TypeIndex` values. If this is called with a
    /// primitive `TypeIndex` then it will return `Err`.
    pub fn record(&self, type_index: TypeIndex) -> anyhow::Result<TypeRecord<'_>> {
        let Some(relative_type_index) = type_index.0.checked_sub(self.type_index_begin.0) else {
            bail!("The given TypeIndex is a primitive type index, not a type record.");
        };

        let starts = self.record_starts();
        let Some(&record_start) = starts.get(relative_type_index as usize) else {
            bail!("The given TypeIndex {type_index:?} is out of bounds (exceeds maximum allowed TypeIndex)");
        };

        let all_type_records = self.type_records_bytes();
        let Some(this_type_record_slice) = all_type_records.get(record_start as usize..) else {
            // This should never happen, but let's be cautious.
            bail!("Internal error: record offset is out of range.");
        };

        let mut iter = TypesIter::new(this_type_record_slice);
        if let Some(record) = iter.next() {
            Ok(record)
        } else {
            bail!("Failed to decode type record");
        }
    }

    /// Iterate the fields of an `LF_STRUCTURE`, `LF_CLASS`, `LF_ENUM`, etc. This correctly
    /// iterates across chains of `LF_FIELDLIST`.
    pub fn iter_fields(&self, field_list: TypeIndex) -> IterFieldChain<'_, StreamData> {
        // We initialize `fields` to an empty iterator so that the first iteration of
        // IterFieldChain::next() will find no records and will then check next_field_list.
        IterFieldChain {
            type_stream: self,
            next_field_list: if field_list.0 != 0 {
                Some(field_list)
            } else {
                None
            },
            fields: IterFields { bytes: &[] },
        }
    }
}

/// Iterator state for `iter_fields`
pub struct IterFieldChain<'a, StreamData>
where
    StreamData: AsRef<[u8]>,
{
    /// The current `LF_FIELDLIST` record that we are decoding.
    fields: IterFields<'a>,

    /// Allows us to read `LF_FIELDLIST` records.
    type_stream: &'a TypeStream<StreamData>,

    /// The pointer to the next `LF_FIELDLIST` that we will decode.
    next_field_list: Option<TypeIndex>,
}

impl<'a, StreamData> Iterator for IterFieldChain<'a, StreamData>
where
    StreamData: AsRef<[u8]>,
{
    type Item = Field<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(field) = self.fields.next() {
                if let Field::Index(index) = &field {
                    // The full field list is split across more than one LF_FIELDLIST record.
                    // Store the link to the next field list and do not return this item to the caller.
                    self.next_field_list = Some(*index);
                    continue;
                }

                return Some(field);
            }

            // We have run out of fields in the current LF_FIELDLIST record.
            // See if there is a pointer to another LF_FIELDLIST.
            let next_field_list = self.next_field_list.take()?;
            let next_record = self.type_stream.record(next_field_list).ok()?;
            match next_record.parse().ok()? {
                TypeData::FieldList(fl) => {
                    // Restart iteration on the new field list.
                    self.fields = fl.iter();
                }
                _ => {
                    // Wrong record type!
                    return None;
                }
            }
        }
    }
}

impl<F: ReadAt> crate::Pdb<F> {
    /// Reads the TPI stream.
    pub fn read_type_stream(&self) -> anyhow::Result<TypeStream<Vec<u8>>> {
        self.read_tpi_or_ipi_stream(Stream::TPI)
    }

    /// Reads the IPI stream.
    pub fn read_ipi_stream(&self) -> anyhow::Result<TypeStream<Vec<u8>>> {
        self.read_tpi_or_ipi_stream(Stream::IPI)
    }

    /// Reads the TPI or IPI stream.
    pub fn read_tpi_or_ipi_stream(
        &self,
        stream_index: Stream,
    ) -> anyhow::Result<TypeStream<Vec<u8>>> {
        let stream_data = self.read_stream_to_vec(stream_index.into())?;
        TypeStream::parse(stream_index, stream_data)
    }
}

/// Maps `TypeIndex` values to the byte range of records within a type stream.
pub struct TypeIndexMap {
    /// Copied from type stream header.
    pub type_index_begin: TypeIndex,

    /// Copied from type stream header.
    pub type_index_end: TypeIndex,

    /// Contains a "starts" vector for the byte offsets of each type record.
    ///
    /// This vector has an additional value at the end, which gives the size in bytes of the
    /// type stream.
    pub starts: Vec<u32>,
}

impl TypeIndexMap {
    /// Tests whether a `TypeIndex` is a primitive type.
    pub fn is_primitive(&self, ti: TypeIndex) -> bool {
        ti < self.type_index_begin
    }

    /// Given a `TypeIndex`, returns the byte range within a `TypeStream` where that record
    /// is stored.
    ///
    /// If `ti` is a primitive type then this function will return `Err`. The caller should
    /// use the `is_primitive` method to check whether a `TypeIndex` is a primitive type.
    pub fn record_range(&self, ti: TypeIndex) -> anyhow::Result<Range<usize>> {
        let Some(i) = ti.0.checked_sub(self.type_index_begin.0) else {
            bail!("The TypeIndex is a primitive type, not a type record.");
        };

        if let Some(w) = self.starts.get(i as usize..i as usize + 2) {
            Ok(w[0] as usize..w[1] as usize)
        } else {
            bail!("The TypeIndex is out of range.");
        }
    }
}

/// Represents the cached state of a Type Stream header.
pub struct CachedTypeStreamHeader {
    pub(crate) header: Option<TypeStreamHeader>,
}

impl CachedTypeStreamHeader {
    /// Gets direct access to the type stream header, if any.
    pub fn header(&self) -> Option<&TypeStreamHeader> {
        self.header.as_ref()
    }

    /// Gets the beginning of the type index space, or `TypeIndex::MIN_BEGIN` if this type stream
    /// does not contain any data.
    pub fn type_index_begin(&self) -> TypeIndex {
        if let Some(h) = &self.header {
            h.type_index_begin.get()
        } else {
            TypeIndex::MIN_BEGIN
        }
    }

    /// Gets the end of the type index space, or `TypeIndex::MIN_BEGIN` if this type stream does
    /// not contain any data.
    pub fn type_index_end(&self) -> TypeIndex {
        if let Some(h) = &self.header {
            h.type_index_end.get()
        } else {
            TypeIndex::MIN_BEGIN
        }
    }
}
