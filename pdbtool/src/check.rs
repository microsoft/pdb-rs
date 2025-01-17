use anyhow::{bail, Context, Result};
use mspdb::dbi::section_map::SectionMap;
use mspdb::diag::Diags;
use mspdb::utils::iter::IteratorWithRangesExt;
use mspdb::{syms, Stream};
use tracing::{trace_span, debug};
use std::io::Write;
use std::path::Path;
use structopt::StructOpt;

#[derive(StructOpt)]
pub struct CheckOptions {
    #[structopt(flatten)]
    pdbs: crate::glob_pdbs::PdbList,

    /// Perform all supported checks. This is the default if no specific checks are enabled.
    #[structopt(long)]
    all: bool,

    /// Check the Names Stream (`/names`)
    #[structopt(long)]
    names: bool,

    /// Check the Type Stream (TPI).
    #[structopt(long)]
    tpi: bool,

    /// Check the Id Stream (IPI)
    #[structopt(long)]
    ipi: bool,

    /// Check the Global Stream Stream
    #[structopt(long)]
    gss: bool,

    /// Check the DBI Section Map
    #[structopt(long)]
    section_map: bool,

    /// Check the DBI Section Contributions
    #[structopt(long)]
    section_contribs: bool,

    #[structopt(long)]
    modules: bool,

    /// Check symbol streams for each module
    #[structopt(long)]
    module_symbols: bool,

    #[structopt(long)]
    dbi_sources: bool,

    /// multithreaded
    #[structopt(long)]
    mt: bool,

    #[structopt(flatten)]
    warnings: Warnings,

    #[structopt(long)]
    verbose: bool,
}

#[derive(StructOpt)]
pub struct Warnings {
    /// Turns on all warnings.
    #[structopt(long)]
    warn_all: bool,

    /// Warn if the `unused1` field of `ModuleInfo` in the DBI stream is non-zero.
    #[structopt(long)]
    warn_module_info_unused1: bool,

    /// Warn if the `unused2` field of `ModuleInfo` in the DBI stream is non-zero.
    #[structopt(long)]
    warn_module_info_unused2: bool,

    /// Warn if the `module_index` field of a `SectionContrib` structure within a `ModuleInfo`
    /// in the DBI has an incorrect value.
    #[structopt(long)]
    warn_section_contrib_module_index: bool,

    /// Warn if a module is found that contains obsolete C11 line data.
    #[structopt(long)]
    warn_obsolete_c11_line_data: bool,
}

impl Warnings {
    pub fn activate_all(&mut self) {
        if self.warn_all {
            self.warn_module_info_unused1 = true;
            self.warn_module_info_unused2 = true;
            self.warn_section_contrib_module_index = true;
            self.warn_obsolete_c11_line_data = true;
        }
    }
}

pub fn check_command(mut options: CheckOptions) -> anyhow::Result<()> {
    let _span = trace_span!("check_command").entered();

    macro_rules! enable_defaults {
        ($($field:ident),*) => {
            if options.all || (true $( && !options.$field )* ) {
                println!("Enabling all checks");
                $( options.$field = true; )*
            }
        }
    }
    enable_defaults!(
        names,
        tpi,
        ipi,
        gss,
        section_map,
        section_contribs,
        modules,
        module_symbols,
        dbi_sources
    );

    options.warnings.activate_all();

    let file_names = options.pdbs.get_paths()?;

    let mut num_failed: u32 = 0;
    let mut num_succeeded: u32 = 0;

    let mut show_one_result = |file_name: &Path, result: anyhow::Result<Diags>| {
        let stdout = std::io::stdout();

        match result {
            Ok(diags) => {
                num_succeeded += 1;
                let mut out = stdout.lock();
                if diags.has_errors() {
                    writeln!(out, "Checks failed for PDB: {}", file_name.display()).unwrap();
                    writeln!(out, "{}", diags).unwrap();
                } else if diags.has_warnings() {
                    writeln!(
                        out,
                        "Checks passed, but with warnings, for PDB: {}",
                        file_name.display()
                    )
                    .unwrap();
                    writeln!(out, "{}", diags).unwrap();
                } else {
                    if options.verbose {
                        writeln!(out, "Checks passed: {}", file_name.display()).unwrap();
                    }
                }
            }
            Err(e) => {
                let mut out = stdout.lock();
                writeln!(out, "Check failed for PDB: {}", file_name.display()).unwrap();
                writeln!(out, "    {e:?}").unwrap();
                num_failed += 1;
            }
        }
    };

    for file_name in file_names.iter() {
        let result = check_one_pdb(&options, file_name);
        show_one_result(file_name, result);
    }

    println!("Number of PDBs that passed checks: {}", num_succeeded);

    if num_failed != 0 {
        println!("Number of PDBs that failed checks: {}", num_failed);
        bail!("One or more PDBs failed checks.");
    }

    Ok(())
}

