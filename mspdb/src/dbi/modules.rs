//! DBI Modules Substream

use super::*;
use crate::utils::iter::{HasRestLen, IteratorWithRangesExt};
use crate::StreamIndexU16;
use bstr::BStr;
use std::mem::take;

/// The header of a Module Info record. Module Info records are stored in the DBI stream.
///
/// See `msvc\src\vctools\PDB\dbi\dbi.h`, `struct MODI_60_Persist`
#[derive(Unaligned, AsBytes, FromBytes, FromZeroes, Clone, Debug)]
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
    ///
    ///
    pub sym_byte_size: U32<LE>,

    /// Specifies the length of the C11 Line Data in a Module Information Stream.
    /// We do not support C11 Line Data, so this value should always be zero.
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

/// Output of [`sort_module_info_records`]
pub struct SortModulesOutput {
    /// Maps `new_module_index --> old_module_index`
    pub new_to_old: Vec<u32>,

    /// Maps `old_module_index --> new_module_index`
    pub old_to_new: Vec<u32>,
}

/// Sorts the Module Info records within a DBI stream.
///
/// `new_stream_index_base` specifies the starting value for the module streams. The `stream_index`
/// field of each `ModuleInfo` is updated to `new_stream_index_base + module_index`. The old
/// stream index for each module is returned in `SortModulesOutput::old_module_streams`. Nil stream
/// indexes are not modified.
pub fn sort_module_info_records(records: &mut [u8]) -> anyhow::Result<SortModulesOutput> {
    assert!(records.len() <= u32::MAX as usize);

    // Verify that the module_index in the first section contribution for each module matches
    // the module index (the order of the Module Info record in the substream data).

    let mut num_modules = 0;
    for (i, module) in IterModuleInfoMut::new(records).enumerate() {
        let contrib_module_index = module.header.section_contrib.module_index.get();
        if contrib_module_index != 0xffff {
            if contrib_module_index as usize != i {
                bail!(
                    "Module Info record #{i} has inconsistent value for the module_index ({contrib_module_index}) \
                     of its first section contribution."
                );
            }
        }

        num_modules += 1;
    }

    // Take another pass through the modules and build a table of (module_name, object_file),
    // which forms our primary key for sorting.
    //
    // Also record the module byte ranges, and create records_starts from it.

    // Contains (module_name, obj_file) of each module.
    // Index is the old module index.
    let mut module_strings: Vec<(&BStr, &BStr)> = Vec::with_capacity(num_modules);
    let mut records_starts: Vec<u32> = Vec::with_capacity(num_modules + 1);

    for (record_range, module) in IterModuleInfo::new(records).with_ranges() {
        records_starts.push(record_range.start as u32);
        module_strings.push((module.module_name, module.obj_file));
    }
    records_starts.push(records.len() as u32);

    // Build the reordering permutation vector and sort the module records.
    let mut reorder_vec: Vec<u32> = identity_permutation_u32(num_modules);
    let mut found_equal: Option<(u32, u32)> = None;
    reorder_vec.sort_unstable_by(|&a, &b| {
        let module_strings_a = &module_strings[a as usize];
        let module_strings_b = &module_strings[b as usize];
        let ord = Ord::cmp(module_strings_a, module_strings_b);
        if ord.is_eq() {
            found_equal = Some((a, b));
        }
        ord
    });

    // Check to see whether there are any modules that have the same (name, obj).
    if let Some((a, b)) = found_equal {
        let (name, obj) = module_strings[a as usize];
        bail!(
            "Two modules exist that have the same name and object file.\n\
               Module indexes {a}, {b}.\n\
               Module name: {name}\n\
               Module object file: {obj}"
        );
    }

    assert_eq!(records_starts.len(), num_modules + 1);

    // Copy the old module records into a temporary buffer, since we are going to rewrite the
    // record data buffer.
    let old_record_data = records.to_vec();
    let get_module_data = |i: usize| -> &[u8] {
        &old_record_data[records_starts[i] as usize..records_starts[i + 1] as usize]
    };

    let mut dst_iter: &mut [u8] = records;
    for &i in reorder_vec.iter() {
        let src = get_module_data(i as usize);
        let (lo, hi) = dst_iter.split_at_mut(src.len());
        lo.copy_from_slice(src);
        dst_iter = hi;
    }
    assert!(dst_iter.is_empty(), "dst_iter.len() = {}", dst_iter.len());

    // Fix the module_index in the first section contribution.
    for (i, module) in IterModuleInfoMut::new(records).enumerate() {
        // In most PDBs, unused1 is zero. In some, we see values that appear to be equal to the
        // module index. Set it to zero.
        module.header.unused1 = U32::new(0);

        // We sometimes see garbage values in unused2.
        module.header.unused2 = U32::new(0);

        module.header.padding = [0; 2];

        let old_module_index = module.header.section_contrib.module_index.get();
        if old_module_index != 0xffff {
            module.header.section_contrib.module_index = U16::new(i as u16);
        }
    }

    let inverse_module_order = invert_permutation_u32(&reorder_vec);

    Ok(SortModulesOutput {
        new_to_old: reorder_vec,
        old_to_new: inverse_module_order,
    })
}

