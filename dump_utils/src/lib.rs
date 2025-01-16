//! Utilities for dumping byte slices as hex or possibly-invalid UTF-8 strings.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_else_if)]

use std::fmt::{Debug, Formatter, Write};

/// Dumps a byte slice. The bytes are formatted into rows, with a byte offset displayed on the
/// left, the byte values in hex in the center, and ASCII characters on the right.
pub struct HexDump<'a> {
    bytes: &'a [u8],
    start: usize,
    show_chars: bool,
    show_header: bool,
    row_len: usize,
    style: HexDumpStyle,
}

/// Specifies the style to use for `HexDump`
#[derive(Copy, Clone, Eq, PartialEq, Default, Ord, PartialOrd, Debug)]
pub enum HexDumpStyle {
    /// No `0x` prefix, no comma
    #[default]
    Normal,
    /// Close to compilable Rust syntax
    Rust,
}

impl<'a> HexDump<'a> {
    /// Creates a `HexDump` over a byte slice with the default display settings.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            start: 0,
            show_chars: true,
            show_header: false,
            row_len: 16,
            style: HexDumpStyle::Normal,
        }
    }

    /// Limits the input to a maximum length.
    pub fn max(self, max: usize) -> Self {
        Self {
            bytes: &self.bytes[..self.bytes.len().min(max)],
            ..self
        }
    }

    /// Sets the number of values to show per row.
    pub fn row_len(self, row_len: usize) -> Self {
        assert!(row_len > 0);
        Self { row_len, ..self }
    }

    /// Sets the style to [`HexDumpStyle::Rust`].
    pub fn rust_style(self) -> Self {
        Self {
            style: HexDumpStyle::Rust,
            ..self
        }
    }

    /// Sets the displayed byte offset to a value.
    pub fn at(self, start: usize) -> Self {
        Self { start, ..self }
    }

    /// Specifies whether characters should be displayed or not.
    pub fn chars(self, show_chars: bool) -> Self {
        Self { show_chars, ..self }
    }

    /// Suppresses displaying ASCII characters.
    pub fn no_chars(self) -> Self {
        Self {
            show_chars: false,
            ..self
        }
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

        let rust_style = matches!(self.style, HexDumpStyle::Rust);

        if self.show_header && !rust_style {
            writeln!(
                f,
                "________ : 00 01 02 03 04 05 06 07-08 09 0a 0b 0c 0d 0e 0f"
            )?;
        }

        let row_len = self.row_len;

        let write_offset = |f: &mut Formatter, offset: usize| -> std::fmt::Result {
            match self.style {
                HexDumpStyle::Normal => write!(f, "{offset:08x} : "),
                HexDumpStyle::Rust => write!(f, "/* {offset:08x} */ "),
            }
        };

        let empty_col_size = match self.style {
            HexDumpStyle::Normal => 3, // two hex values and a space
            HexDumpStyle::Rust => 6,   // 0x, two hex values, comma, space
        };

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
                match self.style {
                    HexDumpStyle::Normal => write!(f, " {:02x}", b)?,
                    HexDumpStyle::Rust => write!(f, " 0x{:02x},", b)?,
                }
            }
            for _ in 0..(row_len - row.len()) * empty_col_size {
                f.write_char(' ')?;
            }

            if self.show_chars {
                match self.style {
                    HexDumpStyle::Normal => write!(f, " : ")?,
                    HexDumpStyle::Rust => write!(f, " // ")?,
                }
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

    /// Limits the input to a maximum length.
    pub fn max(self, max: usize) -> Self {
        Self {
            bytes: &self.bytes[..self.bytes.len().min(max)],
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
            write!(fmt, "{:02x}", b)?;
        }

        Ok(())
    }
}

/// Displays a list of integers. If the list contains sequences of contiguous (increasing) values,
/// then these will be displayed using `start-end` notation, rather than displaying each value.
///
/// The user of this type provides a function which indicates whether items are "adjacent" or not.
#[derive(Copy, Clone)]
pub struct DumpRanges<'a, T, F>
where
    T: std::fmt::Debug,
    F: Fn(&T, &T) -> bool,
{
    items: &'a [T],
    is_adjacent: F,
}

impl<'a, T, F> DumpRanges<'a, T, F>
where
    T: std::fmt::Debug,
    F: Fn(&T, &T) -> bool,
{
    /// Creates a new one
    pub fn new(items: &'a [T], is_adjacent: F) -> Self {
        Self { items, is_adjacent }
    }
}

