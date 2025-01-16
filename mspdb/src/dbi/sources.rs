//! DBI Sources Substream

use super::*;
use crate::utils::align_4;
use crate::BStr;
use crate::{diag::Diags, utils::is_aligned_4};
use std::collections::HashMap;

/// The "Sources" substream of the DBI stream. This stream describes the merged set of source
/// files that were the inputs (compilands) of all modules.
///
/// See: <https://llvm.org/docs/PDB/DbiStream.html#id7>
pub struct DbiSourcesSubstream<'a> {
    /// The `module_file_starts` array gives the index within `file_name_offsets` where the file
    /// names for each module begin. That is, `file_name_offsets[module_file_starts[m]]` is the file
    /// name offset for the first file in the set of files for module `m`.
    ///
    /// When combined with the `module_file_counts` array, you can easily find the slice within
    /// `file_name_offsets` of files for a specific module.
    ///
    /// The length of this slice is equal to `num_modules`. This slice _does not_ have an extra
    /// entry at the end, so you must use `file_name_offsets.len()` as the end of the per-module
    /// slice for the last entry in this slice.
    module_file_starts: &'a [U16<LE>],

    /// For each module, gives the number of source files that contribute to that module.
    module_file_counts: &'a [U16<LE>],

    /// Contains the concatenated list of file name lists, one list per module. For each module
    /// `m`, the set of items within `file_name_offsets` is given by
    /// `file_name_offsets[module_file_starts[m]..][..module_file_counts[m]]`.
    ///
    /// Each item in this list is an offset into `names_buffer` and points to the start of a
    /// NUL-terminated UTF-8 string.
    ///
    /// This array can (and usually does) contain duplicate values. The values are ordered by the
    /// module which referenced a given set of source files. Since many modules will read a shared
    /// set of header files (e.g. `windows.h`), those shared header files will appear many times
    /// in this list.
    ///
    /// The length of `file_name_offsets` is usually higher than the number of _unique_ source files
    /// because many source files (header files) are referenced by more than one module.
    ///
    /// The length of this slice is equal to the sum of the values in the `module_file_counts`.
    /// The on-disk file format stores a field that counts the number of source files, but the field
    /// is only 16-bit, so it can easily overflow on large executables. That is why this
    /// value is computed when the substream is parsed, instead of using the on-disk version.
    file_name_offsets: &'a [U32<LE>],

    /// Contains the file name strings, encoded in UTF-8 and NUL-terminated.
    names_buffer: &'a [u8],
}

impl<'a> DbiSourcesSubstream<'a> {
    /// The number of modules
    pub fn num_modules(&self) -> usize {
        self.module_file_starts.len()
    }

