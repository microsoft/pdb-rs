//! Decodes line information found in Module Streams.
//!
//! # References
//! * [/ZH (Hash algorithm for calculation of file checksum in debug info)](https://learn.microsoft.com/en-us/cpp/build/reference/zh?view=msvc-170)

mod checksum;
mod subsection;

pub use checksum::*;
pub use subsection::*;

use crate::names::NameIndex;
use anyhow::{bail, Context};
use ms_codeview::parser::{Parser, ParserError, ParserMut};
use ms_codeview::{HasRestLen, IteratorWithRangesExt};
use std::mem::{size_of, take};
use tracing::{trace, warn};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, LE, U16, U32};

/// Enumerates the kind of subsections found in C13 Line Data.
///
/// See `cvinfo.h`, `DEBUG_S_SUBSECTION_TYPE`.
#[derive(Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct SubsectionKind(pub u32);

macro_rules! subsections {
    ($( $(#[$a:meta])*  $name:ident = $value:expr;)*) => {
        impl SubsectionKind {
            $(
                $(#[$a])*
                #[allow(missing_docs)]
                pub const $name: SubsectionKind = SubsectionKind($value);
            )*
        }

        impl std::fmt::Debug for SubsectionKind {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
                let s: &str = match *self {
                    $( SubsectionKind::$name => stringify!($name), )*
                    _ => return write!(fmt, "??(0x{:x})", self.0),
                };
                fmt.write_str(s)
            }
        }
    }
}

subsections! {
    SYMBOLS = 0xf1;
    /// Contains C13 Line Data
    LINES = 0xf2;
    STRING_TABLE = 0xf3;
    /// Contains file checksums and pointers to file names. For a given module, there should be
    /// at most one `FILE_CHECKSUMS` subsection.
    FILE_CHECKSUMS = 0xf4;

    FRAMEDATA = 0xF5;
    INLINEELINES = 0xF6;
    CROSSSCOPEIMPORTS = 0xF7;
    CROSSSCOPEEXPORTS = 0xF8;

    IL_LINES = 0xF9;
    FUNC_MDTOKEN_MAP = 0xFA;
    TYPE_MDTOKEN_MAP = 0xFB;
    MERGED_ASSEMBLYINPUT = 0xFC;

    COFF_SYMBOL_RVA = 0xFD;
}

/// Enables decoding of the line data stored in a Module Stream. This decodes the "C13 line data"
/// substream.
pub struct LineData<'a> {
    bytes: &'a [u8],
}

impl<'a> LineData<'a> {
    /// Use this to create a new decoder for the C13 line data. Usually, you want to pass the
    /// result of calling `ModiStreamData::c13_line_data_bytes()` to this function.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Iterates subsections
    pub fn subsections(&self) -> SubsectionIter<'a> {
        SubsectionIter::new(self.bytes)
    }

    /// Finds the `FILE_CHECKSUMS` subsection. There should only be one.
    pub fn find_checksums_bytes(&self) -> Option<&'a [u8]> {
        for subsection in self.subsections() {
            if subsection.kind == SubsectionKind::FILE_CHECKSUMS {
                return Some(subsection.data);
            }
        }
        None
    }

    /// Finds the `FILE_CHECKSUMS` subsection. There should only be one.
    pub fn find_checksums(&self) -> Option<FileChecksumsSubsection<'a>> {
        let subsection_bytes = self.find_checksums_bytes()?;
        Some(FileChecksumsSubsection::new(subsection_bytes))
    }

    /// Iterates the `NameIndex` values that appear in this Line Data section.
    ///
    /// This may iterate the same `NameIndex` value more than once.
    pub fn iter_name_index<F>(&self, mut f: F) -> anyhow::Result<()>
    where
        F: FnMut(NameIndex),
    {
        if let Some(checksums) = self.find_checksums() {
            for subsection in self.subsections() {
                match subsection.kind {
                    SubsectionKind::LINES => {
                        let lines_subsection = LinesSubsection::parse(subsection.data)?;
                        for block in lines_subsection.blocks() {
                            let file = checksums.get_file(block.header.file_index.get())?;
                            let ni = file.header.name.get();
                            f(NameIndex(ni));
                        }
                    }
                    _ => {}
                }
            }
        } else {
            for subsection in self.subsections() {
                match subsection.kind {
                    SubsectionKind::LINES => {
                        bail!("This C13 Line Data substream contains LINES subsections, but does not contain a FILE_CHECKSUMS subsection.");
                    }
                    _ => {}
                }
            }
        };

        Ok(())
    }
}