fn check_one_pdb(options: &CheckOptions, file_name: &Path) -> anyhow::Result<Diags> {
    let mut diags = Diags::new();
    let pdb = mspdb::Pdb::open(file_name)?;

    if let Some(msf) = pdb.msf() {
        if msf.page_size() < mspdb::msf::MIN_PAGE_SIZE
            || msf.page_size() > mspdb::msf::MAX_PAGE_SIZE
        {
            diags.warning(format!("This PDB uses a page size of 0x{:x} ({} bits), which is outside of the legal range of 0x{:x} ..= 0x{:x}",
            u32::from(msf.page_size()),
            msf.page_size().exponent(),
            u32::from(mspdb::msf::MIN_PAGE_SIZE),
            u32::from(mspdb::msf::MAX_PAGE_SIZE)
        ));
        }
    }

    // Most PDBs have a `/names` stream, but not all of them.
    let names = pdb.names()?;
    if let Some(names_stream) = pdb.named_stream(mspdb::names::NAMES_STREAM_NAME) {
        if options.names {
            names.check(names_stream, 0, &mut diags);
        }
    }

    let dbi = pdb.read_dbi_stream()?;

    if options.section_map {
        let section_map = dbi.section_map()?;
        check_section_map(&mut diags, &section_map)?;
    }

    if options.section_contribs {
        let section_contribs = dbi.section_contributions()?;
        section_contribs.check(&mut diags);
    }

    let sources = dbi.sources()?;
    if options.dbi_sources {
        sources.check(&mut diags);
    }

    let gss = pdb.read_gss()?;

    if options.gss {
        if let Ok(gss_stream) = dbi.header().sym_record_stream() {
            syms::check::check_symbol_stream(
                &mut diags,
                syms::SymbolStreamKind::Global,
                gss_stream,
                &gss.stream_data,
            )?;
        }
    }

    let tpi = if options.tpi {
        mspdb::tpi::check::check_tpi_stream(&pdb, &mut diags)?
    } else {
        // Load the TPI, even if we don't check its invariants.
        pdb.read_type_stream()?
    };

    if options.ipi {
        mspdb::tpi::check::check_ipi_stream(
            &pdb,
            &mut diags,
            tpi.type_index_begin(),
            tpi.type_index_end(),
        )?;
    }

    // Read the records in the GSS and build a table of the byte offsets of each record in the GSS.
    // We will use this to validate byte offsets found in other tables, below.
    let mut gss_symbol_offsets: Vec<u32> = Vec::new();
    for (sym_range, _sym) in gss.iter_syms().with_ranges() {
        gss_symbol_offsets.push(sym_range.start as u32);
    }

    let is_gss_byte_offset_valid =
        |byte_offset: u32| -> bool { gss_symbol_offsets.binary_search(&byte_offset).is_ok() };

    if options.modules {
        mspdb::dbi::check_module_infos(&pdb, &mut diags)?;

        for (module_index, module) in dbi.iter_modules().enumerate() {
            let mut check_module = || -> Result<()> {
                let h = module.header();

                if options.warnings.warn_module_info_unused1
                    && h.unused1.get() != 0
                    && h.unused1.get() as usize != module_index
                {
                    if let Some(w) = diags.warning(format!(
                        "module has invalid value in ModuleInfo::unused1 field: 0x{:08x}",
                        h.unused1.get()
                    )) {
                        w.stream(Stream::DBI.into())
                            .module(module_index as u32, module.module_name());
                    }
                }

                if options.warnings.warn_module_info_unused2 && h.unused2.get() != 0 {
                    if let Some(w) = diags.warning(format!(
                        "module has non-zero value in ModuleInfo::unused2 field: 0x{:08x}",
                        h.unused2.get()
                    )) {
                        w.module(module_index as u32, module.module_name());
                    }
                }

                // C11 line data is obsolete
                if options.warnings.warn_obsolete_c11_line_data
                    && module.header().c11_byte_size.get() != 0
                {
                    if let Some(w) =
                        diags.warning("module has non-zero value for c11_byte_size (obsolete)")
                    {
                        w.module(module_index as u32, module.module_name());
                    }
                }

                if let Some(module_stream_data) = pdb.read_module_stream(&module)? {
                    let module_stream_index = module.stream().unwrap();
                    mspdb::modi::check::check_module_stream(
                        &mut diags,
                        module_index,
                        &module,
                        &module_stream_data,
                        names,
                        &sources,
                    )?;

                    // Check the Global Ref substream. This is a barely-documented extension.
                    let global_refs = module_stream_data.global_refs();
                    for &global_ref in global_refs.iter() {
                        if !diags.wants_warning() {
                            break;
                        }

                        let global_ref = global_ref.get();
                        if !is_gss_byte_offset_valid(global_ref) {
                            diags.warning(format!("module #{module_index} has a global ref at byte offset #{global_ref} that is invalid"));
                        }
                    }

                    syms::check::check_symbol_stream(
                        &mut diags,
                        syms::SymbolStreamKind::Module,
                        module_stream_index,
                        module_stream_data.sym_data(),
                    )?;
                }
                Ok(())
            };

            check_module().with_context(|| {
                format!(
                    "In module # {module_index}, module stream {:?}",
                    module.stream()
                )
            })?;
        }
    }

    Ok(diags)
}

fn check_section_map(diags: &mut Diags, section_map: &SectionMap) -> anyhow::Result<()> {
    debug!("Checking DBI Section Map");

    for (i, section) in section_map.entries.iter().enumerate() {
        if section.section_name.get() != 0xffff {
            if let Some(w) = diags.warning(format!(
                "section #{i} has a value other than 0xffff for section_name"
            )) {
                w.stream(Stream::DBI.into());
            }
        }
        if section.class_name.get() != 0xffff {
            diags.warning(format!(
                "section #{i} has a value other than 0xffff for class_name"
            ));
        }

        // In all unmanaged PDBs, we see that the last section map entry has section_length
        // equal to 0xffff_ffff. However, in some managed PDBs, this is not true.
        // TODO: Decide whether the managed PDBs are "wrong" or whether the invariant should be relaxed.
        if false {
            let section_length = section.section_length.get();
            if i < section_map.entries.len() - 1 {
                if section_length == 0xffff_ffff {
                    diags.warning(format!("section #{i} has an invalid length (0xffffffff)"));
                }
            } else {
                if section_length != 0xffff_ffff {
                    diags.warning(format!("section #{i} (last section) was expected to have a length of 0xffffffff, but instead the length is 0x{:x}", section_length));
                }
            }
        }
    }

    Ok(())
}