/// Check the DBI Modules Substream for consistency.
pub fn check_module_infos<F: ReadAt>(
    pdb: &crate::Pdb<F>,
    diags: &mut crate::diag::Diags,
) -> Result<()> {
    let modules = pdb.read_modules()?;

    for (module_index, module) in modules.iter().enumerate() {
        check_module_info(pdb, module_index as u16, &module, diags);
    }

    Ok(())
}

/// Check the fields of `ModuleInfo` for consistency.
pub fn check_module_info<F: ReadAt>(
    pdb: &crate::Pdb<F>,
    module_index: u16,
    module: &ModuleInfo,
    diags: &mut crate::diag::Diags,
) {
    // Check that the module index in the first section contribution is the same as the current
    // module index, or is 0xffff, indicating that there is no section contribution at all.
    let contrib_module_index = module.header.section_contrib.module_index.get();
    if !(contrib_module_index == module_index || contrib_module_index == 0xffff) {
        diags.error(format!(
            "Module #{module_index} has incorrect value for section_contrib.module_index"
        ));
    }

    if module.c11_size() % 4 != 0 {
        diags.error(format!(
            "Module #{module_index} has invalid value for c11_byte_size (not a multiple of 4)"
        ));
    }

    if module.c13_size() % 4 != 0 {
        diags.error(format!(
            "Module #{module_index} has invalid value for c13_byte_size (not a multiple of 4)"
        ));
    }

    if module.sym_size() % 4 != 0 {
        diags.error(format!(
            "Module #{module_index} has invalid value for sym_byte_size (not a multiple of 4)"
        ));
    }

    if module.c11_size() != 0 && module.c13_size() != 0 {
        diags.error(format!(
            "Module #{module_index} has both C11 and C13 line data; these should be mutually-exclusive."
        ));
    }

    if let Some(module_stream) = module.stream() {
        // is_stream_valid() checks both the stream index and checks whether the stream is nil.
        if pdb.is_stream_valid(module_stream) {
            let module_stream_len = pdb.stream_len(module_stream);

            // Cast to u64 to avoid overflow
            let sum_of_sizes =
                module.c11_size() as u64 + module.c13_size() as u64 + module.sym_size() as u64;
            if sum_of_sizes > module_stream_len {
                diags.error(format!("Module #{module_index} specifies substream sizes that exceed the size of the module stream."));
            }
        } else {
            diags.error(format!("Module #{module_index} specifies stream #{module_stream}, but that stream is invalid."));
        }
    } else {
        if module.c11_size() != 0 {
            diags.warning(format!(
                "Module #{module_index} has non-zero c11_byte_size, but no stream"
            ));
        }

        if module.c13_size() != 0 {
            diags.warning(format!(
                "Module #{module_index} has non-zero c13_byte_size, but no stream"
            ));
        }

        if module.sym_size() != 0 {
            diags.warning(format!(
                "Module #{module_index} has non-zero sym_byte_size, but no stream"
            ));
        }
    }
}