/// Enables decoding of the line data stored in a Module Stream. This decodes the "C13 line data"
/// substream.
pub struct LineDataMut<'a> {
    bytes: &'a mut [u8],
}

impl<'a> LineDataMut<'a> {
    /// Initializes a new `LineDataMut`. This does not validate the contents of the data.
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes }
    }

    /// Iterates subsections, with mutable access.
    pub fn subsections_mut(&mut self) -> SubsectionIterMut<'_> {
        SubsectionIterMut::new(self.bytes)
    }

    /// Iterates through all of the name indexes stored within this Line Data.
    /// Remaps all entries using `f` as the remapping function.
    ///
    /// `NameIndex` values are found in the `FILE_CHECKSUMS` debug subsections. However, it is not
    /// possible to directly enumerate the entries stored within a `FILE_CHECKSUMS` subsection,
    /// because they are not at guaranteed positions. There may be gaps.
    ///
    /// To find the `NameIndex` values within each `FILE_CHECKSUMS` debug subsection, we first scan
    /// the `LINES` subsections that point to them, and use a `HashSet` to avoid modifying the
    /// same `NameIndex` more than once.
    pub fn remap_name_indexes<F>(&mut self, name_remapping: F) -> anyhow::Result<()>
    where
        F: Fn(NameIndex) -> anyhow::Result<NameIndex>,
    {
        for subsection in self.subsections_mut() {
            match subsection.kind {
                SubsectionKind::FILE_CHECKSUMS => {
                    let mut checksums = FileChecksumsSubsectionMut::new(subsection.data);
                    for checksum in checksums.iter_mut() {
                        // This `name_offset` value points into the Names stream (/names).
                        let old_name = NameIndex(checksum.header.name.get());
                        let new_name = name_remapping(old_name)
                            .with_context(|| format!("old_name: {old_name}"))?;
                        checksum.header.name = U32::new(new_name.0);
                    }
                }

                _ => {}
            }
        }

        Ok(())
    }
}

/// Represents one contribution. Each contribution consists of a sequence of variable-length
/// blocks.
///
/// Each `LINES` subsection represents one "contribution", which has a `ContributionHeader`,
/// followed by a sequence of blocks. Each block is a variable-length record.
pub struct LinesSubsection<'a> {
    /// The fixed-size header of the `Lines` subsection.
    pub contribution: &'a Contribution,
    /// Contains a sequence of variable-sized "blocks". Each block specifies a source file
    /// and a set of mappings from instruction offsets to line numbers within that source file.
    pub blocks_data: &'a [u8],
}

impl<'a> LinesSubsection<'a> {
    /// Parses the contribution header and prepares for iteration of blocks.
    pub fn parse(bytes: &'a [u8]) -> Result<Self, ParserError> {
        let mut p = Parser::new(bytes);
        Ok(Self {
            contribution: p.get()?,
            blocks_data: p.into_rest(),
        })
    }

    /// Iterates through the line number blocks.
    pub fn blocks(&self) -> IterBlocks<'a> {
        IterBlocks {
            bytes: self.blocks_data,
            have_columns: self.contribution.have_columns(),
        }
    }
}

/// Represents one contribution. Each contribution consists of a sequence of variable-length
/// blocks.
///
/// Each `LINES` subsection represents one "contribution", which has a `ContributionHeader`,
/// followed by a sequence of blocks. Each block is a variable-length record.
pub struct LinesSubsectionMut<'a> {
    /// The fixed-size header of the `Lines` subsection.
    pub contribution: &'a mut Contribution,
    /// Contains a sequence of variable-sized "blocks". Each block specifies a source file
    /// and a set of mappings from instruction offsets to line numbers within that source file.
    pub blocks_data: &'a mut [u8],
}

impl<'a> LinesSubsectionMut<'a> {
    /// Parses the contribution header and prepares for iteration of blocks.
    pub fn parse(bytes: &'a mut [u8]) -> Result<Self, ParserError> {
        let mut p = ParserMut::new(bytes);
        Ok(Self {
            contribution: p.get_mut()?,
            blocks_data: p.into_rest(),
        })
    }

