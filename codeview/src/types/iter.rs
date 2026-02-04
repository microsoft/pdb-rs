//! Code for iterating through type streams

use super::Leaf;
use crate::parser::{Parser, ParserError, ParserMut};
use crate::utils::iter::{HasRestLen, IteratorWithRangesExt};
use std::mem::take;

/// Parses a type record stream and iterates `TypeRecord` values.
#[derive(Clone)]
pub struct TypesIter<'a> {
    buffer: &'a [u8],
}

impl<'a> TypesIter<'a> {
    /// Starts a new iterator.
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer }
    }

    /// Returns the "rest" of the data that has not been parsed.
    pub fn rest(&self) -> &'a [u8] {
        self.buffer
    }
}

impl<'a> HasRestLen for TypesIter<'a> {
    fn rest_len(&self) -> usize {
        self.buffer.len()
    }
}

impl<'a> Iterator for TypesIter<'a> {
    type Item = TypeRecord<'a>;

    /// Finds the next type record
    ///
    /// This implementation makes an important guarantee: If it cannot decode the next record,
    /// it _will not_ change `self.buffer`. This is important because it allows an application
    /// to detect the exact length and contents of an unparseable record.
    fn next(&mut self) -> Option<TypeRecord<'a>> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.buffer);

        let record_len = p.u16().ok()?;
        if record_len < 2 {
            // Type record has length that is too short to be valid
            return None;
        }

        let type_kind = p.u16().ok()?;

        let Ok(record_data) = p.bytes(record_len as usize - 2) else {
            // Type record is too short to be valid.
            return None;
        };

        self.buffer = p.into_rest();

        Some(TypeRecord {
            data: record_data,
            kind: Leaf(type_kind),
        })
    }
}

/// Represents a record that was enumerated within a type record stream (the TPI or IPI).
#[derive(Clone)]
pub struct TypeRecord<'a> {
    /// Indicates how to interpret the payload (the `data` field).
    pub kind: Leaf,
    /// Record data. This does NOT include `kind` and the record data length.
    pub data: &'a [u8],
}

impl<'a> TypeRecord<'a> {
    /// Parses the payload of this type record.
    pub fn parse(&self) -> Result<crate::types::TypeData<'a>, ParserError> {
        crate::types::TypeData::parse(self.kind, &mut Parser::new(self.data))
    }
}

/// Builds a "starts" table that gives the starting location of each type record.
pub fn build_types_starts(num_records_expected: usize, type_records: &[u8]) -> Vec<u32> {
    let mut starts: Vec<u32> = Vec::with_capacity(num_records_expected + 1);
    let mut iter = TypesIter::new(type_records).with_ranges();

    // This loop pushes a byte offset (pos) for the start of every record, plus 1 additional
    // value at the end of the sequence.  This will correctly handle the case where the last
    // record has some undecodable garbage at the end.
    loop {
        let pos = iter.pos();
        starts.push(pos as u32);

        if iter.next().is_none() {
            break;
        }
    }

    starts.shrink_to_fit();
    starts
}

/// Parses a type record stream and iterates `TypeRecord` values.
pub struct TypesIterMut<'a> {
    buffer: &'a mut [u8],
}

impl<'a> TypesIterMut<'a> {
    /// Starts a new iterator.
    pub fn new(buffer: &'a mut [u8]) -> Self {
        Self { buffer }
    }
}

impl<'a> HasRestLen for TypesIterMut<'a> {
    fn rest_len(&self) -> usize {
        self.buffer.len()
    }
}

impl<'a> Iterator for TypesIterMut<'a> {
    type Item = TypeRecordMut<'a>;

    fn next(&mut self) -> Option<TypeRecordMut<'a>> {
        if self.buffer.is_empty() {
            return None;
        }

        let mut parser = ParserMut::new(take(&mut self.buffer));

        let record_len = parser.u16().ok()?;
        if record_len < 2 {
            // Type record has length that is too short to be valid
            return None;
        }

        let type_kind = parser.u16().ok()?;

        let Ok(record_data) = parser.bytes_mut(record_len as usize - 2) else {
            // Type record is too short to be valid.
            return None;
        };

        self.buffer = parser.into_rest();

        Some(TypeRecordMut {
            data: record_data,
            kind: Leaf(type_kind),
        })
    }
}

/// Represents a record that was enumerated within a type record stream (the TPI or IPI).
/// Allows mutable access.
pub struct TypeRecordMut<'a> {
    /// Indicates how to interpret the payload (the `data` field).
    pub kind: Leaf,
    /// Record data. This does NOT include `kind` and the record data length.
    pub data: &'a mut [u8],
}
