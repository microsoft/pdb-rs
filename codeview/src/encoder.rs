//! Support for encoding primitives and blittable types into output buffers.
#![allow(missing_docs)]

use bstr::BStr;
use uuid::Uuid;
use zerocopy::{Immutable, IntoBytes};

/// A simple type which helps encode CodeView records into a buffer.
pub struct Encoder<'a> {
    pub buf: &'a mut Vec<u8>,
}

impl<'a> Encoder<'a> {
    pub fn new(buf: &'a mut Vec<u8>) -> Self {
        Self { buf }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn u8(&mut self, x: u8) {
        self.buf.push(x);
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.buf.extend_from_slice(b);
    }

    pub fn u16(&mut self, x: u16) {
        self.bytes(&x.to_le_bytes());
    }

    pub fn u32(&mut self, x: u32) {
        self.bytes(&x.to_le_bytes());
    }

    pub fn t<T: IntoBytes + Immutable>(&mut self, x: &T) {
        self.buf.extend_from_slice(x.as_bytes());
    }

    pub fn strz(&mut self, s: &BStr) {
        self.buf.extend_from_slice(s);
        self.buf.push(0);
    }

    pub fn uuid(&mut self, u: &Uuid) {
        self.bytes(&u.to_bytes_le())
    }
}