    /// Iterates through the line number blocks.
    pub fn blocks(&self) -> IterBlocks<'_> {
        IterBlocks {
            bytes: self.blocks_data,
            have_columns: self.contribution.have_columns(),
        }
    }

    /// Iterates through the line number blocks, with mutable access.
    pub fn blocks_mut(&mut self) -> IterBlocksMut<'_> {
        IterBlocksMut {
            bytes: self.blocks_data,
            have_columns: self.contribution.have_columns(),
        }
    }
}

/// Iterator state for `LinesSubsection::blocks`.
pub struct IterBlocks<'a> {
    bytes: &'a [u8],
    have_columns: bool,
}

impl<'a> HasRestLen for IterBlocks<'a> {
    fn rest_len(&self) -> usize {
        self.bytes.len()
    }
}

impl<'a> Iterator for IterBlocks<'a> {
    type Item = Block<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.bytes);
        let Ok(header) = p.get::<BlockHeader>() else {
            warn!("failed to read BlockHeader");
            return None;
        };

        let block_size: usize = header.block_size.get() as usize;
        let Some(data_len) = block_size.checked_sub(size_of::<BlockHeader>()) else {
            warn!("invalid block; block_size is less than size of block header");
            return None;
        };

        trace!(
            file_index = header.file_index.get(),
            num_lines = header.num_lines.get(),
            block_size = header.block_size.get(),
            data_len,
            "block header"
        );

        let Ok(data) = p.bytes(data_len) else {
            warn!(
                needed_bytes = data_len,
                have_bytes = p.len(),
                "invalid block: need more bytes for block contents"
            );
            return None;
        };

        self.bytes = p.into_rest();
        Some(Block {
            header,
            data,
            have_columns: self.have_columns,
        })
    }
}

/// Iterator state for `LinesSubsection::blocks`.
pub struct IterBlocksMut<'a> {
    bytes: &'a mut [u8],
    have_columns: bool,
}

impl<'a> HasRestLen for IterBlocksMut<'a> {
    fn rest_len(&self) -> usize {
        self.bytes.len()
    }
}

impl<'a> Iterator for IterBlocksMut<'a> {
    type Item = BlockMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let mut p = ParserMut::new(take(&mut self.bytes));
        let Ok(header) = p.get_mut::<BlockHeader>() else {
            warn!("failed to read BlockHeader");
            return None;
        };

        let block_size: usize = header.block_size.get() as usize;
        let Some(data_len) = block_size.checked_sub(size_of::<BlockHeader>()) else {
            warn!("invalid block; block_size is less than size of block header");
            return None;
        };

        trace!(
            "block header: file_index = {}, num_lines = {}, block_size = {}, data_len = {}",
            header.file_index.get(),
            header.num_lines.get(),
            header.block_size.get(),
            data_len
        );

        let Ok(data) = p.bytes_mut(data_len) else {
            warn!(
                "invalid block: need {} bytes for block contents, only have {}",
                data_len,
                p.len()
            );
            return None;
        };

        self.bytes = p.into_rest();
        Some(BlockMut {
            header,
            data,
            have_columns: self.have_columns,
        })
    }
}

/// One block of line data. Each block has a header which points to a source file. All of the line
/// locations within the block point to line numbers (and potentially column numbers) within that
/// source file.
pub struct Block<'a> {
    /// Fixed-size header for the block.
    pub header: &'a BlockHeader,
    /// If `true`, then this block has column numbers as well as line numbers.
    pub have_columns: bool,
    /// Contains the encoded line numbers, followed by column numbers. The number of entries is
    /// specified by `header.num_lines`.
    pub data: &'a [u8],
}

impl<'a> Block<'a> {
    /// Gets the line records for this block.
    pub fn lines(&self) -> &'a [LineRecord] {
        let num_lines = self.header.num_lines.get() as usize;
        if let Ok((lines, _)) = <[LineRecord]>::ref_from_prefix_with_elems(self.data, num_lines) {
            lines
        } else {
            warn!("failed to get lines_data for a block; wrong size");
            &[]
        }
    }

    /// Gets the column records for this block, if it has any.
    pub fn columns(&self) -> Option<&'a [ColumnRecord]> {
        if !self.have_columns {
            return None;
        }

        let num_lines = self.header.num_lines.get() as usize;
        let lines_size = num_lines * size_of::<LineRecord>();
        let Some(column_data) = self.data.get(lines_size..) else {
            warn!("failed to get column data for a block; wrong size");
            return None;
        };

        let Ok((columns, _)) = <[ColumnRecord]>::ref_from_prefix_with_elems(column_data, num_lines)
        else {
            warn!("failed to get column data for a block; byte size is wrong");
            return None;
        };

        Some(columns)
    }
}

