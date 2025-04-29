//! DBI Modules Substream

use super::*;
use crate::StreamIndexU16;
use bstr::BStr;
use ms_codeview::HasRestLen;
use std::mem::take;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// The header of a Module Info record. Module Info records are stored in the DBI stream.
///
/// See `dbi.h`, `MODI_60_Persist`
#[derive(Unaligned, IntoBytes, FromBytes, Immutable, KnownLayout, Clone, Debug)]
#[repr(C)]
pub struct ModuleInfoFixed {
    /// This appears to be a module index field, but it is not always set.
    ///
    /// In some PDBs, we see this field being set to the zero-based index of this Module Info record
    /// in the DBI Modules Substream.  In other PDBs, this value is 0.  Set this to 0.
    pub unused1: U32<LE>,

    /// This module's first section contribution.
    pub section_contrib: SectionContribEntry,

    /// Various flags
    ///
    /// * bit 0: set to 1 if this module has been written since DBI opened
    /// * bit 1: set to 1 if this module has EC symbolic information
    /// * bits 2-7: not used
    /// * bits 8-15: index into TSM list for this mods server
    pub flags: U16<LE>,

    /// Stream index of the Module Stream for this module, which contains the symbols and line data
    /// for this module. If this is 0xffff, then this module does not have a module info stream.
    pub stream: StreamIndexU16,

    /// Specifies the size of the symbols substream within the Module Stream.
    pub sym_byte_size: U32<LE>,

    /// Specifies the length of the C11 Line Data in a Module Information Stream.
    /// C11 line data is obsolete and is not supported.
    pub c11_byte_size: U32<LE>,

    /// Specifies the length of the C13 Line Data in a Module Information Stream.
    pub c13_byte_size: U32<LE>,

    /// Number of files contributing to this module.
    pub source_file_count: U16<LE>,

    /// Alignment padding.
    pub padding: [u8; 2],

    /// Do not read. Set to 0 when encoding.
    pub unused2: U32<LE>,

    /// Unknown; possible that this relates to Edit-and-Continue.
    pub source_file_name_index: U32<LE>,

    /// Unknown; possible that this relates to Edit-and-Continue.
    pub pdb_file_path_name_index: U32<LE>,
}

impl ModuleInfoFixed {
    /// Gets the stream for this module, if any. This stream contains the symbol data and C13 Line
    /// Data for the module.
    pub fn stream(&self) -> Option<u32> {
        self.stream.get()
    }
}

/// Holds or refers to the data of a substream within a Module Info record.
#[derive(Clone)]
pub struct ModInfoSubstream<D: AsRef<[u8]>> {
    /// The substream data.
    pub substream_data: D,
}

impl<D: AsRef<[u8]>> ModInfoSubstream<D> {
    /// Iterates the Module Info records contained within the DBI Stream.
    pub fn iter(&self) -> IterModuleInfo<'_> {
        IterModuleInfo {
            rest: self.substream_data.as_ref(),
        }
    }
}

/// An in-memory representation of a Module Info record.
///
/// The `IterModInfo` iterator produces these items.
#[allow(missing_docs)]
pub struct ModuleInfo<'a> {
    pub header: &'a ModuleInfoFixed,
    pub module_name: &'a BStr,
    pub obj_file: &'a BStr,
}

/// A mutable view of a Module Info record.
#[allow(missing_docs)]
pub struct ModuleInfoMut<'a> {
    pub header: &'a mut ModuleInfoFixed,
    pub module_name: &'a BStr,
    pub obj_file: &'a BStr,
}

