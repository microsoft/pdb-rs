//! Reads data from Module Info (`modi`) streams.
//!
//! # References
//! * <https://llvm.org/docs/PDB/ModiStream.html>

pub mod check;

use crate::dbi::ModuleInfoFixed;
use crate::parser::Parser;
use crate::utils::vec::replace_range_copy;
use crate::ReadAt;
use crate::{dbi::ModuleInfo, syms::SymIter};
use anyhow::anyhow;
use log::{debug, warn};
use std::mem::size_of;
use std::ops::Range;
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U32};

/// The Module Symbols substream begins with this header. It is located at stream offset 0 in the
/// Module Stream.
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
pub struct ModuleSymbolsHeader {
    /// Indicates the version of the module symbol stream. Use the `CV_SIGNATURE_*` constants.
    /// The expected value is `CV_SIGNATURE_C13`.
    pub signature: U32<LE>,
}

const MODULE_SYMBOLS_HEADER_LEN: usize = 4;
static_assertions::const_assert_eq!(size_of::<ModuleSymbolsHeader>(), MODULE_SYMBOLS_HEADER_LEN);

/// Actual signature is >64K
pub const CV_SIGNATURE_C6: u32 = 0;
/// First explicit signature
pub const CV_SIGNATURE_C7: u32 = 1;
/// C11 (vc5.x) 32-bit types
pub const CV_SIGNATURE_C11: u32 = 2;
/// C13 (vc7.x) zero terminated names
pub const CV_SIGNATURE_C13: u32 = 4;
/// All signatures from 5 to 64K are reserved
pub const CV_SIGNATURE_RESERVED: u32 = 5;

impl<F: ReadAt> crate::Pdb<F> {
    /// Reads a Module Info stream. The caller must provide a [`ModuleInfo`] structure, which comes
    /// from the DBI Stream.  Use [`crate::dbi::read_dbi_stream`] to enumerate [`ModuleInfo`] values.
    ///
    /// If the Module Info record has a NIL stream, then this function returns `Ok(None)`.
    pub fn read_module_stream(
        &self,
        mod_info: &ModuleInfo,
    ) -> Result<Option<ModiStreamData>, anyhow::Error> {
        let Some(stream) = mod_info.stream() else {
            return Ok(None);
        };

        let stream_data = self.read_stream_to_vec(stream)?;
        Ok(Some(ModiStreamData::new(stream_data, mod_info.header())?))
    }
}

/// Contains (or refers to) the stream data for a Module Info stream.
#[allow(missing_docs)]
pub struct ModiStreamData {
    /// The contents of the stream.
    pub stream_data: Vec<u8>,
    pub sym_byte_size: u32,
    pub c11_byte_size: u32,
    pub c13_byte_size: u32,
    pub global_refs_size: u32,
}

impl ModiStreamData {
    /// Initializes a new `ModiStreamData`. This validates the byte sizes of the substreams,
    /// which are specified in the [`ModuleInfo`] structure, not within the Module Stream itself.
    pub fn new(stream_data: Vec<u8>, module: &ModuleInfoFixed) -> anyhow::Result<Self> {
        let stream_bytes: &[u8] = &stream_data;

        // Validate the byte sizes against the size of the stream data.
        let sym_byte_size = module.sym_byte_size.get();
        let c11_byte_size = module.c11_byte_size.get();
        let c13_byte_size = module.c13_byte_size.get();

        let mut p = Parser::new(stream_bytes);

        p.skip(sym_byte_size as usize).map_err(|_| {
            anyhow!("Module info has a sym_byte_size that exceeds the size of the stream.")
        })?;
        p.skip(c11_byte_size as usize).map_err(|_| {
            anyhow!("Module info has a c11_byte_size that exceeds the size of the stream.")
        })?;
        p.skip(c13_byte_size as usize).map_err(|_| {
            anyhow!("Module info has a c13_byte_size that exceeds the size of the stream.")
        })?;

        let mut global_refs_size;
        if !p.is_empty() {
            global_refs_size = p
                .u32()
                .map_err(|_| anyhow!("Failed to decode global_refs_size. There are {} bytes after the module symbols substream.", p.len()))?;

            if global_refs_size == 0xffff_ffff {
                warn!("Module has global_refs_size = 0xffff_ffff");
                global_refs_size = 0;
            } else {
                p.skip(global_refs_size as usize)
                .map_err(|_| anyhow!("Failed to decode global_refs substream. global_refs_size = 0x{:x}, but there are only 0x{:x} bytes left.",
                global_refs_size,
                p.len()
            ))?;
            }

            if !p.is_empty() {
                debug!("Module stream has extra bytes at end, len = {}", p.len());
            }
        } else {
            global_refs_size = 0;
        }

        Ok(Self {
            stream_data,
            sym_byte_size,
            c11_byte_size,
            c13_byte_size,
            global_refs_size,
        })
    }

