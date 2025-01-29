//! Provides access to the DBI Stream (Debug Information).
//!
//! The DBI Stream is a central data structure of the PDB. It contains many vital fields, and
//! points to other streams that contain other important information. The DBI is stream 3.
//!
//! Briefly, the DBI contains these substreams:
//!
//! * Modules: This lists the modules (compilands / translation units) that compose an executable.
//!   Each Module Info structure contains many important fields, including the stream number for
//!   a Module Stream.
//!
//! * Section Contributions Substream
//!
//! * Section Map Substream
//!
//! * Sources Substream: This lists the source files that were inputs to all of the translation units.
//!
//! * Type Server Map Substream
//!
//! * Optional Debug Header Substream
//!
//! * Edit-and-Continue Substream
//!
//! The `Dbi` stream holds section contributions and the list of modules (compilands).
//!
//! * <https://llvm.org/docs/PDB/DbiStream.html>
//! * <https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/langapi/include/pdb.h#L860>

use crate::parser::{Parser, ParserError, ParserMut};
use crate::Container;
use crate::{get_or_init_err, Stream};
use crate::{StreamIndexIsNilError, StreamIndexU16};
use anyhow::{bail, Result};
use std::mem::size_of;
use std::ops::Range;
use sync_file::ReadAt;
use tracing::{error, warn};
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, I32, LE, U16, U32};

#[cfg(doc)]
use crate::Pdb;

pub mod modules;
pub mod optional_dbg;
pub mod section_contrib;
pub mod section_map;
pub mod sources;

pub use modules::*;
#[doc(inline)]
pub use section_contrib::*;
#[doc(inline)]
pub use sources::*;

/// The header of the DBI (Debug Information) stream.
#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned, Debug, Clone)]
#[allow(missing_docs)]
pub struct DbiStreamHeader {
    /// Always -1
    pub signature: I32<LE>,

    /// One of the `DBI_STREAM_VERSION_*` values; typically, `DBI_STREAM_VERSION_V110`.
    pub version: U32<LE>,

    /// The number of times this PDB has been modified. The value is set to 1 when a PDB is
    /// first created. This value must match the same field within the PE header.
    pub age: U32<LE>,

    /// The index of the Global Symbol Index, which contains a name-to-symbol lookup table for
    /// global symbols. The symbol records are not stored in this stream; they are stored in the
    /// Global Symbol Stream.
    pub global_symbol_index_stream: StreamIndexU16,

    pub build_number: U16<LE>,

    /// The index of the stream that contains the Public Symbol Index (GSI). This contains a
    /// name-to-symbol map and an address-to-symbol map. See [`crate::globals::gsi`].
    pub public_symbol_index_stream: StreamIndexU16,

    /// The version of the MSPDB DLL which produced this DBI stream.
    pub pdb_dll_version: U16<LE>,

    /// The stream that contains the Global Symbol Stream. This contains symbol records, which can
    /// be decoded using [`crate::syms::SymIter`].
    pub global_symbol_stream: StreamIndexU16,

    pub pdb_dll_rbld: U16<LE>,

    // Substreams
    pub mod_info_size: I32<LE>,
    pub section_contribution_size: I32<LE>,
    pub section_map_size: I32<LE>,
    pub source_info_size: I32<LE>,
    pub type_server_map_size: I32<LE>,
    /// This field is _not_ a substream size. Not sure what it is.
    pub mfc_type_server_index: U32<LE>,
    pub optional_dbg_header_size: I32<LE>,
    pub edit_and_continue_size: I32<LE>,

    pub flags: U16<LE>,
    pub machine: U16<LE>,
    pub padding: U32<LE>,
}

