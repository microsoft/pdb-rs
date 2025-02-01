//! DBI Sources Substream

use super::*;
use crate::BStr;
use std::collections::HashMap;

/// The "Sources" substream of the DBI stream. This stream describes the merged set of source
/// files that were the inputs (compilands) of all modules.
///
/// See: <https://llvm.org/docs/PDB/DbiStream.html#file-info-substream>
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
