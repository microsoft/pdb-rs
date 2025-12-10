//! Reads and writes Program Database (PDB) files.
//!
//! # References
//! * <https://llvm.org/docs/PDB/index.html>
//! * <https://github.com/microsoft/microsoft-pdb>

#![forbid(unused_must_use)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::single_match)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::needless_lifetimes)]
#![allow(clippy::needless_late_init)]

pub mod container;
pub mod dbi;
pub mod globals;
pub mod guid;
pub mod hash;
pub mod lines;
pub mod modi;
pub mod taster;
pub use ::uuid::Uuid;
use ms_codeview::arch::Arch;
use ms_codeview::syms::{SymIter, SymKind};
use ms_coff::IMAGE_FILE_MACHINE;
pub use ms_pdb_msf as msf;
pub use ms_pdb_msfz as msfz;
use tracing::warn;
mod coff_groups;
mod embedded_sources;
pub mod names;
pub mod pdbi;
mod stream_index;
pub mod tpi;
pub mod utils;
pub mod writer;

pub use bstr::BStr;
pub use coff_groups::{CoffGroup, CoffGroups};
pub use container::{Container, StreamReader};
pub use ms_codeview::{self as codeview, syms, types};
pub use ms_coff::{self as coff, IMAGE_SECTION_HEADER};
pub use msfz::StreamData;
pub use stream_index::{NIL_STREAM_INDEX, Stream, StreamIndexIsNilError, StreamIndexU16};
pub use sync_file::{RandomAccessFile, ReadAt, WriteAt};

use anyhow::bail;
use globals::gsi::GlobalSymbolIndex;
use globals::gss::GlobalSymbolStream;
use globals::psi::PublicSymbolIndex;
use names::{NameIndex, NamesStream};
use std::cell::OnceCell;
use std::fmt::Debug;
use std::fs::File;
use std::path::Path;
use syms::{Pub, Sym};
use zerocopy::{FromZeros, IntoBytes};

use crate::dbi::ModuleInfo;
use crate::dbi::optional_dbg::OptionalDebugHeaders;