/// Data for an empty DBI stream
pub static EMPTY_DBI_STREAM_HEADER: [u8; DBI_STREAM_HEADER_LEN] = [
    0xFF, 0xFF, 0xFF, 0xFF, // signature
    0x77, 0x09, 0x31, 0x01, // version
    0x01, 0x00, 0x00, 0x00, // age
    0xFF, 0xFF, // global_stream_index
    0x00, 0x00, // build_number
    0xFF, 0xFF, // public_stream_index
    0x00, 0x00, // pdb_dll_version
    0xFF, 0xFF, // sym_record_stream
    0x00, 0x00, // pdb_dll_rbld
    0x00, 0x00, 0x00, 0x00, // mod_info_size
    0x00, 0x00, 0x00, 0x00, // section_contribution_size
    0x00, 0x00, 0x00, 0x00, // section_map_size
    0x00, 0x00, 0x00, 0x00, // source_info_size
    0x00, 0x00, 0x00, 0x00, // type_server_map_size
    0x00, 0x00, 0x00, 0x00, // mfc_type_server_index
    0x00, 0x00, 0x00, 0x00, // optional_dbg_header_size
    0x00, 0x00, 0x00, 0x00, // edit_and_continue_size
    0x00, 0x00, // flags
    0x00, 0x00, // machine
    0x00, 0x00, 0x00, 0x00, // padding
];

#[test]
fn test_parse_empty_dbi_stream_header() {
    let h = <DbiStreamHeader as FromBytes>::read_from(EMPTY_DBI_STREAM_HEADER.as_slice()).unwrap();
    assert!(h.global_symbol_index_stream.get().is_none());
}

impl DbiStreamHeader {
    /// Gets the stream index for the Global Symbol Stream.
    pub fn sym_record_stream(&self) -> Result<u32, StreamIndexIsNilError> {
        self.global_symbol_stream.get_err()
    }

    /// Gets the stream index for the Public Symbol Index.
    pub fn public_stream_index(&self) -> Result<u32, StreamIndexIsNilError> {
        self.public_symbol_index_stream.get_err()
    }

    /// Gets the stream index for the Global Symbol Index.
    pub fn global_stream_index(&self) -> Result<u32, StreamIndexIsNilError> {
        self.global_symbol_index_stream.get_err()
    }

    /// Byte range of the Modules substream.
    pub fn modules_range(&self) -> anyhow::Result<Range<usize>> {
        let start = DBI_STREAM_HEADER_LEN;
        let size = self.mod_info_size.get() as usize;
        Ok(start..start + size)
    }

    /// Byte range of the Modules substream.
    pub fn sources_range(&self) -> anyhow::Result<Range<usize>> {
        let start = DBI_STREAM_HEADER_LEN
            + self.mod_info_size.get() as usize
            + self.section_contribution_size.get() as usize
            + self.section_map_size.get() as usize;
        let size = self.source_info_size.get() as usize;
        Ok(start..start + size)
    }
}

static_assertions::const_assert_eq!(size_of::<DbiStreamHeader>(), DBI_STREAM_HEADER_LEN);
const DBI_STREAM_HEADER_LEN: usize = 64;

/// MSVC version 4.1
pub const DBI_STREAM_VERSION_VC41: u32 = 930803;
/// MSVC version 5.0
pub const DBI_STREAM_VERSION_V50: u32 = 19960307;
/// MSVC version 6.0
pub const DBI_STREAM_VERSION_V60: u32 = 19970606;
/// MSVC version 7.0
pub const DBI_STREAM_VERSION_V70: u32 = 19990903;
/// MSVC version 11.0
pub const DBI_STREAM_VERSION_V110: u32 = 20091201;

/// Holds or refers to the DBI stream.
///
/// The `StreamData` type parameter can be any type that can contain `[u8]`.
///
/// This type contains (or refers to) the _entire_ DBI stream, not just the header.
#[derive(Clone)]
pub struct DbiStream<StreamData = Vec<u8>>
where
    StreamData: AsRef<[u8]>,
{
    /// The contents of the stream.
    pub stream_data: StreamData,

    /// The byte ranges of the substreams.
    pub substreams: DbiSubstreamRanges,
}