    /// Provides access to the file name offsets slice. Each value is a file name offset, and can
    /// be used with `get_source_name_at()`.
    pub fn file_name_offsets(&self) -> &'a [U32<LE>] {
        self.file_name_offsets
    }

    /// Parses the file info substream.
    ///
    /// This does not parse or validate every part of the substream. It only parses enough to find
    /// the module indices and file names.
    pub fn parse(substream_data: &'a [u8]) -> anyhow::Result<Self> {
        let mut p = Parser::new(substream_data);
        let num_modules = p.u16()? as usize;

        // In theory this is supposed to contain the number of source files for which this substream
        // contains information. But that would present a problem in that the width of this field
        // being 16-bits would prevent one from having more than 64K source files in a program. In
        // early versions of the file format, this seems to have been the case. In order to support
        // more than this, this field of the is simply ignored, and computed dynamically by summing
        // up the values of the ModFileCounts array (discussed below).
        //
        // In short, this value should be ignored. However, we still have to read the value in
        // order to parse the header correctly.
        let _obsolete_num_source_files = p.u16()? as usize;

        let module_file_starts: &[U16<LE>] = p.slice(num_modules)?;

        // An array of num_modules integers, each one containing the number of source files which
        // contribute to the module at the specified index. While each individual module is limited
        // to 64K contributing source files, the union of all modules' source files may be greater
        // than 64K. The real number of source files is thus computed by summing this array.
        //
        // Note that summing this array does not give the number of _unique source files_, only the
        // total number of source file contributions to modules.
        let module_file_counts: &[U16<LE>] = p.slice(num_modules)?;

        let num_file_offsets = module_file_counts.iter().map(|c| c.get() as usize).sum();

        // At this point, we could scan module_file_starts + module_file_counts and validate that
        // no entry exceeds num_file_offsets.

        let file_name_offsets = p.slice(num_file_offsets)?;
        let names_buffer = p.into_rest();

        Ok(Self {
            module_file_starts,
            module_file_counts,
            file_name_offsets,
            names_buffer,
        })
    }

    /// Consistency checks for DBI Sources Substream.
    pub fn check(&self, diags: &mut Diags) {
        assert_eq!(self.module_file_starts.len(), self.module_file_counts.len());

        let num_file_offsets = self.file_name_offsets.len();

        for (module_index, (&start, &count)) in self
            .module_file_starts
            .iter()
            .zip(self.module_file_counts.iter())
            .enumerate()
        {
            let start = start.get() as usize;
            let count = count.get() as usize;
            let end = start + count;

            if end > self.file_name_offsets.len() {
                diags.error(format!(
                    "Module {module_index} has invalid values for sources. \
                     module_file_starts[{module_index}] = {start}, \
                     module_file_ends[{module_index} = {end}, which exceeds the maximum \
                     value of {num_file_offsets}"
                ));
            }
        }

        for (i, &offset) in self.file_name_offsets.iter().enumerate() {
            // It might still be invalid, but at least we can check this.
            if offset.get() as usize >= self.names_buffer.len() {
                diags.error(format!(
                    "File name offsets array contains invalid value. Index {i}, value {offset}"
                ));
            }
        }
    }

    /// Given a source file index, returns the source file name.
    pub fn get_source_file_name(&self, source_file_index: usize) -> Result<&'a BStr, ParserError> {
        let offset = self.file_name_offsets[source_file_index].get();
        self.get_source_file_name_at(offset)
    }

    /// Given a file name offset (within `name_buffer`), returns the source file name.
    pub fn get_source_file_name_at(&self, file_name_offset: u32) -> Result<&'a BStr, ParserError> {
        let Some(string_data) = self.names_buffer.get(file_name_offset as usize..) else {
            return Err(ParserError);
        };
        let mut p = Parser::new(string_data);
        let file_name = p.strz()?;
        Ok(file_name)
    }

    /// Caller is expected to validate module_index (against `num_modules()`) before calling
    pub fn name_offsets_for_module(&self, module_index: usize) -> anyhow::Result<&[U32<LE>]> {
        let start = self.module_file_starts[module_index].get() as usize;
        let count = self.module_file_counts[module_index].get() as usize;
        let Some(s) = self.file_name_offsets.get(start..start + count) else {
            bail!("File name offsets for module #{module_index} are invalid.  start: {start}, count: {count}, len available: {}", self.file_name_offsets.len());
        };
        Ok(s)
    }

    /// Iterates source files in the DBI Sources Substream.
    pub fn iter_sources(&self) -> IterSources<'_> {
        IterSources {
            names_buffer: self.names_buffer,
            file_name_offsets: self.file_name_offsets.iter(),
        }
    }

    /// Builds a HashMap that maps from file name offsets to strings.
    pub fn sources_map(&self) -> anyhow::Result<HashMap<u32, &BStr>> {
        let mut unique_offsets: Vec<u32> = self.file_name_offsets.iter().map(|i| i.get()).collect();
        unique_offsets.sort_unstable();
        unique_offsets.dedup();

        let mut map = HashMap::new();
        for &offset in unique_offsets.iter() {
            let name = self.get_source_file_name_at(offset)?;
            map.insert(offset, name);
        }

        Ok(map)
    }
}

/// Iterates source files in the DBI Sources Substream.
pub struct IterSources<'a> {
    names_buffer: &'a [u8],
    file_name_offsets: std::slice::Iter<'a, U32<LE>>,
}