impl<'a> ModuleInfo<'a> {
    /// The name of the module.
    ///
    /// * For simple object files, this is the same as `file_name()`.
    /// * For DLL import libraries, this is the name of the DLL, e.g. `KernelBase.dll`.
    /// * For static libraries, this is the name (and possibly path) of the object file within the
    ///   static library, not the static library itself.
    pub fn module_name(&self) -> &'a BStr {
        self.module_name
    }

    /// The file name of this module.
    ///
    /// * For individual `*.obj` files that are passed directly to the linker (not in a static
    ///   library), this is the filename.
    /// * For static libraries, this is the `*.lib` file, not the modules within it.
    /// * For DLL import libraries, this is the import library, e.g. `KernelBase.lib`.
    pub fn obj_file(&self) -> &'a BStr {
        self.obj_file
    }

    /// The header of this Module Info record.
    pub fn header(&self) -> &'a ModuleInfoFixed {
        self.header
    }

    /// The stream index of the stream which contains the symbols defined by this module.
    ///
    /// Some modules do not have a symbol stream. In that case, this function will return `None`.
    pub fn stream(&self) -> Option<u32> {
        self.header.stream()
    }

    /// Gets the size in bytes of the C11 Line Data.
    pub fn c11_size(&self) -> u32 {
        self.header.c11_byte_size.get()
    }

    /// Gets the size in bytes of the C13 Line Data.
    pub fn c13_size(&self) -> u32 {
        self.header.c13_byte_size.get()
    }

    /// Gets the size in bytes of the symbol stream for this module. This value includes the size
    /// of the 4-byte symbol stream header.
    pub fn sym_size(&self) -> u32 {
        self.header.sym_byte_size.get()
    }
}

/// Iterates module info records
pub struct IterModuleInfo<'a> {
    rest: &'a [u8],
}

impl<'a> IterModuleInfo<'a> {
    #[allow(missing_docs)]
    pub fn new(data: &'a [u8]) -> Self {
        Self { rest: data }
    }

    /// Returns the data in the iterator that has not yet been parsed.
    pub fn rest(&self) -> &'a [u8] {
        self.rest
    }
}

impl<'a> HasRestLen for IterModuleInfo<'a> {
    fn rest_len(&self) -> usize {
        self.rest.len()
    }
}

impl<'a> Iterator for IterModuleInfo<'a> {
    type Item = ModuleInfo<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.rest);

        let len_before = p.len();
        let header: &ModuleInfoFixed = p.get().ok()?;
        let module_name = p.strz().ok()?;
        let obj_file = p.strz().ok()?;

        // Each ModInfo structures is variable-length. It ends with two NUL-terminated strings.
        // However, the ModInfo structures have an alignment requirement, so if the strings
        // did not land us on an aligned boundary, we have to skip a few bytes to restore
        // alignment.

        // Find the number of bytes that were used for this structure.
        let mod_record_bytes = len_before - p.len();
        let alignment = (4 - (mod_record_bytes & 3)) & 3;
        p.bytes(alignment).ok()?;

        // Save iterator position.
        self.rest = p.into_rest();

        Some(ModuleInfo {
            header,
            module_name,
            obj_file,
        })
    }
}

/// Mutable iterator
pub struct IterModuleInfoMut<'a> {
    rest: &'a mut [u8],
}

impl<'a> IterModuleInfoMut<'a> {
    #[allow(missing_docs)]
    pub fn new(data: &'a mut [u8]) -> Self {
        Self { rest: data }
    }
}

impl<'a> Iterator for IterModuleInfoMut<'a> {
    type Item = ModuleInfoMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        // TODO: note that we steal the byte slice, which means that
        // if anything goes wrong, we'll never put it back.
        let mut p = ParserMut::new(take(&mut self.rest));

        let len_before = p.len();
        let header: &mut ModuleInfoFixed = p.get_mut().ok()?;
        let module_name = p.strz().ok()?;
        let obj_file = p.strz().ok()?;

        // Each ModInfo structures is variable-length. It ends with two NUL-terminated strings.
        // However, the ModInfo structures have an alignment requirement, so if the strings
        // did not land us on an aligned boundary, we have to skip a few bytes to restore
        // alignment.

        // Find the number of bytes that were used for this structure.
        let mod_record_bytes = len_before - p.len();
        let alignment = (4 - (mod_record_bytes & 3)) & 3;
        p.bytes(alignment).ok()?;

        // Save iterator position.
        self.rest = p.into_rest();
        Some(ModuleInfoMut {
            header,
            module_name,
            obj_file,
        })
    }
}