// The DBI stream contains a fixed number of "substreams". The DBI header specifies the
// length of each substream.  The position of each substream is found by computing the
// sum of all previous substreams (and the header).
macro_rules! dbi_substreams {
    (
        $(
            $name:ident,
            $mut_name:ident,
            $size_field:ident ;
        )*
    ) => {
        /// Contains the byte ranges of the substreams within the DBI stream.
        #[derive(Clone, Debug, Default)]
        pub struct DbiSubstreamRanges {
            $(
                #[doc = concat!("The range of the ", stringify!($name), " substream.")]
                pub $name: Range<usize>,
            )*
        }

        impl<StreamData: AsRef<[u8]>> DbiStream<StreamData> {
            $(
                #[doc = concat!("The unparsed contents of the ", stringify!($name), " substream.")]
                pub fn $name(&self) -> &[u8] {
                    self.substream_data(self.substreams.$name.clone())
                }

                #[doc = concat!("The unparsed contents of the ", stringify!($name), " substream.")]
                pub fn $mut_name(&mut self) -> &mut [u8]
                where
                    StreamData: AsMut<[u8]>,
                {
                    self.substream_data_mut(self.substreams.$name.clone())
                }

            )*
        }

        impl DbiSubstreamRanges {
            pub(crate) fn from_sizes(sizes: &DbiStreamHeader, stream_len: usize) -> anyhow::Result<Self> {
                let mut pos: usize = DBI_STREAM_HEADER_LEN;
                if pos > stream_len {
                    bail!("DBI stream is too short; pos = {}, stream_len = {}", pos, stream_len);
                }

                $(
                    assert!(pos <= stream_len);
                    let size: i32 = sizes.$size_field.get();
                    if size < 0 {
                        bail!("Substream {} length in DBI header is invalid (is negative)", stringify!($size_field));
                    }

                    let len = size as usize;
                    let available = stream_len - pos;
                    if len > available {
                        bail!("Substream {} length in DBI header is invalid. It extends beyond the end of the stream.", stringify!($size_field));
                    }
                    let start = pos;
                    pos += len;

                    let $name = start..pos;
                )*

                if pos < stream_len {
                    warn!(pos, stream_len, "Something is wrong with the code that finds the ranges of substreams. Expected pos to be equal to stream_len.");
                } else if pos > stream_len {
                    error!(pos, stream_len, "Something is very wrong with the DBI header. The sum of the subtream lengths (pos) exceeds the stream len.");
                } else {
                    // Substream sizes look good.
                }

                Ok(Self {
                    $( $name, )*
                })
            }
        }
    }
}

dbi_substreams! {
    // The order of these determines the order of the substream data in the stream.
    modules_bytes, modules_bytes_mut, mod_info_size;
    section_contributions_bytes, section_contributions_bytes_mut, section_contribution_size;
    section_map_bytes, section_map_bytes_mut, section_map_size;
    source_info, source_info_mut, source_info_size;
    type_server_map, type_server_map_mut, type_server_map_size;
    edit_and_continue, edit_and_continue_mut, edit_and_continue_size;
    optional_debug_header_bytes, optional_debug_header_bytes_mut, optional_dbg_header_size;
}

impl<StreamData: AsRef<[u8]>> DbiStream<StreamData> {
    /// Returns the DBI stream header.
    pub fn header(&self) -> &DbiStreamHeader {
        zerocopy::Ref::<&[u8], DbiStreamHeader>::new_from_prefix(self.stream_data.as_ref())
            .unwrap()
            .0
            .into_ref()
    }

    /// Provides mutable access to the DBI stream header.
    pub fn header_mut(&mut self) -> &mut DbiStreamHeader
    where
        StreamData: AsMut<[u8]>,
    {
        zerocopy::Ref::<&mut [u8], DbiStreamHeader>::new_from_prefix(self.stream_data.as_mut())
            .unwrap()
            .0
            .into_mut()
    }

    fn substream_data(&self, range: Range<usize>) -> &[u8] {
        &self.stream_data.as_ref()[range]
    }

    fn substream_data_mut(&mut self, range: Range<usize>) -> &mut [u8]
    where
        StreamData: AsMut<[u8]>,
    {
        &mut self.stream_data.as_mut()[range]
    }

    /// Reads the Module Information substream.
    pub fn modules(&self) -> ModInfoSubstream<&[u8]> {
        ModInfoSubstream {
            substream_data: self.modules_bytes(),
        }
    }