impl<'a> Iterator for IterSources<'a> {
    /// name_offset (in bytes), name
    type Item = (u32, &'a BStr);

    fn next(&mut self) -> Option<Self::Item> {
        let offset = self.file_name_offsets.next()?.get();
        let mut p = Parser::new(self.names_buffer);
        p.skip(offset as usize).ok()?;
        let name = p.strz().ok()?;
        Some((offset, name))
    }
}

#[repr(C)]
#[derive(Clone, Debug, AsBytes, FromBytes, FromZeroes, Unaligned)]
struct SourcesSubstreamHeader {
    num_modules: U16<LE>,
    num_sources: U16<LE>,
}

/// Output of calling `sort_sources`.
pub struct SortSourcesOutput {
    /// The contents of the new DBI Sources Substream. The size can be different (shorter or longer)
    /// than the input.
    pub new_substream_data: Vec<u8>,

    /// Maps old name offsets to new name offsets.
    ///
    /// Contains `(old_name_offset, new_name_offset)`, and is sorted by `old_name_offset`.
    /// Use binary search to do the mapping.
    pub name_offset_map: Vec<(u32, u32)>,

    /// Contains a sequence of permutation vectors, one for each module. The range of values in
    /// each permutation vector is given by `modules_file_permutations[m] .. modules_file_permutations[m + 1]`,
    /// where `m` is a module index.
    ///
    /// The values within each permutation vector are not name offsets; they are simply the order
    /// of sources listed for this module. These indexes are used for reordering
    /// `DEBUG_S_FILE_CHECKSUMS` subsections in C13 Line Data sections.
    ///
    /// This maps "new" to "old" indexes. This gives the output order for the file checksums in a
    /// `DEBUG_S_FILE_CHECKSUMS` subsection.
    ///
    /// Invariant: `modules_file_permutations.len() == num_sources`
    pub modules_file_permutations: Vec<u32>,

