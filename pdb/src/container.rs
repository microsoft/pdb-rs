//! Provides an abstraction over MSF and MSFZ files.

use super::*;
use std::io::{Read, Seek, SeekFrom};

/// An abstraction over MSF and MSFZ files. Both types of files contain a set of streams.
pub enum Container<F> {
    /// The underlying file is an MSF file.
    Msf(msf::Msf<F>),
    /// The underlying file is an MSFZ file.
    Msfz(msfz::Msfz<F>),
}

impl<F: ReadAt> Container<F> {
    /// Provides direct access to the MSF layer. If this PDB file is using MSFZ instead of MSF,
    /// then this function returns `None`.
    pub fn msf(&self) -> Option<&msf::Msf<F>> {
        match self {
            Container::Msf(msf) => Some(msf),
            _ => None,
        }
    }

    /// Provides direct, mutable access to the MSF layer. If this PDB file is using MSFZ instead of
    /// MSF, then this function returns `None`.
    pub fn msf_mut(&mut self) -> Option<&mut msf::Msf<F>> {
        match self {
            Container::Msf(msf) => Some(msf),
            _ => None,
        }
    }

    /// Provides direct, mutable access to the MSF layer. If this PDB file is using MSFZ instead of
    /// MSF, then this function returns `None`.
    pub fn msf_mut_err(&mut self) -> anyhow::Result<&mut msf::Msf<F>> {
        match self {
            Container::Msf(msf) => Ok(msf),
            _ => bail!("This operation requires a PDB/MSF file. It cannot use a PDB/MSFZ file."),
        }
    }

    /// The total number of streams in this PDB.
    ///
    /// Some streams may be NIL.
    pub fn num_streams(&self) -> u32 {
        match self {
            Self::Msf(m) => m.num_streams(),
            Self::Msfz(m) => m.num_streams(),
        }
    }

    /// Returns an object which can read from a given stream.  The returned object implements
    /// the [`Read`], [`Seek`], and [`ReadAt`] traits.
    pub fn get_stream_reader(&self, stream: u32) -> anyhow::Result<StreamReader<'_, F>> {
        match self {
            Self::Msf(m) => Ok(StreamReader::Msf(m.get_stream_reader(stream)?)),
            Self::Msfz(m) => Ok(StreamReader::Msfz(m.get_stream_reader(stream)?)),
        }
    }

    /// Reads an entire stream to a vector.
    pub fn read_stream_to_vec(&self, stream: u32) -> anyhow::Result<Vec<u8>> {
        match self {
            Self::Msf(m) => m.read_stream_to_vec(stream),
            Self::Msfz(m) => Ok(m.read_stream(stream)?.into_vec()),
        }
    }

    /// Reads an entire stream to a vector.
    ///
    /// If the stream data is stored within a single compressed chunk, then this function returns
    /// a reference to the decompressed stream data.
    pub fn read_stream(&self, stream: u32) -> anyhow::Result<StreamData> {
        match self {
            Self::Msf(m) => Ok(StreamData::Box(m.read_stream_to_box(stream)?)),
            Self::Msfz(m) => m.read_stream(stream),
        }
    }

    /// Reads an entire stream into an existing vector.
    pub fn read_stream_to_vec_mut(
        &self,
        stream: u32,
        stream_data: &mut Vec<u8>,
    ) -> anyhow::Result<()> {
        match self {
            Self::Msf(m) => m.read_stream_to_vec_mut(stream, stream_data),
            Self::Msfz(m) => {
                let src = m.read_stream(stream)?;
                stream_data.clear();
                stream_data.extend_from_slice(&src);
                Ok(())
            }
        }
    }

    /// Gets the length of a given stream, in bytes.
    ///
    /// The `stream` value must be in a valid range of `0..num_streams()`.
    ///
    /// If `stream` is a NIL stream, this function returns 0.
    pub fn stream_len(&self, stream: u32) -> u64 {
        match self {
            Self::Msf(m) => m.stream_size(stream) as u64,
            Self::Msfz(m) => m.stream_size(stream).unwrap_or_default(),
        }
    }

    /// Returns `true` if `stream` is a valid stream index and is not a nil stream.
    pub fn is_stream_valid(&self, stream: u32) -> bool {
        match self {
            Self::Msf(m) => m.is_stream_valid(stream),
            Self::Msfz(m) => m.is_stream_valid(stream),
        }
    }
}

/// Allows reading a stream using the [`Read`], [`Seek`], and [`ReadAt`] traits.
pub enum StreamReader<'a, F> {
    /// A stream stored within an MSF file.
    Msf(msf::StreamReader<'a, F>),
    /// A stream stored within an MSFZ file.
    Msfz(msfz::StreamReader<'a, F>),
}

impl<'a, F: ReadAt> StreamReader<'a, F> {
    /// Tests whether this stream is empty (zero-length)
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Msf(s) => s.is_empty(),
            Self::Msfz(s) => s.is_empty(),
        }
    }

    /// Returns the length in bytes of the stream.
    ///
    /// If the stream is a nil stream, this returns 0.
    pub fn stream_size(&self) -> u64 {
        match self {
            Self::Msf(s) => s.len() as u64,
            Self::Msfz(s) => s.stream_size(),
        }
    }
}

impl<'a, F: ReadAt> ReadAt for StreamReader<'a, F> {
    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        match self {
            Self::Msf(s) => s.read_at(buf, offset),
            Self::Msfz(s) => s.read_at(buf, offset),
        }
    }

    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        match self {
            Self::Msf(s) => s.read_exact_at(buf, offset),
            Self::Msfz(s) => s.read_exact_at(buf, offset),
        }
    }
}

impl<'a, F: ReadAt> Read for StreamReader<'a, F> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Self::Msf(s) => s.read(buf),
            Self::Msfz(s) => s.read(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        match self {
            Self::Msf(s) => s.read_exact(buf),
            Self::Msfz(s) => s.read_exact(buf),
        }
    }
}

impl<'a, F: ReadAt> Seek for StreamReader<'a, F> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        match self {
            Self::Msf(s) => s.seek(pos),
            Self::Msfz(s) => s.seek(pos),
        }
    }
}
