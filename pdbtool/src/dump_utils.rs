//! Utilities for dumping byte slices as hex or possibly-invalid UTF-8 strings.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_else_if)]
#![allow(clippy::needless_lifetimes)]

use std::fmt::{Debug, Formatter, Write};

/// Dumps a byte slice. The bytes are formatted into rows, with a byte offset displayed on the
/// left, the byte values in hex in the center, and ASCII characters on the right.
pub(crate) struct HexDump<'a> {
    bytes: &'a [u8],
    start: usize,
    show_header: bool,
    row_len: usize,
}

impl<'a> HexDump<'a> {
    /// Creates a `HexDump` over a byte slice with the default display settings.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            start: 0,
            show_header: false,
            row_len: 16,
        }
    }

    /// Limits the input to a maximum length.
    pub fn max(self, max: usize) -> Self {
        Self {
            bytes: &self.bytes[..self.bytes.len().min(max)],
            ..self
        }
    }

    /// Sets the displayed byte offset to a value.
    pub fn at(self, start: usize) -> Self {
        Self { start, ..self }
    }

    /// Specifies whether to display the header line.
    pub fn header(self, show_header: bool) -> Self {
        Self {
            show_header,
            ..self
        }
    }
}

impl<'a> std::fmt::Display for HexDump<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        <Self as Debug>::fmt(self, fmt)
    }
}

impl<'a> Debug for HexDump<'a> {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let mut pos = self.start;

        let mut repeat_start: usize = 0;
        let mut repeat_len: usize = 0;
        let mut repeat_byte: u8 = 0;

        if self.show_header {
            writeln!(
                f,
                "________ : 00 01 02 03 04 05 06 07-08 09 0a 0b 0c 0d 0e 0f"
            )?;
        }

        let row_len = self.row_len;

        let write_offset =
            |f: &mut Formatter, offset: usize| -> std::fmt::Result { write!(f, "{offset:08x} : ") };

        let empty_col_size = 3; // two hex values and a space

        for row in self.bytes.chunks(row_len) {
            if row.len() == row_len {
                if repeat_len != 0 {
                    // Are we extending a repeated set of rows?
                    if row.iter().all(|&b| b == repeat_byte) {
                        repeat_len += row_len;
                        pos += row_len;
                        continue;
                    }
                } else {
                    // Did we find the beginning of a new repeated row?
                    let row0 = row[0];
                    if row.iter().all(|&b| b == row0) {
                        repeat_byte = row0;
                        repeat_start = pos;
                        repeat_len = row_len;
                        pos += row_len;
                        continue;
                    }
                }
            }

            if repeat_len != 0 {
                write_offset(f, repeat_start)?;
                writeln!(f, "... {repeat_byte:02x} repeated ...")?;
                repeat_len = 0;
                repeat_start = 0;
                repeat_byte = 0;
            }

            write_offset(f, pos)?;
            for &b in row.iter() {
                write!(f, " {b:02x}")?;
            }
            for _ in 0..(row_len - row.len()) * empty_col_size {
                f.write_char(' ')?;
            }

            {
                write!(f, " : ")?;
                for &b in row.iter() {
                    let c = if matches!(b, 0x20..=0x7e) {
                        char::from(b)
                    } else {
                        '.'
                    };
                    f.write_char(c)?;
                }
            }

            f.write_char('\n')?;

            pos += row_len;
        }

        if repeat_len != 0 {
            write_offset(f, repeat_start)?;
            writeln!(f, "... {repeat_byte:02x} repeated ...")?;
            write_offset(f, pos)?;
            writeln!(f, "(end)")?;
        }

        Ok(())
    }
}

/// Displays a byte slice in hexadecimal.
pub struct HexStr<'a> {
    bytes: &'a [u8],
    packed: bool,
}

impl<'a> HexStr<'a> {
    /// Creates a new `HexStr` over a slice with the default display settings.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            packed: false,
        }
    }

    /// Specifies that the hex string should be displayed without spaces between the bytes.
    pub fn packed(self) -> Self {
        Self {
            packed: true,
            ..self
        }
    }
}

impl<'a> Debug for HexStr<'a> {
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        for (i, &b) in self.bytes.iter().enumerate() {
            if i != 0 && !self.packed {
                fmt.write_char(' ')?;
            }
            write!(fmt, "{b:02x}")?;
        }

        Ok(())
    }
}

/// Helps display indentation in debug output
#[derive(Copy, Clone)]
pub struct Indent(pub u32);

impl std::fmt::Display for Indent {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        for _ in 0..self.0 {
            fmt.write_char(' ')?;
        }
        Ok(())
    }
}

/// Creates an `Indent`.
pub fn indent(n: u32) -> Indent {
    Indent(n)
}
