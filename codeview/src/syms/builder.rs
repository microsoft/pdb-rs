//! Supports building new symbol streams

use super::SymKind;
use crate::encoder::Encoder;
use crate::types::TypeIndex;
use bstr::BStr;

/// Writes symbol records into a buffer.
#[derive(Default)]
pub struct SymBuilder {
    /// Contains the symbol stream
    pub buffer: Vec<u8>,
}

impl SymBuilder {
    /// Creates a new empty symbol stream builder.
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }

    /// Consumes this builder and returns the symbol stream.
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    /// Starts adding a new record to the builder.
    pub fn record(&mut self, kind: SymKind) -> RecordBuilder<'_> {
        let record_start = self.buffer.len();
        self.buffer.extend_from_slice(&[0, 0]); // placeholder for record length
        self.buffer.extend_from_slice(&kind.0.to_le_bytes());
        RecordBuilder {
            enc: Encoder::new(&mut self.buffer),
            record_start,
        }
    }

    /// Adds an `S_UDT` record.
    pub fn udt(&mut self, ty: TypeIndex, name: &BStr) {
        let mut r = self.record(SymKind::S_UDT);
        r.enc.u32(ty.0);
        r.enc.strz(name);
    }

    /// Adds an `S_PUB32` record.
    pub fn pub32(&mut self, flags: u32, offset: u32, segment: u16, name: &str) {
        let mut r = self.record(SymKind::S_PUB32);
        r.enc.u32(flags);
        r.enc.u32(offset);
        r.enc.u16(segment);
        r.enc.strz(name.into());
    }
}

/// State for writing a single record. When this is dropped, it will terminate the record.
pub struct RecordBuilder<'a> {
    /// Encoder which can write the payload of the current record.
    pub enc: Encoder<'a>,
    /// Byte offset of the start of the current record. We use this to patch the record length
    /// when we're done writing the record.
    record_start: usize,
}

impl<'a> Drop for RecordBuilder<'a> {
    fn drop(&mut self) {
        // Align the buffer to a 4-byte boundary
        match self.enc.buf.len() & 3 {
            1 => self.enc.buf.push(0xf1),
            2 => self.enc.buf.extend_from_slice(&[0xf1, 0xf2]),
            3 => self.enc.buf.extend_from_slice(&[0xf1, 0xf2, 0xf3]),
            _ => {}
        }

        let record_len = self.enc.buf.len() - self.record_start - 2;
        let record_field = &mut self.enc.buf[self.record_start..];
        record_field[0] = record_len as u8;
        record_field[1] = (record_len >> 8) as u8;
    }
}
