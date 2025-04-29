//! Support for parsing byte-oriented data

#[cfg(test)]
mod tests;

use crate::types::TypeIndex;
use bstr::{BStr, ByteSlice};
use std::mem::{size_of, take};
use zerocopy::byteorder::{I16, I32, I64, LE, U16, U32, U64};
use zerocopy::{FromBytes, I128, Immutable, IntoBytes, KnownLayout, U128, Unaligned};

pub use crate::types::number::Number;

/// A byte-oriented parser, for use in decoding CodeView records.
#[derive(Clone)]
pub struct Parser<'a> {
    /// The bytes that have not yet been parsed.
    pub bytes: &'a [u8],
}

impl<'a> Parser<'a> {
    /// Starts a new parser.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Gets the rest of the unparsed bytes in the parser. The parser still retains a reference to
    /// the same data.
    pub fn peek_rest(&self) -> &'a [u8] {
        self.bytes
    }

    /// Gets the rest of the unparsed
    pub fn take_rest(&mut self) -> &'a [u8] {
        take(&mut self.bytes)
    }

    /// Consumes this `Parser` and returns the unparsed bytes within it.
    ///
    /// This should be used in situations where there is no valid reason to use the `Parser`
    /// after taking the rest of the bytes within it. In situations where a `parse()` method only
    /// has access to `&mut Parser`, then this function cannot be used, and the caller should use
    /// `Parser::take_rest`.
    pub fn into_rest(self) -> &'a [u8] {
        self.bytes
    }

    /// Indicates whether there are any bytes left to parse.
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    /// Returns the number of unparsed bytes in the parser.
    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    /// Checks that the buffer has at least `n` bytes.
    ///
    /// This can be used as an optimization improvement in some situations. Ordinarily, code like
    /// this will compile to a series of bounds checks:
    ///
    /// ```ignore
    /// let mut p = Parser::new(bytes);
    /// let a = p.u32()?;
    /// let b = p.u16()?;
    /// let c = p.u16()?;
    /// let d = p.u32()?;
    /// ```
    ///
    /// Inserting a `a.needs(12)?` statement can sometimes enable the compiler to collapse a
    /// series of bounds checks (4, in this case) to a single bounds check.
    #[inline(always)]
    pub fn needs(&self, n: usize) -> Result<(), ParserError> {
        if n <= self.bytes.len() {
            Ok(())
        } else {
            Err(ParserError::new())
        }
    }

    /// Takes the next `n` bytes of input and returns a slice to it. The parser is advanced by `n`.
    #[inline(always)]
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], ParserError> {
        if self.bytes.len() < n {
            return Err(ParserError::new());
        }

        let (lo, hi) = self.bytes.split_at(n);
        self.bytes = hi;
        Ok(lo)
    }

    /// Skips `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), ParserError> {
        if self.bytes.len() < n {
            return Err(ParserError::new());
        }

        self.bytes = &self.bytes[n..];
        Ok(())
    }

    /// Parses a reference to a structure. The input must contain at least [`size_of::<T>()`] bytes.
    #[inline(always)]
    pub fn get<T: FromBytes + Unaligned + KnownLayout + Immutable>(
        &mut self,
    ) -> Result<&'a T, ParserError> {
        if let Ok((value, rest)) = T::ref_from_prefix(self.bytes) {
            self.bytes = rest;
            Ok(value)
        } else {
            Err(ParserError::new())
        }
    }

    /// Parses a copy of a structure. The input must contain at least [`size_of::<T>()`] bytes.
    #[inline(always)]
    pub fn copy<T: FromBytes + Unaligned>(&mut self) -> Result<T, ParserError> {
        let item = self.bytes(size_of::<T>())?;
        Ok(T::read_from_bytes(item).unwrap())
    }

    /// Parses a `T` from the input, if `T` knows how to read from a `Parser`.
    ///
    /// This exists mainly to allow more succinct calls, using type inference.
    #[inline(always)]
    pub fn parse<T: Parse<'a>>(&mut self) -> Result<T, ParserError> {
        T::from_parser(self)
    }

    /// Parses a slice of items. The input must contain at least [`size_of::<T>() * n`] bytes.
    pub fn slice<T: FromBytes + Unaligned + Immutable>(
        &mut self,
        len: usize,
    ) -> Result<&'a [T], ParserError> {
        if let Ok((lo, hi)) = <[T]>::ref_from_prefix_with_elems(self.bytes, len) {
            self.bytes = hi;
            Ok(lo)
        } else {
            Err(ParserError::new())
        }
    }

    /// Copies an array of items with a constant size and advances the parser.
    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], ParserError> {
        let s = self.bytes(N)?;
        Ok(<[u8; N]>::try_from(s).unwrap())
    }

    /// Reads one byte and advances.
    pub fn u8(&mut self) -> Result<u8, ParserError> {
        let b = self.bytes(1)?;
        Ok(b[0])
    }

    /// Reads one signed byte and advances.
    pub fn i8(&mut self) -> Result<i8, ParserError> {
        let b = self.bytes(1)?;
        Ok(b[0] as i8)
    }

    /// Reads an `i16` (in little-endian order) and advances.
    pub fn i16(&mut self) -> Result<i16, ParserError> {
        Ok(self.copy::<I16<LE>>()?.get())
    }

    /// Reads an `i32` (in little-endian order) and advances.
    pub fn i32(&mut self) -> Result<i32, ParserError> {
        Ok(self.copy::<I32<LE>>()?.get())
    }

    /// Reads an `i64` (in little-endian order) and advances.
    pub fn i64(&mut self) -> Result<i64, ParserError> {
        Ok(self.copy::<I64<LE>>()?.get())
    }

    /// Reads an `u16` (in little-endian order) and advances.
    pub fn u16(&mut self) -> Result<u16, ParserError> {
        Ok(self.copy::<U16<LE>>()?.get())
    }

    /// Reads an `u32` (in little-endian order) and advances.
    pub fn u32(&mut self) -> Result<u32, ParserError> {
        Ok(self.copy::<U32<LE>>()?.get())
    }

    /// Reads an `u64` (in little-endian order) and advances.
    pub fn u64(&mut self) -> Result<u64, ParserError> {
        Ok(self.copy::<U64<LE>>()?.get())
    }

    /// Reads an `u128` (in little-endian order) and advances.
    pub fn u128(&mut self) -> Result<u128, ParserError> {
        Ok(self.copy::<U128<LE>>()?.get())
    }

    /// Reads an `i128` (in little-endian order) and advances.
    pub fn i128(&mut self) -> Result<i128, ParserError> {
        Ok(self.copy::<I128<LE>>()?.get())
    }

    /// Reads an `f32` (in little-endian order) and advances.
    pub fn f32(&mut self) -> Result<f32, ParserError> {
        let bytes: [u8; 4] = self.copy()?;
        Ok(f32::from_le_bytes(bytes))
    }

    /// Reads an `f64` (in little-endian order) and advances.
    pub fn f64(&mut self) -> Result<f64, ParserError> {
        let bytes: [u8; 8] = self.copy()?;
        Ok(f64::from_le_bytes(bytes))
    }

    /// Skips over a NUL-terminated string.
    pub fn skip_strz(&mut self) -> Result<(), ParserError> {
        for i in 0..self.bytes.len() {
            if self.bytes[i] == 0 {
                self.bytes = &self.bytes[i + 1..];
                return Ok(());
            }
        }

        Err(ParserError::new())
    }

    /// Reads a NUL-terminated string, without checking that it is UTF-8 encoded.
    pub fn strz(&mut self) -> Result<&'a BStr, ParserError> {
        for i in 0..self.bytes.len() {
            if self.bytes[i] == 0 {
                let str_bytes = &self.bytes[..i];
                self.bytes = &self.bytes[i + 1..];
                return Ok(BStr::new(str_bytes));
            }
        }

        Err(ParserError::new())
    }

    /// Reads a length-prefixed string, without checking that it is UTF-8 encoded.
    pub fn strt_raw(&mut self) -> Result<&'a BStr, ParserError> {
        let len = self.u8()?;
        let bytes = self.bytes(len as usize)?;
        Ok(BStr::new(bytes))
    }

    /// Reads a length-prefixed string.
    pub fn strt(&mut self) -> Result<&'a str, ParserError> {
        let bytes = self.strt_raw()?;
        if let Ok(s) = core::str::from_utf8(bytes.as_ref()) {
            Ok(s)
        } else {
            Err(ParserError::new())
        }
    }

    /// Parses a 32-bit TypeIndex.
    pub fn type_index(&mut self) -> Result<TypeIndex, ParserError> {
        Ok(TypeIndex(self.u32()?))
    }

    /// Parses a generic number value.
    ///
    /// See Section 4, numeric leaves
    pub fn number(&mut self) -> Result<crate::types::number::Number<'a>, ParserError> {
        self.parse()
    }
}