#[cfg(test)]
#[static_init::dynamic]
static INIT_LOGGER: () = {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .with_test_writer()
        .with_file(true)
        .with_line_number(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .without_time()
        .finish();
};

/// Allows reading the contents of a PDB file.
///
/// This type provides read-only access. It does not provide any means to modify a PDB file or
/// to create a new one.
pub struct Pdb<F = sync_file::RandomAccessFile> {
    container: Container<F>,

    /// The header of the DBI Stream. The DBI Stream contains many of the important data structures
    /// for PDB, or has pointers (stream indexes) for them. Nearly all programs that read PDBs
    /// need to read the DBI, so we always load the header.
    dbi_header: dbi::DbiStreamHeader,
    dbi_substreams: dbi::DbiSubstreamRanges,

    pdbi: pdbi::PdbiStream,

    cached: PdbCached,
}

#[derive(Default)]
struct PdbCached {
    names: OnceCell<NamesStream<Vec<u8>>>,

    tpi_header: OnceCell<tpi::CachedTypeStreamHeader>,
    ipi_header: OnceCell<tpi::CachedTypeStreamHeader>,

    /// Cached contents of DBI Modules Substream.
    dbi_modules_cell: OnceCell<dbi::ModInfoSubstream<Vec<u8>>>,
    /// Cached contents of DBI Sources Substream.
    dbi_sources_cell: OnceCell<Vec<u8>>,

    gss: OnceCell<Box<GlobalSymbolStream>>,
    gsi: OnceCell<Box<GlobalSymbolIndex>>,
    psi: OnceCell<Box<PublicSymbolIndex>>,

    coff_groups: OnceCell<CoffGroups>,
    optional_dbg_streams: OnceCell<OptionalDebugHeaders>,
    section_headers: OnceCell<Box<[IMAGE_SECTION_HEADER]>>,
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum AccessMode {
    Read,
    ReadWrite,
}

impl<F: ReadAt> Pdb<F> {
    /// Reads the header of a PDB file and provides access to the streams contained within the
    /// PDB file. Allows read/write access, if using an MSF container format.
    ///
    /// This function reads the MSF File Header, which is the header for the entire file.
    /// It also reads the stream directory, so it knows how to find each of the streams
    /// and the pages of the streams.
    fn from_file_access(file: F, access_mode: AccessMode) -> anyhow::Result<Box<Self>> {
        use crate::taster::{Flavor, what_flavor};

        let Some(flavor) = what_flavor(&file)? else {
            bail!("The file is not a recognized PDB or PDZ format.");
        };

        let container = match (flavor, access_mode) {
            (Flavor::PortablePdb, _) => bail!("Portable PDBs are not supported."),
            (Flavor::Pdb, AccessMode::Read) => Container::Msf(msf::Msf::open_with_file(file)?),
            (Flavor::Pdb, AccessMode::ReadWrite) => {
                Container::Msf(msf::Msf::modify_with_file(file)?)
            }
            (Flavor::Pdz, AccessMode::Read) => Container::Msfz(msfz::Msfz::from_file(file)?),
            (Flavor::Pdz, AccessMode::ReadWrite) => {
                bail!("The MSFZ file format is read-only.")
            }
        };

        let dbi_header = dbi::read_dbi_stream_header(&container)?;
        let stream_len = container.stream_len(Stream::DBI.into());
        let dbi_substreams = if stream_len != 0 {
            dbi::DbiSubstreamRanges::from_sizes(&dbi_header, stream_len as usize)?
        } else {
            dbi::DbiSubstreamRanges::default()
        };

        let pdbi_stream_data = container.read_stream_to_vec(Stream::PDB.into())?;
        let pdbi = pdbi::PdbiStream::parse(&pdbi_stream_data)?;

        Ok(Box::new(Self {
            container,
            dbi_header,
            dbi_substreams,
            pdbi,
            cached: Default::default(),
        }))
    }

    /// Gets access to the PDB Information Stream.
    ///
    /// This loads the PDBI on-demand. The PDBI is usually fairly small.
    pub fn pdbi(&self) -> &pdbi::PdbiStream {
        &self.pdbi
    }

    /// Gets access to the Named Streams table.
    pub fn named_streams(&self) -> &pdbi::NamedStreams {
        &self.pdbi.named_streams
    }

    /// Gets mutable access to the Named Streams table.
    pub fn named_streams_mut(&mut self) -> &mut pdbi::NamedStreams {
        &mut self.pdbi.named_streams
    }

    /// Searches the Named Streams table for a stream with a given name.
    /// Returns `None` if the stream is not found.
    pub fn named_stream(&self, name: &str) -> Option<u32> {
        self.pdbi.named_streams().get(name)
    }

    /// Searches the Named Streams table for a stream with a given name.
    /// Returns an error if the stream is not found.
    pub fn named_stream_err(&self, name: &str) -> anyhow::Result<u32> {
        if let Some(s) = self.pdbi.named_streams().get(name) {
            Ok(s)
        } else {
            anyhow::bail!("There is no stream with the name {:?}.", name);
        }
    }

    /// The header of the DBI Stream.
    pub fn dbi_header(&self) -> &dbi::DbiStreamHeader {
        &self.dbi_header
    }

    /// The byte ranges of the DBI substreams.
    pub fn dbi_substreams(&self) -> &dbi::DbiSubstreamRanges {
        &self.dbi_substreams
    }

    /// Gets the TPI Stream Header.
    ///
    /// This loads the TPI Stream Header on-demand. This does not load the rest of the TPI Stream.
    pub fn tpi_header(&self) -> anyhow::Result<&tpi::CachedTypeStreamHeader> {
        self.tpi_or_ipi_header(Stream::TPI, &self.cached.tpi_header)
    }

    /// Gets the IPI Stream Header.
    ///
    /// This loads the IPI Stream Header on-demand. This does not load the rest of the TPI Stream.
    pub fn ipi_header(&self) -> anyhow::Result<&tpi::CachedTypeStreamHeader> {
        self.tpi_or_ipi_header(Stream::IPI, &self.cached.ipi_header)
    }

    fn tpi_or_ipi_header<'s>(
        &'s self,
        stream: Stream,
        cell: &'s OnceCell<tpi::CachedTypeStreamHeader>,
    ) -> anyhow::Result<&'s tpi::CachedTypeStreamHeader> {
        get_or_init_err(cell, || {
            let r = self.get_stream_reader(stream.into())?;
            let mut header = tpi::TypeStreamHeader::new_zeroed();
            let header_bytes = header.as_mut_bytes();
            let bytes_read = r.read_at(header_bytes, 0)?;
            if bytes_read == 0 {
                // This stream is zero-length.
                return Ok(tpi::CachedTypeStreamHeader { header: None });
            }

            if bytes_read < header_bytes.len() {
                bail!(
                    "The type stream (stream {}) does not contain enough data for a valid header.",
                    stream
                );
            }

            Ok(tpi::CachedTypeStreamHeader {
                header: Some(header),
            })
        })
    }

    /// Gets the Names Stream
    ///
    /// This loads the Names Stream on-demand.
    pub fn names(&self) -> anyhow::Result<&NamesStream<Vec<u8>>> {
        get_or_init_err(&self.cached.names, || {
            if let Some(stream) = self.named_stream(names::NAMES_STREAM_NAME) {
                let stream_data = self.read_stream_to_vec(stream)?;
                Ok(NamesStream::parse(stream_data)?)
            } else {
                let stream_data = names::EMPTY_NAMES_STREAM_DATA.to_vec();
                Ok(NamesStream::parse(stream_data)?)
            }
        })
    }

    /// Gets a name from the Names Stream.
    pub fn get_name(&self, offset: NameIndex) -> anyhow::Result<&BStr> {
        let names = self.names()?;
        names.get_string(offset)
    }

    /// The binding key that associates this PDB with a given PE executable.
    pub fn binding_key(&self) -> BindingKey {
        let pdbi = self.pdbi();
        pdbi.binding_key()
    }

    /// Checks whether this PDB has a given feature enabled.
    pub fn has_feature(&self, feature_code: pdbi::FeatureCode) -> bool {
        self.pdbi.has_feature(feature_code)
    }

    /// Indicates that this PDB was built using the "Mini PDB" option, i.e. `/DEBUG:FASTLINK`.
    pub fn mini_pdb(&self) -> bool {
        self.has_feature(pdbi::FeatureCode::MINI_PDB)
    }

    /// Gets a reference to the Global Symbol Stream (GSS). This loads the GSS on-demand.
    #[inline]
    pub fn gss(&self) -> anyhow::Result<&GlobalSymbolStream> {
        if let Some(gss) = self.cached.gss.get() {
            Ok(gss)
        } else {
            self.gss_slow()
        }
    }

    /// Gets a reference to the Global Symbol Stream (GSS). This loads the GSS on-demand.
    #[inline(never)]
    fn gss_slow(&self) -> anyhow::Result<&GlobalSymbolStream> {
        let box_ref = get_or_init_err(
            &self.cached.gss,
            || -> anyhow::Result<Box<GlobalSymbolStream>> { Ok(Box::new(self.read_gss()?)) },
        )?;
        Ok(box_ref)
    }

    /// If the GSS has been loaded by using the `gss()` function, then this method frees it.
    pub fn gss_drop(&mut self) {
        self.cached.gss.take();
    }

    /// Gets a reference to the Global Symbol Index (GSI). This loads the GSI on-demand.
    #[inline(never)]
    pub fn gsi(&self) -> anyhow::Result<&GlobalSymbolIndex> {
        if let Some(gsi) = self.cached.gsi.get() {
            Ok(gsi)
        } else {
            self.gsi_slow()
        }
    }

    #[inline(never)]
    fn gsi_slow(&self) -> anyhow::Result<&GlobalSymbolIndex> {
        let box_ref = get_or_init_err(
            &self.cached.gsi,
            || -> anyhow::Result<Box<GlobalSymbolIndex>> { Ok(Box::new(self.read_gsi()?)) },
        )?;
        Ok(box_ref)
    }

    /// If the GSI has been loaded by using the `gsi()` function, then this method frees it.
    pub fn gsi_drop(&mut self) {
        self.cached.gsi.take();
    }

    /// Gets a reference to the Public Symbol Index (PSI). This loads the PSI on-demand.
    #[inline]
    pub fn psi(&self) -> anyhow::Result<&PublicSymbolIndex> {
        if let Some(psi) = self.cached.psi.get() {
            Ok(psi)
        } else {
            self.psi_slow()
        }
    }

    #[inline(never)]
    fn psi_slow(&self) -> anyhow::Result<&PublicSymbolIndex> {
        let box_ref = get_or_init_err(
            &self.cached.psi,
            || -> anyhow::Result<Box<PublicSymbolIndex>> { Ok(Box::new(self.read_psi()?)) },
        )?;
        Ok(box_ref)
    }

    /// If the PSI has been loaded by using the `psi()` function, then this method frees it.
    pub fn psi_drop(&mut self) {
        self.cached.psi.take();
    }

    /// Searches for an `S_PUB32` symbol by name.
    pub fn find_public_by_name(&self, name: &BStr) -> anyhow::Result<Option<Pub<'_>>> {
        let gss = self.gss()?;
        let psi = self.psi()?;
        psi.find_symbol_by_name(gss, name)
    }

    /// Searches for a global symbol symbol by name.
    ///
    /// This uses the Global Symbol Index (GSI). This index _does not_ contain `S_PUB32` records.
    /// Use `find_public_by_name` to search for `S_PUB32` records.
    pub fn find_global_by_name(&self, name: &'_ BStr) -> anyhow::Result<Option<Sym<'_>>> {
        let gss = self.gss()?;
        let gsi = self.gsi()?;
        gsi.find_symbol(gss, name)
    }

    /// Writes any changes that have been buffered in memory to disk. However, this does not commit
    /// the changes. It is still necessary to call the `commit()` method.
    ///
    /// The return value indicates whether any changes were written to disk. `Ok(true)` indicates
    /// that some change were written to disk.  `Ok(false)` indicates that there were no buffered
    /// changes and nothing has been written to disk.
    pub fn flush_all(&mut self) -> anyhow::Result<bool>
    where
        F: WriteAt,
    {
        let mut any = false;

        if self.pdbi.named_streams.modified {
            let pdbi_data = self.pdbi.to_bytes()?;
            let mut w = self.msf_mut_err()?.write_stream(Stream::PDB.into())?;
            w.set_contents(&pdbi_data)?;
            self.pdbi.named_streams.modified = false;
            any = true;
        }

        Ok(any)
    }

    /// Gets access to the underlying container.
    pub fn container(&self) -> &Container<F> {
        &self.container
    }

    /// Find the `"* Linker *"` module, which contains the S_COFFGROUP symbols.
    ///
    /// If the PDB does not contain a linker module then this returns `Err`.
    pub fn linker_module(&self) -> anyhow::Result<ModuleInfo<'_>> {
        if let Some(module) = self.linker_module_opt()? {
            Ok(module)
        } else {
            bail!("This PDB does not contain a linker module.");
        }
    }

    /// Find the `"* Linker *"` module, which contains the S_COFFGROUP symbols.
    ///
    /// If the PDB does not contain a linker module then this returns `Ok(None)`.
    pub fn linker_module_opt(&self) -> anyhow::Result<Option<ModuleInfo<'_>>> {
        let modules = self.modules()?;
        for module in modules.iter() {
            if module.module_name == LINKER_MODULE_NAME {
                return Ok(Some(module));
            }
        }
        Ok(None)
    }

    /// Gets the list of COFF groups defined in this binary.
    pub fn coff_groups(&self) -> anyhow::Result<&CoffGroups> {
        get_or_init_err(&self.cached.coff_groups, || self.read_coff_groups())
    }

    /// Reads (uncached) the list of COFF groups defined in this binary.
    pub fn read_coff_groups(&self) -> anyhow::Result<CoffGroups> {
        // S_COFFGROUP symbols are defined in the linker module.
        let Some(linker_module) = self.linker_module_opt()? else {
            return Ok(CoffGroups { vec: Vec::new() });
        };

        let Some(linker_module_stream) = linker_module.stream() else {
            bail!("The linker module does not contain any symbols.");
        };

        let mut linker_module_syms: Vec<u8> = vec![0; linker_module.sym_size() as usize];
        let sr = self.get_stream_reader(linker_module_stream)?;
        sr.read_exact_at(&mut linker_module_syms, 0)?;

        // Count the number of S_COFFGROUP symbols. We can use this to do a precise allocation.
        let mut num_coff_groups: usize = 0;
        for sym in SymIter::for_module_syms(&linker_module_syms) {
            if sym.kind == SymKind::S_COFFGROUP {
                num_coff_groups += 1;
            }
        }

        let mut groups = Vec::with_capacity(num_coff_groups);

        for sym in SymIter::for_module_syms(&linker_module_syms) {
            if sym.kind == SymKind::S_COFFGROUP {
                match sym.parse_as::<ms_codeview::syms::CoffGroup>() {
                    Ok(group) => {
                        groups.push(CoffGroup {
                            name: group.name.to_string(),
                            characteristics: group.fixed.characteristics.get(),
                            offset_segment: group.fixed.off_seg,
                            size: group.fixed.cb.get(),
                        });
                    }
                    Err(_) => {
                        warn!("failed to parse S_COFFGROUP symbol");
                    }
                }
            }
        }

        groups.sort_unstable_by_key(|g| g.offset_segment);

        Ok(CoffGroups { vec: groups })
    }

    /// Returns the target architecture for this PE binary.
    pub fn machine(&self) -> IMAGE_FILE_MACHINE {
        IMAGE_FILE_MACHINE(self.dbi_header.machine.get())
    }

    /// Returns the target CPU architecture.
    pub fn arch(&self) -> anyhow::Result<Arch> {
        match self.machine() {
            IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_AMD64 => Ok(Arch::AMD64),
            IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_ARM64 => Ok(Arch::ARM64),
            IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_I386 => Ok(Arch::X86),
            _ => bail!("target machine not supported"),
        }
    }
}