/// One block of line data. Each block has a header which points to a source file. All of the line
/// locations within the block point to line numbers (and potentially column numbers) within that
/// source file.
pub struct BlockMut<'a> {
    /// Fixed-size header for the block.
    pub header: &'a mut BlockHeader,
    /// If `true`, then this block has column numbers as well as line numbers.
    pub have_columns: bool,
    /// Contains the encoded line numbers, followed by column numbers. The number of entries is
    /// specified by `header.num_lines`.
    pub data: &'a mut [u8],
}

/// A single line record
///
/// See `CV_Line_t` in `cvinfo.h`
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Clone)]
#[repr(C)]
pub struct LineRecord {
    /// The byte offset from the start of this contribution (in the instruction stream, not the
    /// Lines Data) for this line
    pub offset: U32<LE>,

    /// Encodes three bit-fields
    ///
    /// * Bits 0-23 are `line_num_start`. This is the 1-based starting line number within the source
    ///   file of this line record.
    /// * Bits 24-30 are `delta_line_end`. It specifies a value to add to line_num_start to find the
    ///   ending line. If this value is zero, then this line record encodes only a single line, not
    ///   a span of lines.
    /// * Bit 31 is the `statement` bit field. If set to 1, it indicates that this line record describes a statement.
    pub flags: U32<LE>,
}

impl LineRecord {
    /// The line number of this location. This value is 1-based.
    pub fn line_num_start(&self) -> u32 {
        self.flags.get() & 0x00_ff_ff_ff
    }

    /// If non-zero, then this indicates the delta in bytes within the source file from the start
    /// of the source location to the end of the source location.
    pub fn delta_line_end(&self) -> u8 {
        ((self.flags.get() >> 24) & 0x7f) as u8
    }

    /// True if this location points to a statement.
    pub fn statement(&self) -> bool {
        (self.flags.get() >> 31) != 0
    }
}

impl std::fmt::Debug for LineRecord {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(fmt, "+{} L{}", self.offset.get(), self.line_num_start())?;

        let delta_line_end = self.delta_line_end();
        if delta_line_end != 0 {
            write!(fmt, "..+{}", delta_line_end)?;
        }

        if self.statement() {
            write!(fmt, " S")?;
        }

        Ok(())
    }
}

/// A single column record
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct ColumnRecord {
    /// byte offset in a source line
    pub start_offset: U16<LE>,
    /// byte offset in a source line
    pub end_offset: U16<LE>,
}

#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
#[allow(missing_docs)]
pub struct Contribution {
    pub contribution_offset: U32<LE>,
    pub contribution_segment: U16<LE>,
    pub flags: U16<LE>,
    pub contribution_size: U32<LE>,
    // Followed by a sequence of block records. Each block is variable-length and begins with
    // BlockHeader.
}

impl Contribution {
    /// Indicates whether this block (contribution) also has column numbers.
    pub fn have_columns(&self) -> bool {
        (self.flags.get() & CV_LINES_HAVE_COLUMNS) != 0
    }
}

/// Bit flag for `Contribution::flags` field
pub const CV_LINES_HAVE_COLUMNS: u16 = 0x0001;

#[allow(missing_docs)]
pub struct LinesEntry<'a> {
    pub header: &'a Contribution,
    pub blocks: &'a [u8],
}

/// Header for a variable-length Block record.
///
/// Each block contains a sequence of line records, and optionally column records.
#[derive(IntoBytes, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct BlockHeader {
    /// The byte offset into the file checksums subsection for this file.
    pub file_index: U32<LE>,
    /// The number of `LineRecord` entries that immediately follow this structure. Also, if the
    /// contribution header indicates that the contribution has column values, this specifies
    /// the number of column records that follow the file records.
    pub num_lines: U32<LE>,
    /// Size of the data for this block. This value includes the size of the block header itself,
    /// so the minimum value value is 12.
    pub block_size: U32<LE>,
    // Followed by [u8; block_size - 12]. This data contains [LineRecord; num_lines], optionally
    // followed by [ColumnRecord; num_lines].
}