impl<'a, T, F> Debug for DumpRanges<'a, T, F>
where
    T: std::fmt::Debug,
    F: Fn(&T, &T) -> bool,
{
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        let mut iter = self.items;
        let mut need_comma = false;

        while let Some((first, mut rest)) = iter.split_first() {
            if need_comma {
                write!(fmt, ", ")?;
            }
            need_comma = true;

            let mut last = first;
            let mut is_range = false;

            // Are we in the range case, or the singleton case?
            while let Some((next, after_next)) = rest.split_first() {
                if (self.is_adjacent)(last, next) {
                    is_range = true;
                    last = next;
                    rest = after_next;
                } else {
                    break;
                }
            }

            if is_range {
                write!(fmt, "{:?}-{:?}", first, last)?;
            } else {
                // Singleton case.
                write!(fmt, "{:?}", first)?;
            }

            iter = rest;
        }
        Ok(())
    }
}

#[test]
fn test_dump_ranges_f() {
    macro_rules! case {
        ($input:expr, $expected_output:expr) => {
            let input: &[_] = &$input;
            let dump = DumpRanges::new(input, |&a, &b| a + 1 == b);
            let actual_output = format!("{:?}", dump);
            println!("dump_ranges: {:?} --> {:?}", input, actual_output);
            assert_eq!(
                actual_output.as_str(),
                $expected_output,
                "input: {:?}",
                input
            );
        };
    }

    case!([] as [u32; 0], "");
    case!([10u32], "10");
    case!([10u32, 20], "10, 20");
    case!([10u32, 11, 20], "10-11, 20");
    case!([10u32, 12, 13, 14, 15, 20], "10, 12-15, 20");
}

/// Returns the next item in a sequence.
pub trait Successor {
    /// Returns the next item in a sequence.
    fn successor(&self) -> Self;
}

macro_rules! int_successor {
    ($t:ty) => {
        impl Successor for $t {
            fn successor(&self) -> Self {
                *self + 1
            }
        }
    };
}
int_successor!(u8);
int_successor!(u16);
int_successor!(u32);
int_successor!(u64);
int_successor!(u128);
int_successor!(i8);
int_successor!(i16);
int_successor!(i32);
int_successor!(i64);
int_successor!(i128);

/// Shows values with ranges collapsed, using a Successor implementation.
pub struct DumpRangesSucc<'a, T> {
    items: &'a [T],
}

impl<'a, T> DumpRangesSucc<'a, T>
where
    T: std::fmt::Debug,
{
    /// Creates a new one
    pub fn new(items: &'a [T]) -> Self {
        Self { items }
    }
}

impl<'a, T> std::fmt::Debug for DumpRangesSucc<'a, T>
where
    T: std::fmt::Debug + PartialEq + Successor,
{
    fn fmt(&self, fmt: &mut Formatter) -> std::fmt::Result {
        let mut iter = self.items;
        let mut need_comma = false;

        while let Some((first, mut rest)) = iter.split_first() {
            if need_comma {
                write!(fmt, ", ")?;
            }
            need_comma = true;

            let mut expected = first.successor();

            // Are we in the range case, or the singleton case?
            if !rest.is_empty() && rest[0] == expected {
                // Range case.
                let mut last = &rest[0];
                while !rest.is_empty() && rest[0] == expected {
                    expected = expected.successor();
                    last = &rest[0];
                    rest = &rest[1..];
                }

                write!(fmt, "{:?}-{:?}", first, last)?;
            } else {
                // Singleton case.
                write!(fmt, "{:?}", first)?;
            }

            iter = rest;
        }
        Ok(())
    }
}

#[test]
fn test_dump_ranges_succ() {
    macro_rules! case {
        ($input:expr, $expected_output:expr) => {
            let input: &[_] = &$input;
            let dump = DumpRangesSucc::new(input);
            let actual_output = format!("{:?}", dump);
            println!("dump_ranges: {:?} --> {:?}", input, actual_output);
            assert_eq!(
                actual_output.as_str(),
                $expected_output,
                "input: {:?}",
                input
            );
        };
    }

    case!([] as [u32; 0], "");
    case!([10u32], "10");
    case!([10u32, 20], "10, 20");
    case!([10u32, 11, 20], "10-11, 20");
    case!([10u32, 12, 13, 14, 15, 20], "10, 12-15, 20");
}

/// Formats a byte slice using `HexDump` and writes it to a file.
pub fn save_hex_dump(filename: &str, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::io::BufWriter::new(std::fs::File::create(filename)?);
    write!(f, "{:?}", HexDump::new(data))?;
    Ok(())
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