    /// Iterates the Module records in the Module Information Substream.
    pub fn iter_modules(&self) -> IterModuleInfo<'_> {
        IterModuleInfo::new(self.modules_bytes())
    }

    /// Iterates the Module records in the Module Information Substream, with mutable access.
    pub fn iter_modules_mut(&mut self) -> IterModuleInfoMut<'_>
    where
        StreamData: AsMut<[u8]>,
    {
        IterModuleInfoMut::new(self.modules_bytes_mut())
    }

    /// Return a DbiStream over just a a reference
    pub fn as_slice(&self) -> DbiStream<&[u8]> {
        DbiStream {
            stream_data: self.stream_data.as_ref(),
            substreams: self.substreams.clone(),
        }
    }

    /// Read the DBI Stream header and validate it.
    pub fn parse(stream_data: StreamData) -> anyhow::Result<Self> {
        let stream_bytes: &[u8] = stream_data.as_ref();

        if stream_bytes.is_empty() {
            return Ok(Self {
                substreams: Default::default(),
                stream_data,
            });
        }

        let mut p = Parser::new(stream_bytes);
        let dbi_header: &DbiStreamHeader = p.get()?;

        let substreams = DbiSubstreamRanges::from_sizes(dbi_header, stream_bytes.len())?;

        // We just computed the ranges for each of the substreams, and we verified that the end of
        // the substreams is equal to the size of the entire stream. That implicitly validates all
        // of the range checks for the substreams, so we don't need explicit / verbose checks.
        // We can simply use normal range indexing.

        Ok(Self {
            stream_data,
            substreams,
        })
    }

    /// Parses the DBI Sources Substream section.
    pub fn sources(&self) -> anyhow::Result<sources::DbiSourcesSubstream<'_>> {
        DbiSourcesSubstream::parse(self.source_info())
    }

    /// Parses the header of the Section Contributions Substream and returns an object which can
    /// query it.
    pub fn section_contributions(
        &self,
    ) -> anyhow::Result<section_contrib::SectionContributionsSubstream<'_>> {
        let substream_bytes = self.section_contributions_bytes();
        section_contrib::SectionContributionsSubstream::parse(substream_bytes)
    }

    /// Parses the header of the Section Map Substream and returns an object which can query it.
    pub fn section_map(&self) -> anyhow::Result<section_map::SectionMap<'_>> {
        let section_map_bytes = self.section_map_bytes();
        section_map::SectionMap::parse(section_map_bytes)
    }

    /// Parses the Optional Debug Header Substream and returns an object which can query it.
    pub fn optional_debug_header(&self) -> anyhow::Result<optional_dbg::OptionalDebugHeader> {
        optional_dbg::OptionalDebugHeader::parse(self.optional_debug_header_bytes())
    }

    /// Gets a mutable reference to the Optional Debug Header substream.
    pub fn optional_debug_header_mut(&mut self) -> anyhow::Result<&mut [U16<LE>]>
    where
        StreamData: AsMut<[u8]>,
    {
        if self.substreams.optional_debug_header_bytes.is_empty() {
            Ok(&mut [])
        } else {
            let substream_bytes =
                &mut self.stream_data.as_mut()[self.substreams.optional_debug_header_bytes.clone()];
            if let Some(lv) = zerocopy::Ref::<&mut [u8], _>::new_slice_unaligned(substream_bytes) {
                Ok(lv.into_mut_slice())
            } else {
                bail!("The Optional Debug Header substream within the DBI stream is malformed (length is not valid).");
            }
        }
    }
}

/// Reads the header of the DBI stream. This does **not** validate the header.
///
/// This is a free function because we need to use it before constructing an instance of [`Pdb`].
pub fn read_dbi_stream_header<F: ReadAt>(msf: &Container<F>) -> anyhow::Result<DbiStreamHeader> {
    let stream_reader = msf.get_stream_reader(Stream::DBI.into())?;
    if !stream_reader.is_empty() {
        let mut dbi_header = DbiStreamHeader::new_zeroed();
        stream_reader.read_exact_at(dbi_header.as_bytes_mut(), 0)?;
        Ok(dbi_header)
    } else {
        Ok(DbiStreamHeader::read_from(EMPTY_DBI_STREAM_HEADER.as_slice()).unwrap())
    }
}