/// A parser that can return mutable references to the data that it parses.
///
/// Most of the methods defined on `ParserMut` are equivalent to the same methods on `Parser`.
pub struct ParserMut<'a> {
    /// The remaining, unparsed data.
    pub bytes: &'a mut [u8],
}

#[allow(missing_docs)]
impl<'a> ParserMut<'a> {
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes }
    }

    pub fn peek_rest(&self) -> &[u8] {
        self.bytes
    }

    pub fn peek_rest_mut(&mut self) -> &mut [u8] {
        self.bytes
    }

    pub fn into_rest(self) -> &'a mut [u8] {
        self.bytes
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn skip(&mut self, n: usize) -> Result<(), ParserError> {
        if n <= self.bytes.len() {
            let b = take(&mut self.bytes);
            self.bytes = &mut b[n..];
            Ok(())
        } else {
            Err(ParserError::new())
        }
    }

    #[inline(always)]
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8], ParserError> {
        if self.bytes.len() < n {
            return Err(ParserError::new());
        }

        let (lo, hi) = take(&mut self.bytes).split_at_mut(n);
        self.bytes = hi;

        Ok(lo)
    }

    #[inline(always)]
    pub fn bytes_mut(&mut self, n: usize) -> Result<&'a mut [u8], ParserError> {
        if self.bytes.len() < n {
            return Err(ParserError::new());
        }

        let (lo, hi) = take(&mut self.bytes).split_at_mut(n);
        self.bytes = hi;

        Ok(lo)
    }

    #[inline(always)]
    pub fn get<T: FromBytes + Unaligned + Immutable + KnownLayout>(
        &mut self,
    ) -> Result<&'a T, ParserError> {
        let bytes = self.bytes(size_of::<T>())?;
        Ok(T::ref_from_bytes(bytes).unwrap())
    }

    #[inline(always)]
    pub fn get_mut<T: FromBytes + IntoBytes + Unaligned + Immutable + KnownLayout>(
        &mut self,
    ) -> Result<&'a mut T, ParserError> {
        let bytes = self.bytes_mut(size_of::<T>())?;
        Ok(T::mut_from_bytes(bytes).unwrap())
    }

    #[inline(always)]
    pub fn copy<T: FromBytes + Unaligned + Immutable>(&mut self) -> Result<T, ParserError> {
        let item = self.bytes(size_of::<T>())?;
        Ok(T::read_from_bytes(item).unwrap())
    }

    pub fn slice_mut<T: FromBytes + IntoBytes + Unaligned>(
        &mut self,
        len: usize,
    ) -> Result<&'a mut [T], ParserError> {
        let d = take(&mut self.bytes);
        if let Ok((lo, hi)) = <[T]>::mut_from_prefix_with_elems(d, len) {
            self.bytes = hi;
            Ok(lo)
        } else {
            Err(ParserError::new())
        }
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], ParserError> {
        let s = self.bytes(N)?;
        Ok(<[u8; N]>::try_from(s).unwrap())
    }

    pub fn u8(&mut self) -> Result<u8, ParserError> {
        let b = self.bytes(1)?;
        Ok(b[0])
    }

    pub fn i8(&mut self) -> Result<i8, ParserError> {
        let b = self.bytes(1)?;
        Ok(b[0] as i8)
    }

    pub fn i16(&mut self) -> Result<i16, ParserError> {
        Ok(self.copy::<I16<LE>>()?.get())
    }

    pub fn i32(&mut self) -> Result<i32, ParserError> {
        Ok(self.copy::<I32<LE>>()?.get())
    }

    pub fn i64(&mut self) -> Result<i64, ParserError> {
        Ok(self.copy::<I64<LE>>()?.get())
    }

    pub fn u16(&mut self) -> Result<u16, ParserError> {
        Ok(self.copy::<U16<LE>>()?.get())
    }

    pub fn u32(&mut self) -> Result<u32, ParserError> {
        Ok(self.copy::<U32<LE>>()?.get())
    }

    pub fn u64(&mut self) -> Result<u64, ParserError> {
        Ok(self.copy::<U64<LE>>()?.get())
    }

    pub fn skip_strz(&mut self) -> Result<(), ParserError> {
        for i in 0..self.bytes.len() {
            if self.bytes[i] == 0 {
                let stolen_bytes = take(&mut self.bytes);
                self.bytes = &mut stolen_bytes[i + 1..];
                return Ok(());
            }
        }

        Err(ParserError::new())
    }

    pub fn strz(&mut self) -> Result<&'a mut BStr, ParserError> {
        for i in 0..self.bytes.len() {
            if self.bytes[i] == 0 {
                let stolen_bytes = take(&mut self.bytes);
                let (str_bytes, hi) = stolen_bytes.split_at_mut(i);
                self.bytes = &mut hi[1..];
                return Ok(str_bytes.as_bstr_mut());
            }
        }

        Err(ParserError::new())
    }

    pub fn type_index(&mut self) -> Result<TypeIndex, ParserError> {
        Ok(TypeIndex(self.u32()?))
    }

    pub fn skip_number(&mut self) -> Result<(), ParserError> {
        let mut p = Parser::new(self.bytes);
        let len_before = p.len();
        let _ = p.number()?;
        let num_len = len_before - p.len();
        self.skip(num_len)?;
        Ok(())
    }
}

/// Zero-sized type for representing parsing errors.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ParserError;

impl ParserError {
    /// Constructor for ParserError, also logs an event. This is useful for setting breakpoints.
    #[cfg_attr(debug_assertions, inline(never))]
    #[cfg_attr(not(debug_assertions), inline(always))]
    pub fn new() -> Self {
        #[cfg(debug_assertions)]
        {
            tracing::debug!("ParserError");
        }
        Self
    }
}

impl Default for ParserError {
    fn default() -> Self {
        Self::new()
    }
}

impl std::error::Error for ParserError {}

impl std::fmt::Display for ParserError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("Parsing error")
    }
}

/// Defines types that can parse from a byte stream
pub trait Parse<'a>
where
    Self: Sized,
{
    /// Parses an instance of `Self` from a `Parser`.
    /// This allows the caller to detect which bytes were not consumed at the end of the input.
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError>;

    /// Parses an instance of `Self` from a byte slice.
    fn parse(bytes: &'a [u8]) -> Result<Self, ParserError> {
        let mut p = Parser::new(bytes);
        Self::from_parser(&mut p)
    }
}