fn get_or_init_err<T, E, F: FnOnce() -> Result<T, E>>(cell: &OnceCell<T>, f: F) -> Result<&T, E> {
    if let Some(value) = cell.get() {
        return Ok(value);
    }

    match f() {
        Ok(value) => {
            let _ = cell.set(value);
            Ok(cell.get().unwrap())
        }
        Err(e) => Err(e),
    }
}

impl Pdb<RandomAccessFile> {
    /// Opens a PDB file.
    pub fn open(file_name: &Path) -> anyhow::Result<Box<Pdb<RandomAccessFile>>> {
        let f = ms_pdb_msf::open_options_shared(File::options().read(true)).open(file_name)?;
        let random_file = RandomAccessFile::from(f);
        Self::from_file_access(random_file, AccessMode::Read)
    }

    /// Reads the header of a PDB file and provides access to the streams contained within the
    /// PDB file.
    ///
    /// This function reads the MSF File Header, which is the header for the entire file.
    /// It also reads the stream directory, so it knows how to find each of the streams
    /// and the pages of the streams.
    pub fn open_from_file(file: File) -> anyhow::Result<Box<Self>> {
        let random_file = RandomAccessFile::from(file);
        Self::from_file_access(random_file, AccessMode::Read)
    }

    /// Opens a PDB file for editing. The file must use the MSF container format.
    pub fn modify(filename: &Path) -> anyhow::Result<Box<Pdb<sync_file::RandomAccessFile>>> {
        let file = File::options().read(true).write(true).open(filename)?;
        let random_file = sync_file::RandomAccessFile::from(file);
        Self::from_file_access(random_file, AccessMode::ReadWrite)
    }