    /// Returns an iterator for the symbol data for this module.
    pub fn iter_syms(&self) -> SymIter<'_> {
        SymIter::new(self.sym_data())
    }

    /// Returns a reference to the encoded symbol data for this module.
    ///
    /// This _does not_ include the CodeView signature.
    pub fn sym_data(&self) -> &[u8] {
        self.stream_data
            .get(MODULE_SYMBOLS_HEADER_LEN..self.sym_byte_size as usize)
            .unwrap_or_default()
    }

    /// Returns a mutable reference to the encoded symbol data for this module.
    ///
    /// This _does not_ include the CodeView signature.
    pub fn sym_data_mut(&mut self) -> &mut [u8] {
        self.stream_data
            .get_mut(MODULE_SYMBOLS_HEADER_LEN..self.sym_byte_size as usize)
            .unwrap_or_default()
    }

    /// Returns a reference to the encoded symbol data for this module.
    ///
    /// This _does_ include the CodeView signature.
    pub fn full_sym_data(&self) -> &[u8] {
        &self.stream_data[..self.sym_byte_size as usize]
    }

    /// Returns a mutable reference to the encoded symbol data for this module.
    ///
    /// This _does_ include the CodeView signature.
    pub fn full_sym_data_mut(&mut self) -> &mut [u8] {
        self.stream_data
            .get_mut(..self.sym_byte_size as usize)
            .unwrap_or_default()
    }

    /// Replace the symbol data for this module.  `new_sym_data` includes the CodeView signature.
    pub fn replace_sym_data(&mut self, new_sym_data: &[u8]) {
        if new_sym_data.len() == self.sym_byte_size as usize {
            self.stream_data[..new_sym_data.len()].copy_from_slice(new_sym_data);
        } else {
            replace_range_copy(
                &mut self.stream_data,
                0,
                self.sym_byte_size as usize,
                new_sym_data,
            );
            self.sym_byte_size = new_sym_data.len() as u32;
        }
    }

    /// Returns the byte range of the C13 Line Data within this Module Information Stream.
    pub fn c13_line_data_range(&self) -> Range<usize> {
        if self.c13_byte_size == 0 {
            return 0..0;
        }

        let start = self.sym_byte_size as usize + self.c11_byte_size as usize;
        start..start + self.c13_byte_size as usize
    }

    /// Returns the byte data for the C13 line data.
    pub fn c13_line_data_bytes(&self) -> &[u8] {
        if self.c13_byte_size == 0 {
            return &[];
        }

        // The range has already been validated.
        let stream_data: &[u8] = self.stream_data.as_ref();
        let range = self.c13_line_data_range();
        &stream_data[range]
    }

    /// Returns a mutable reference to the byte data for the C13 Line Data.
    pub fn c13_line_data_bytes_mut(&mut self) -> &mut [u8] {
        if self.c13_byte_size == 0 {
            return &mut [];
        }

        // The range has already been validated.
        let range = self.c13_line_data_range();
        &mut self.stream_data[range]
    }

    /// Returns an object which can decode the C13 Line Data.
    pub fn c13_line_data(&self) -> crate::lines::LineData<'_> {
        crate::lines::LineData::new(self.c13_line_data_bytes())
    }

    /// Returns an object which can decode and modify the C13 Line Data.
    pub fn c13_line_data_mut(&mut self) -> crate::lines::LineDataMut<'_> {
        crate::lines::LineDataMut::new(self.c13_line_data_bytes_mut())
    }

    /// Gets the byte range within the stream data for the global refs
    pub fn global_refs_range(&self) -> Range<usize> {
        if self.global_refs_size == 0 {
            return 0..0;
        }

        // The Global Refs start after the C13 line data.
        // This offset was validated in Self::new().
        // The size_of::<u32>() is for the global_refs_size field itself.
        let global_refs_offset = self.sym_byte_size as usize
            + self.c11_byte_size as usize
            + self.c13_byte_size as usize
            + size_of::<U32<LE>>();
        global_refs_offset..global_refs_offset + self.global_refs_size as usize
    }

    /// Returns a reference to the global refs stored in this Module Stream.
    ///
    /// Each value in the returned slice is a byte offset into the Global Symbol Stream of
    /// a global symbol that this module references.
    pub fn global_refs(&self) -> &[U32<LE>] {
        let range = self.global_refs_range();
        let stream_bytes: &[u8] = self.stream_data.as_ref();
        zerocopy::Ref::new_slice_unaligned(&stream_bytes[range])
            .unwrap()
            .into_slice()
    }

    /// Returns a mutable reference to the global refs stored in this Module Stream.
    pub fn global_refs_mut(&mut self) -> &mut [U32<LE>] {
        let range = self.global_refs_range();
        zerocopy::Ref::new_slice_unaligned(&mut self.stream_data[range])
            .unwrap()
            .into_mut_slice()
    }
}

impl ModiStreamData {
    /// Remove the Global Refs section, if present.
    pub fn truncate_global_refs(&mut self) {
        if self.global_refs_size == 0 {
            return;
        }

        let global_refs_offset =
            self.sym_byte_size as usize + self.c11_byte_size as usize + self.c13_byte_size as usize;

        self.stream_data.truncate(global_refs_offset);
        self.global_refs_size = 0;
    }
}