/// Reads the entire DBI Stream, validates the header, and then returns an object that
/// can be used for further queries of the DBI Stream.
///
/// This is a free function because we need to use it before constructing an instance of [`Pdb`].
pub fn read_dbi_stream<F: ReadAt>(
    container: &Container<F>,
) -> Result<DbiStream<Vec<u8>>, anyhow::Error> {
    let mut dbi_stream_data = container.read_stream_to_vec(Stream::DBI.into())?;
    if dbi_stream_data.is_empty() {
        dbi_stream_data = EMPTY_DBI_STREAM_HEADER.to_vec();
    }

    DbiStream::parse(dbi_stream_data)
}

impl<F: ReadAt> crate::Pdb<F> {
    /// Reads the header of the DBI stream. This does **not** validate the header.
    pub fn read_dbi_stream_header(&self) -> anyhow::Result<DbiStreamHeader> {
        read_dbi_stream_header(&self.container)
    }

    /// Reads the entire DBI Stream, validates the header, and then returns an object that
    /// can be used for further queries of the DBI Stream.
    pub fn read_dbi_stream(&self) -> Result<DbiStream<Vec<u8>>, anyhow::Error> {
        read_dbi_stream(&self.container)
    }

    fn read_dbi_substream(&self, range: Range<usize>) -> anyhow::Result<Vec<u8>> {
        let len = range.len();
        let mut substream_data = vec![0; len];
        let reader = self.container.get_stream_reader(Stream::DBI.into())?;
        reader.read_exact_at(&mut substream_data, range.start as u64)?;
        Ok(substream_data)
    }

    /// Reads the module substream data from the DBI stream.
    ///
    /// This function always reads the data from the file. It does not cache the data.
    pub fn read_modules(&self) -> anyhow::Result<ModInfoSubstream<Vec<u8>>> {
        let substream_data = self.read_dbi_substream(self.dbi_substreams.modules_bytes.clone())?;
        Ok(ModInfoSubstream { substream_data })
    }

    /// Gets access to the DBI Modules Substream. This will read the DBI Modules Substream
    /// on-demand, and will cache it.
    pub fn modules(&self) -> anyhow::Result<&ModInfoSubstream<Vec<u8>>> {
        get_or_init_err(&self.dbi_modules_cell, || self.read_modules())
    }

    /// Reads the DBI Sources Substream. This always reads the data, and does not cache it.
    pub fn read_sources_data(&self) -> Result<Vec<u8>> {
        self.read_dbi_substream(self.dbi_substreams.source_info.clone())
    }

    /// Gets access to the DBI Sources Substream data.
    pub fn sources_data(&self) -> Result<&[u8]> {
        let sources_data = get_or_init_err(&self.dbi_sources_cell, || self.read_sources_data())?;
        Ok(sources_data)
    }

    /// Gets access to the DBI Sources Substream and parses the header.
    pub fn sources(&self) -> Result<sources::DbiSourcesSubstream<'_>> {
        let sources_data = self.sources_data()?;
        sources::DbiSourcesSubstream::parse(sources_data)
    }

    /// Drops the cached DBI Sources Substream data, if any.
    pub fn drop_sources(&mut self) {
        self.dbi_sources_cell = Default::default();
    }

    /// Reads the contents of the DBI Section Contributions Substream. This function never caches
    /// the data; it is always read unconditionally.
    pub fn read_section_contributions(&self) -> Result<Vec<u8>> {
        self.read_dbi_substream(self.dbi_substreams.section_contributions_bytes.clone())
    }
}

/// Reads fields of the DBI Stream and validates them for consistency with the specification.
pub fn validate_dbi_stream(stream_data: &[u8]) -> anyhow::Result<()> {
    let dbi_stream = DbiStream::parse(stream_data)?;

    // For now, the only validation that we do in this function is decoding the ModuleInfo records.
    let num_modules: usize = dbi_stream.modules().iter().count();

    let sources = DbiSourcesSubstream::parse(dbi_stream.source_info())?;
    if sources.num_modules() != num_modules {
        bail!("Number of modules found in Sources substream ({}) does not match number of Module Info structs found in Modules substream ({}).",
            sources.num_modules(),
            num_modules
        );
    }

    Ok(())
}