    /// Opens an existing PDB file for read/write access, given a file name.
    ///
    /// The file _must_ use the MSF container format. MSFZ is not supported for read/write access.
    pub fn modify_from_file(file: File) -> anyhow::Result<Box<Self>> {
        let random_file = RandomAccessFile::from(file);
        Self::from_file_access(random_file, AccessMode::ReadWrite)
    }
}

impl<F: ReadAt> Pdb<F> {
    /// Reads the header of a PDB file and provides access to the streams contained within the
    /// PDB file.
    ///
    /// This function reads the MSF File Header, which is the header for the entire file.
    /// It also reads the stream directory, so it knows how to find each of the streams
    /// and the pages of the streams.
    pub fn open_from_random_file(random_file: F) -> anyhow::Result<Box<Self>> {
        Self::from_file_access(random_file, AccessMode::Read)
    }

    /// Opens an existing PDB file for read/write access, given a file name.
    ///
    /// The file _must_ using the MSF container format. MSFZ is not supported for read/write access.
    pub fn modify_from_random_file(random_file: F) -> anyhow::Result<Box<Self>> {
        Self::from_file_access(random_file, AccessMode::ReadWrite)
    }
}

impl<F> std::ops::Deref for Pdb<F> {
    type Target = Container<F>;

    fn deref(&self) -> &Self::Target {
        &self.container
    }
}

impl<F> std::ops::DerefMut for Pdb<F> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.container
    }
}

/// This is the key used to associate a given PE executable (DLL or EXE) with a PDB.
/// All values come from the PDBI stream.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BindingKey {
    /// The GUID. When MSVC tools are run in deterministic mode, this value is a hash of the PE
    /// image, rather than being assigned using an RNG.
    pub guid: uuid::Uuid,
    /// The age of the executable. This is incremented every time the DLL + PDB are modified.
    pub age: u32,
}

impl Debug for BindingKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.age > 0x1000 {
            write!(f, "{:?} age 0x{:x}", self.guid, self.age)
        } else {
            write!(f, "{:?} age {}", self.guid, self.age)
        }
    }
}

/// The name of the special "linker" module.
///
/// The linker module is created by the linker and is not an input to the linker. It contains
/// special / well-known symbols, such as `S_COFFGROUP`.
pub const LINKER_MODULE_NAME: &str = "* Linker *";