/// Updates a C13 Line Data substream after NameIndex values have been updated and after
/// file lists for a given module have been rearranged (sorted).
pub fn fixup_c13_line_data(
    file_permutation: &[u32], // maps new-->old for files within a module
    sorted_names: &crate::names::NameIndexMapping,
    c13_line_data: &mut crate::lines::LineDataMut<'_>,
) -> anyhow::Result<()> {
    // maps old --> new, for the file_index values in DEBUG_S_LINES blocks
    let mut checksum_files_mapping: Vec<(u32, u32)> = Vec::with_capacity(file_permutation.len());

    for subsection in c13_line_data.subsections_mut() {
        match subsection.kind {
            SubsectionKind::FILE_CHECKSUMS => {
                let mut checksums = FileChecksumsSubsectionMut::new(subsection.data);
                let mut checksum_ranges = Vec::with_capacity(file_permutation.len());
                for (checksum_range, checksum) in checksums.iter_mut().with_ranges() {
                    // This `name_offset` value points into the Names stream (/names).
                    let old_name = NameIndex(checksum.header.name.get());
                    let new_name = sorted_names
                        .map_old_to_new(old_name)
                        .with_context(|| format!("old_name: {old_name}"))?;
                    checksum.header.name = U32::new(new_name.0);
                    checksum_ranges.push(checksum_range);
                }

                // Next, we are going to rearrange the FileChecksum records within this
                // section, using the permutation that was generated in dbi::sources::sort_sources().

                let mut new_checksums: Vec<u8> = Vec::with_capacity(subsection.data.len());
                for &old_file_index in file_permutation.iter() {
                    let old_range = checksum_ranges[old_file_index as usize].clone();
                    checksum_files_mapping
                        .push((old_range.start as u32, new_checksums.len() as u32));
                    let old_checksum_data = &subsection.data[old_range];
                    new_checksums.extend_from_slice(old_checksum_data);
                }
                checksum_files_mapping.sort_unstable();

                assert_eq!(new_checksums.len(), subsection.data.len());
                subsection.data.copy_from_slice(&new_checksums);
            }

            _ => {}
        }
    }

    // There is a data-flow dependency (on checksum_files_mapping) between these two loops; the
    // loops cannot be combined. The first loop builds checksum_files_mapping; the second loop
    // reads from it.

    for subsection in c13_line_data.subsections_mut() {
        match subsection.kind {
            SubsectionKind::LINES => {
                // We need to rewrite the file_index values within each line block.
                let mut lines = LinesSubsectionMut::parse(subsection.data)?;
                for block in lines.blocks_mut() {
                    let old_file_index = block.header.file_index.get();
                    match checksum_files_mapping
                        .binary_search_by_key(&old_file_index, |&(old, _new)| old)
                    {
                        Ok(i) => {
                            let (_old, new) = checksum_files_mapping[i];
                            block.header.file_index = U32::new(new);
                        }
                        Err(_) => {
                            bail!("DEBUG_S_LINES section contains invalid file index: {old_file_index}");
                        }
                    }
                }
            }

            _ => {}
        }
    }

    Ok(())
}

/// This special line number is part of the "Just My Code" MSVC compiler feature.
///
/// Debuggers that implement the "Just My Code" feature look for this constant when handling
/// "Step Into" requests. If the user asks to "step into" a function call, the debugger will look
/// up the line number of the start of the function. If the line number is `JMC_LINE_NO_STEP_INTO`,
/// then the debugger will _not_ step into the function. Instead, it will step over it.
///
/// This is useful for implementations of standard library functions, like
/// `std::vector<T>::size()`. Often calls to such functions are embedded in complex statements,
/// and the user wants to debug other parts of the complex statement, not the `size()` call.
///
/// # References
/// * <https://learn.microsoft.com/en-us/cpp/build/reference/jmc?view=msvc-170>
/// * <https://learn.microsoft.com/en-us/visualstudio/debugger/just-my-code>
pub const JMC_LINE_NO_STEP_INTO: u32 = 0xf00f00;

/// This special line number is part of the "Just My Code" MSVC compiler feature.
pub const JMC_LINE_FEE_FEE: u32 = 0xfeefee;

/// Returns true if `line` is a number that is used by the "Just My Code" MSVC compiler feature.
pub fn is_jmc_line(line: u32) -> bool {
    line == JMC_LINE_NO_STEP_INTO || line == JMC_LINE_FEE_FEE
}
