use anyhow::{bail, Context, Result};
use ms_pdb::{Pdb, RandomAccessFile};
use std::path::{Path, PathBuf};
use tracing::{error, info, warn};

/// Checks whether a given PDB is well-formed (not corrupted).
/// Can check more than one PDB at a time.
///
/// The default behavior is to open the PDB and do nothing else. This simulates
/// the most basic behavior of any tool that reads a PDB. Additional checks
/// may be enabled by setting flags.
#[derive(clap::Parser)]
pub(crate) struct CheckOptions {
    /// The files to check.
    pub(crate) files: Vec<String>,

    #[arg(long)]
    pub check_modules: bool,
}

pub(crate) fn command(mut options: CheckOptions) -> Result<()> {
    if options.files.is_empty() {
        bail!("You must specify at least one file name (or file pattern) to check.");
    }

    let mut all_files: Vec<PathBuf> = Vec::with_capacity(options.files.len());

    for file_or_glob in std::mem::take(&mut options.files) {
        if file_or_glob.contains(['*', '?']) {
            let mut found_any = false;

            for file_name in glob::glob(&file_or_glob)
                .with_context(|| format!("File pattern: {file_or_glob}"))?
            {
                let file_name =
                    file_name.with_context(|| format!("File pattern: {file_or_glob}"))?;
                all_files.push(file_name);
                found_any = true;
            }

            if !found_any {
                warn!("File pattern did not match any files: {file_or_glob}");
            }
        } else {
            all_files.push(file_or_glob.into());
        }
    }

    let show_stat = |name: &str, value: u32| {
        info!("{:<40} : {:8}", name, value);
    };

    if all_files.len() > 1 {
        show_stat("Number of PDBs to check", all_files.len() as u32);
    }

    let mut stats = Stats::default();

    for file_name in all_files.iter() {
        stats.num_files_checked += 1;
        check_one(&options, &mut stats, Path::new(file_name));
    }

    info!("Results:");
    show_stat("Number of PDBs checked", stats.num_files_checked);
    show_stat("Number of PDBs with errors", stats.num_files_failed);

    if stats.num_portable_pdbs != 0 {
        show_stat("Number of portable PDBs (ignored)", stats.num_portable_pdbs);
    }
    if stats.num_unknown_files != 0 {
        show_stat(
            "Number of unrecognized files (ignored)",
            stats.num_unknown_files,
        );
    }

    Ok(())
}

#[derive(Default)]
struct Stats {
    pub num_files_checked: u32,
    pub num_files_failed: u32,
    pub num_portable_pdbs: u32,
    pub num_unknown_files: u32,
}

fn check_one(options: &CheckOptions, stats: &mut Stats, file_name: &Path) {
    let mut errors: Vec<String> = Vec::new();

    match check_one_err(options, stats, &mut errors, file_name) {
        Ok(()) => {
            if !errors.is_empty() {
                stats.num_files_failed += 1;

                let mut all_errors_text = String::new();
                for error in errors.iter() {
                    all_errors_text.push_str(error);
                    all_errors_text.push_str("\n");
                }

                error!("{} : has errors:\n", all_errors_text);
            }
        }
        Err(e) => {
            error!("{} : failed: {:?}", file_name.display(), e);
            stats.num_files_failed += 1;
        }
    }
}

fn check_one_err(
    options: &CheckOptions,
    stats: &mut Stats,
    errors: &mut Vec<String>,
    file_name: &Path,
) -> Result<()> {
    let f = RandomAccessFile::open(file_name)?;

    use ms_pdb::taster::{what_flavor, Flavor};
    match what_flavor(&f)? {
        Some(Flavor::Pdb | Flavor::Pdz) => {}

        Some(Flavor::PortablePdb) => {
            stats.num_portable_pdbs += 1;
            return Ok(());
        }

        None => {
            stats.num_unknown_files += 1;
            return Ok(());
        }
    }

    let pdb = Pdb::open_from_random_file(f)?;

    if options.check_modules {
        let modules = pdb.modules().with_context(|| "failed to get modules")?;
        let sources = pdb.sources().with_context(|| "failed to get sources")?;

        let mod_vec: Vec<_> = modules.iter().collect();

        if mod_vec.len() != sources.num_modules() {
            errors.push(format!("The number of DBI modules is not the same as the number of entries in the DBI Sources map.  {} vs {}", mod_vec.len(), sources.num_modules()));
        }
    }

    Ok(())
}