    /// A vector of indexes that point within `modules_file_permutations`. Each pair of values
    /// gives the range within `modules_file_permutations` of a permutation vector for one module.
    ///
    /// This vector has an extra value with is the length of `modules_file_permutations`, which is
    /// unlike the `modules_file_starts` field within the DBI Sources Substream itself.
    ///
    /// The module indexes for this array are _new_ module indexes.
    ///
    /// Invariant: `modules_file_starts.len() = num_modules + 1`
    pub modules_file_starts: Vec<u32>,
}

impl SortSourcesOutput {
    /// Gets the file permutation vector for a specific module. The module index given is the
    /// "new" module index (after sorting module records).
    ///
    /// The returned vector maps "new" to "old", unlike most other permutation vectors.
    pub fn module_file_permutation(&self, module_index: usize) -> &[u32] {
        let start = self.modules_file_starts[module_index] as usize;
        let end = self.modules_file_starts[module_index + 1] as usize;
        &self.modules_file_permutations[start..end]
    }
}

/// Reads the Source Files substream of the DBI Stream and sorts its contents.
///
/// `module_order` provides a permutation vector for the modules. This is necessary because the
/// module order is sorted separately (independently) from sorting the Source Files substream.
/// `module_order[i]` gives the _old_ module index for the output `i`.
pub fn sort_sources(
    old_sources_bytes: &[u8],
    modules_new_to_old: Option<&[u32]>,
) -> anyhow::Result<SortSourcesOutput> {
    let old_sources = DbiSourcesSubstream::parse(old_sources_bytes)?;
    let num_modules = old_sources.num_modules();

    if let Some(modules_new_to_old) = modules_new_to_old {
        crate::sort_utils::assert_is_permutation_u32(modules_new_to_old);

        // Invariant: The num_modules value in the DBI Sources Substream matches the number of modules
        // found in the DBI Modules Substream.
        if modules_new_to_old.len() != num_modules {
            bail!("The DBI Sources Substream specifies num_modules = {}, but the caller specifies num_modules = {}",
            num_modules,
            modules_new_to_old.len(),
        );
        }
    }

    // Find the set of unique file names. The file names are identified by a byte offset into
    // the file name table, and we need to remap from old entries to new entries. So we build
    // a mapping table that goes from old names to new names.
    //
    // It is possible that a name offset points into the middle of an existing string, such as
    // pointing to "world" in "hello, world". In practice, no PDB has been found with that pattern.
    // So we copy the entire string, on the assumption that each name offset points to the start
    // of a stream, and no one is playing string suffix games. If a PDB does have a name offset
    // that points into the middle of a record, we will store a little extra string data.

    let old_file_name_offsets: Vec<u32> = {
        let mut old_file_name_offsets: Vec<u32> = old_sources
            .file_name_offsets
            .iter()
            .map(|i| i.get())
            .collect();
        old_file_name_offsets.sort_unstable();
        old_file_name_offsets.dedup();
        old_file_name_offsets
    };

    // Build a vector of the file names and sort them. Keep the byte offset that we used to create
    // each string, because that is the lookup key that will be used when remapping the old string
    // references to the new output.  sorted_file_names contains (byte_offset, string)
    //
    // We are using an unstable sort, which will _not_ give us a deterministic order for the
    // name offset field. That's ok because that name offset is the old name offset, not the new,
    // so it will not become part of our output.

    // (old_name_offset, string)
    let sorted_file_names: Vec<(u32, &BStr)> = {
        let mut sorted_file_names = Vec::with_capacity(old_file_name_offsets.len());
        for &offset in old_file_name_offsets.iter() {
            let file_name = old_sources.get_source_file_name_at(offset)?;
            sorted_file_names.push((offset, file_name));
        }
        sorted_file_names.sort_unstable_by_key(|&(_, name)| name);
        sorted_file_names
    };

    // Find the precise size of the new names_buffer. This loop has to work the same as the
    // loop below, which actually transfers the data. This loop also builds the mapping table from
    // old name offsets to new name offsets.
    //
    // The new names_buffer can be smaller or larger than the input names buffer. It will be smaller
    // if the input contained duplicate strings. It will be larger if the input contained name
    // offsets that pointed into the middle of strings.
    let new_names_buffer_size_unaligned: usize;
    let mut name_offset_map: Vec<(u32, u32)> = Vec::with_capacity(sorted_file_names.len()); // (old_offset, new_offset)
    {
        let mut next_offset: usize = 0;

        for (i, &(old_name_offset, name)) in sorted_file_names.iter().enumerate() {
            // If this name is a duplicate, then do not write the string to the output.
            if i > 0 && sorted_file_names[i - 1].1 == name {
                // found duplicate name in string table
                let dup_new_offset = name_offset_map[i - 1].1;
                name_offset_map.push((old_name_offset, dup_new_offset));
                continue;
            }

            name_offset_map.push((old_name_offset, next_offset as u32));
            next_offset += name.len() + 1;
        }

        name_offset_map.sort_unstable();

        assert_eq!(name_offset_map.len(), sorted_file_names.len());
        new_names_buffer_size_unaligned = next_offset;
    }
    let new_names_buffer_size_aligned = align_4(new_names_buffer_size_unaligned);

    // Now that we have measured the character data, we can find the size and layout of the output
    // substream data. Allocate the buffer for the output substream data and create the slices that
    // point to its pieces.

    let new_substream_data_len = 4      // substream header (num_modules, num_sources)
        + num_modules * 4               // 4 for 2 u16 arrays
        + old_sources.file_name_offsets.len() * 4               // 4 for u32 array
        + new_names_buffer_size_aligned;
    assert!(is_aligned_4(new_substream_data_len));

    let mut new_substream_data: Vec<u8> = vec![0; new_substream_data_len];
    let mut e = ParserMut::new(new_substream_data.as_mut_slice());
    // Write the number of modules. This value should be the same as the input.
    *e.get_mut().unwrap() = SourcesSubstreamHeader {
        num_modules: U16::new(num_modules as u16),
        // Write the num_sources value to the output substream header. We intentionally chop this
        // to just the low 16 bits, for compatibility with what has been observed in PDBs. However,
        // nothing should actually read this value.
        num_sources: U16::new(old_sources.file_name_offsets.len() as u16),
    };
    let new_module_file_starts: &mut [U16<LE>] = e.slice_mut(num_modules).unwrap();
    let new_module_file_counts: &mut [U16<LE>] = e.slice_mut(num_modules).unwrap();
    let new_file_name_offsets: &mut [U32<LE>] =
        e.slice_mut(old_sources.file_name_offsets.len()).unwrap();
    let new_names_buffer: &mut [u8] =
        &mut e.slice_mut(new_names_buffer_size_aligned).unwrap()[..new_names_buffer_size_unaligned];
    assert!(e.is_empty());

    // Copy the character data for the file names into the output buffer.
    // This loop must match the behavior of the loop above, which computes the offsets.
    {
        let mut next_offset: usize = 0;

        for (i, &(_old_name_offset, name)) in sorted_file_names.iter().enumerate() {
            // If this name is a duplicate, then do not write the string to the output.
            if i > 0 && sorted_file_names[i - 1].1 == name {
                // found duplicate name in string table
                continue;
            }

            let name: &[u8] = name;
            new_names_buffer[next_offset..next_offset + name.len()].copy_from_slice(name);
            next_offset += name.len() + 1;
        }

        assert_eq!(next_offset, new_names_buffer_size_unaligned);
    }

    // Next, we need to sort the file name offsets _within each module_. Since the file name offsets
    // were assigned sequentially after sorting the file names, we can just sort each module's
    // file name list by sorting the file name offsets; there is no need to get the character data
    // for the strings.
    //
    // However, there's an extra complication. We must remember the permutation that we used when
    // sorting these lists, because there is another PDB data structure which depends on their
    // order. That is the DEBUG_S_FILE_CHECKSUMS subsection of the C13 Line Data.
    //
    // We handle this by building a vector that contains the per-module permutations. Then, for
    // each module, we sort the permutation for that module, using the file name offsets as the
    // sorting order.
    //
    // This "vector of permutation vectors" becomes part of our output, so that we can use it to
    // correctly rewrite DEBUG_S_FILE_CHECKSUMS sections.

    let mut modules_file_permutations: Vec<u32> =
        Vec::with_capacity(old_sources.file_name_offsets.len());
    let mut modules_file_starts: Vec<u32> = Vec::with_capacity(num_modules + 1);
    let mut next_module_file_start: usize = 0;

    {
        // Maps from old name offsets to new name offsets.
        let map_name_offset = |old_name_offset: u32| -> u32 {
            // We use unwrap() because this lookup should always succeed. We built this table, above,
            // by iterating the same set of values that are the input to this function.
            let i = name_offset_map
                .binary_search_by_key(&old_name_offset, |&(old, _)| old)
                .unwrap();
            name_offset_map[i].1
        };

        // Contains (new_name_offset, old_order) where old_order is the index in the input where we
        // found this name.  We clear and reuse this buffer once for each module.
        let mut this_module_sources: Vec<(u32, u32)> = Vec::new();

        // Iterate the modules in output order.
        for new_module_index in 0..num_modules {
            // If a module mapping was provided then use it to map to the old module index.
            let old_module_index = if let Some(modules_new_to_old) = modules_new_to_old {
                modules_new_to_old[new_module_index] as usize
            } else {
                new_module_index
            };

            let old_names_start = old_sources.module_file_starts[old_module_index].get() as usize;
            let old_names_count = old_sources.module_file_counts[old_module_index].get() as usize;

            let Some(old_names) = old_sources
                .file_name_offsets
                .get(old_names_start..old_names_start + old_names_count)
            else {
                bail!("invalid data in DBI Sources table");
            };

            // We sort the files associated with the current module. This breaks a relationship
            // with another table: the DEBUG_S_FILE_CHECKSUMS table in the C13 Line Data for this
            // module. For that reason, we build a remapping that allows us to repair the
            // DEBUG_S_FILE_CHECKSUMS table.
            //
            // This is necessary because DIA ignores the contents of the `/names` table for sources,
            // and instead uses the DBI Sources Stream.  It uses the index of each FileChecksum
            // record to index (by record, not by byte) into the per-module list of sources in
            // the DBI Sources Substream.
            //
            // We do not eliminate duplicates from the per-module source file list, because so far
            // we have not seen any duplicates in the PDBs that we have checked. Also, de-duping
            // would make the logic that rewrites DEBUG_S_FILE_CHECKSUMS sections slightly scarier.

            debug_assert!(this_module_sources.is_empty());
            this_module_sources.reserve(old_names.len());
            for (i, &old_name) in old_names.iter().enumerate() {
                let new_name_offset = map_name_offset(old_name.get());
                this_module_sources.push((new_name_offset, i as u32));
            }
            this_module_sources.sort_unstable();

            assert!(this_module_sources.len() <= 0xffff);
            new_module_file_counts[new_module_index] = U16::new(this_module_sources.len() as u16);
            new_module_file_starts[new_module_index] = U16::new(next_module_file_start as u16);

            // Write the new name offsets into the output, and build the permutation vector
            // within modules_file_permutations.
            modules_file_starts.push(modules_file_permutations.len() as u32);
            for &(new_name_offset, old_order) in this_module_sources.iter() {
                new_file_name_offsets[next_module_file_start] = U32::new(new_name_offset);
                next_module_file_start += 1;
                modules_file_permutations.push(old_order);
            }

            this_module_sources.clear();
        }

        modules_file_starts.push(modules_file_permutations.len() as u32);

        assert_eq!(next_module_file_start, old_sources.file_name_offsets.len());
        assert_eq!(
            modules_file_permutations.len(),
            old_sources.file_name_offsets.len()
        );
    }

    Ok(SortSourcesOutput {
        new_substream_data,
        name_offset_map,
        modules_file_permutations,
        modules_file_starts,
    })
}

#[cfg(test)]
#[rustfmt::skip]
static TEST_SOURCES_DATA: &[u8] = &[
    /* 0x0000 */ 4, 0,                     // num_modules = 4
    /* 0x0004 */ 0xee, 0xee,               // obsolete num_sources (bogus)
    /* 0x0008 */ 0, 0, 2, 0, 3, 0, 3, 0,   // module_file_starts = [0, 2, 3, 3]
    /* 0x0010 */ 2, 0, 1, 0, 0, 0, 3, 0,   // module_file_counts = [2, 1, 0, 3] sum = 6

    /* 0x0018 */                           // file_offsets, len = 6 items, 24 bytes
    /* 0x0018 */ 0x00, 0, 0, 0,            // module 0, file_offsets[0] = 0x00, points to "foo.c",
    /* 0x0018 */ 0x14, 0, 0, 0,            // module 0, file_offsets[1] = 0x14, points to "windows.h"
    /* 0x0018 */ 0x06, 0, 0, 0,            // module 1, file_offsets[2] = 0x06, points to "bar.rs"
    /* 0x0018 */ 0x00, 0, 0, 0,            // module 3, file_offsets[3] = 0x00, points to "foo.c"
    /* 0x0018 */ 0x14, 0, 0, 0,            // module 3, file_offsets[4] = 0x14, points to "windows.h"
    /* 0x0018 */ 0x0d, 0, 0, 0,            // module 3, file_offsets[5] = 0x0d, points to "main.c"

    // names_buffer; contains (at relative offsets):
    //      name offset 0x0000 : "foo.c"
    //      name offset 0x0006 : "bar.rs"
    //      name offset 0x000d : "main.c"
    //      name offset 0x0014 : "windows.h"
    /* 0x0030 */                                // names_buffer
    /* 0x0030 */ b'f', b'o', b'o', b'.',
    /* 0x0034 */ b'c', 0,    b'b', b'a',
    /* 0x0038 */ b'r', b'.', b'r', b's',
    /* 0x003c */ 0,    b'm', b'a', b'i',
    /* 0x0040 */ b'n', b'.', b'c', 0,
    /* 0x0044 */ b'w', b'i', b'n', b'd',
    /* 0x0048 */ b'o', b'w', b's', b'.',
    /* 0x004c */ b'h', 0,    0,    0,

    /* 0x0050 : end */
];

#[test]
fn basic_parse() {
    let s = DbiSourcesSubstream::parse(TEST_SOURCES_DATA).unwrap();
    assert_eq!(s.num_modules(), 4);

    assert_eq!(s.file_name_offsets.len(), 6);

    let module_file_starts: Vec<u16> = s.module_file_starts.iter().map(|x| x.get()).collect();
    assert_eq!(&module_file_starts, &[0, 2, 3, 3]);

    let module_file_counts: Vec<u16> = s.module_file_counts.iter().map(|x| x.get()).collect();
    assert_eq!(&module_file_counts, &[2, 1, 0, 3]);

    let file_name_offsets: Vec<u32> = s.file_name_offsets.iter().map(|x| x.get()).collect();
    assert_eq!(&file_name_offsets, &[0x00, 0x14, 0x06, 0x00, 0x14, 0x0d]);

    // Read the file names. Remember that there are duplicates in this list.
    assert_eq!(s.get_source_file_name(0).unwrap(), "foo.c");
    assert_eq!(s.get_source_file_name(1).unwrap(), "windows.h");
    assert_eq!(s.get_source_file_name(2).unwrap(), "bar.rs");
    assert_eq!(s.get_source_file_name(3).unwrap(), "foo.c");
    assert_eq!(s.get_source_file_name(4).unwrap(), "windows.h");
    assert_eq!(s.get_source_file_name(5).unwrap(), "main.c");

    let modsrcs0 = s.name_offsets_for_module(0).unwrap();
    assert_eq!(modsrcs0.len(), 2);
    assert_eq!(modsrcs0[0].get(), 0);
    assert_eq!(modsrcs0[1].get(), 0x14);

    // Test bounds check on get_source_file_name_at()
    assert!(s.get_source_file_name_at(0xeeee).is_err());
}

#[test]
fn basic_check() {
    let s = DbiSourcesSubstream::parse(TEST_SOURCES_DATA).unwrap();
    let mut diags = Diags::new();
    s.check(&mut diags);
    assert!(!diags.has_errors());
}

#[test]
fn test_iter_sources() {
    use bstr::ByteSlice;

    let s = DbiSourcesSubstream::parse(TEST_SOURCES_DATA).unwrap();

    let sources: Vec<(u32, &str)> = s
        .iter_sources()
        .map(|(i, s)| (i, s.to_str().unwrap()))
        .collect();

    assert_eq!(
        &sources,
        &[
            (0x00, "foo.c"),
            (0x14, "windows.h"),
            (0x06, "bar.rs"),
            (0x00, "foo.c"),
            (0x14, "windows.h"),
            (0x0d, "main.c"),
        ]
    );
}

#[test]
fn test_sources_map() {
    let s = DbiSourcesSubstream::parse(TEST_SOURCES_DATA).unwrap();
    let map = s.sources_map().unwrap();
    assert_eq!(map.len(), 4); // 4 unique file names
    assert_eq!(*map.get(&0x00).unwrap(), "foo.c");
    assert_eq!(*map.get(&0x06).unwrap(), "bar.rs");
    assert_eq!(*map.get(&0x0d).unwrap(), "main.c");
    assert_eq!(*map.get(&0x14).unwrap(), "windows.h");
}

#[test]
fn test_sort_sources() {
    // This permutation is arbitrary (for the purpose of this test).
    // module_order is new --> old, so:
    //      new 0 from old 3
    //      new 1 from old 0
    //      new 2 from old 1
    //      new 3 from old 2
    let module_order: &[u32] = &[3, 0, 1, 2];

    let sorted = sort_sources(TEST_SOURCES_DATA, Some(module_order)).unwrap();

    // Parse the sorted sources.
    let ss = DbiSourcesSubstream::parse(sorted.new_substream_data.as_slice()).unwrap();
    assert_eq!(ss.num_modules(), 4); // Still have same number of modules

    // Check that the names are now in the sorted order that we expect.
    // Old order:
    //      0x00 : "foo.c"
    //      0x06 : "bar.rs"
    //      0x0d : "main.c"
    //      0x14 : "windows.h"
    // New order:
    //      0x00 : "bar.rs"
    //      0x07 : "foo.c"
    //      0x0d : "main.c"
    //      0x14 : "windows.h"
    assert_eq!(ss.get_source_file_name_at(0x00).unwrap(), "bar.rs");
    assert_eq!(ss.get_source_file_name_at(0x07).unwrap(), "foo.c");
    assert_eq!(ss.get_source_file_name_at(0x0d).unwrap(), "main.c");
    assert_eq!(ss.get_source_file_name_at(0x14).unwrap(), "windows.h");

    // new module 0 was old module 3
    let names0 = ss.name_offsets_for_module(0).unwrap();
    assert_eq!(names0.len(), 3);
    assert_eq!(
        ss.get_source_file_name_at(names0[0].get()).unwrap(),
        "foo.c"
    );
    assert_eq!(
        ss.get_source_file_name_at(names0[1].get()).unwrap(),
        "main.c"
    );
    assert_eq!(
        ss.get_source_file_name_at(names0[2].get()).unwrap(),
        "windows.h"
    );
}
