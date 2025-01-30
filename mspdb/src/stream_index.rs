use std::fmt::Display;
use zerocopy::{LE, U16};
use zerocopy_derive::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Identifies a stream in a PDB/MSF file.
///
/// This type guards against NIL stream values. The value stored in `Stream` should never be
/// a NIL value (0xFFFF).
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
#[repr(transparent)]
pub struct Stream(u16);

impl Stream {
    // Some streams have a fixed index.

    /// Fixed stream index 0 is the Previous MSF Stream Directory
    pub const OLD_STREAM_DIR: Stream = Stream(0);

    /// Index of the PDB Information Stream. It contains version information and information to
    /// connect this PDB to the executable.
    pub const PDB: Stream = Stream(1);

    /// Index of the Type Information Stream. It contains type records.
    pub const TPI: Stream = Stream(2);

    /// Debug Information Stream (DBI).
    pub const DBI: Stream = Stream(3);

    /// CodeView type records, index of IPI hash stream
    pub const IPI: Stream = Stream(4);

    /// Validates that `index` is non-NIL and converts it to a `Stream` value.
    ///
    /// If `index` is NIL (0xffff), then this returns `None`.
    pub fn new(index: u16) -> Option<Stream> {
        if index == NIL_STREAM_INDEX {
            None
        } else {
            Some(Stream(index))
        }
    }

    /// Returns the value of the stream index.
    pub fn value(self) -> u16 {
        self.0
    }

    /// Returns the value of the stream index, cast to `usize`. Use this when indexing slices.
    pub fn index(self) -> usize {
        debug_assert!(self.0 != NIL_STREAM_INDEX);
        self.0 as usize
    }
}

impl From<Stream> for u32 {
    fn from(value: Stream) -> Self {
        value.value() as u32
    }
}

impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        Display::fmt(&self.0, f)
    }
}

/// A reserved stream index meaning "no stream at all", in `u16`.
pub const NIL_STREAM_INDEX: u16 = 0xffff;

/// Error type for `Stream::try_from` implementations.
#[derive(Clone, Debug)]
pub struct StreamIndexIsNilError;

impl std::error::Error for StreamIndexIsNilError {}

impl Display for StreamIndexIsNilError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("The given stream index is NIL.")
    }
}

#[derive(Clone, Debug)]
pub struct StreamIndexOverflow;

impl std::error::Error for StreamIndexOverflow {}

impl Display for StreamIndexOverflow {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        fmt.write_str("The value is out of range for 16-bit stream indexes.")
    }
}

impl TryFrom<u16> for Stream {
    type Error = StreamIndexIsNilError;

    fn try_from(i: u16) -> Result<Self, Self::Error> {
        if i != NIL_STREAM_INDEX {
            Ok(Self(i))
        } else {
            Err(StreamIndexIsNilError)
        }
    }
}

/// This structure can be embedded directly in structure definitions.
#[derive(
    Copy, Clone, Eq, PartialEq, Debug, IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned,
)]
#[repr(transparent)]
pub struct StreamIndexU16(pub U16<LE>);

impl StreamIndexU16 {
    /// The value of a nil stream index.
    pub const NIL: Self = Self(U16::from_bytes(NIL_STREAM_INDEX.to_le_bytes()));

    /// Checks whether this value is a nil stream index. Returns `Ok` if the value is not a nil
    /// stream index, or `Err` if it is a nil stream index.
    pub fn get(self) -> Option<u32> {
        let s = self.0.get();
        if s != NIL_STREAM_INDEX {
            Some(s as u32)
        } else {
            None
        }
    }

    /// Checks whether this value is a nil stream index. Returns `Ok` if the value is not a nil
    /// stream index, or `Err` if it is a nil stream index.
    pub fn get_err(self) -> Result<u32, StreamIndexIsNilError> {
        let s = self.0.get();
        if s != NIL_STREAM_INDEX {
            Ok(s as u32)
        } else {
            Err(StreamIndexIsNilError)
        }
    }
}

impl TryFrom<u32> for StreamIndexU16 {
    type Error = StreamIndexOverflow;

    fn try_from(s: u32) -> Result<Self, Self::Error> {
        if s < NIL_STREAM_INDEX as u32 {
            Ok(StreamIndexU16(U16::new(s as u16)))
        } else {
            Err(StreamIndexOverflow)
        }
    }
}

impl TryFrom<Option<u32>> for StreamIndexU16 {
    type Error = StreamIndexOverflow;

    fn try_from(s_opt: Option<u32>) -> Result<Self, Self::Error> {
        if let Some(s) = s_opt {
            if s < NIL_STREAM_INDEX as u32 {
                Ok(StreamIndexU16(U16::new(s as u16)))
            } else {
                Err(StreamIndexOverflow)
            }
        } else {
            Ok(Self::NIL)
        }
    }
}
