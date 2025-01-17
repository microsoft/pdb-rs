//! Contains PDB checking rules specific to the Windows (OS) repo.

use anyhow::{bail, Result};
use bstr::{BStr, ByteSlice};
use mspdb::utils::path::path_contains;
use std::path::Path;
use structopt::StructOpt;

use crate::glob_pdbs::PdbList;

#[derive(StructOpt)]
pub struct OSCheckOptions {
    /// The set of PDBs to check.
    #[structopt(flatten)]
    pub pdbs: PdbList,

    /// The list of allowed source root prefixes. Any source file that does not match with one
    /// of these prefixes will cause an error.
    #[structopt(long)]
    pub allowed_source_root: Vec<String>,
}

pub fn oscheck_command(options: OSCheckOptions) -> Result<()> {
    let file_names = options.pdbs.get_paths()?;

    if options.allowed_source_root.is_empty() {
        bail!("You must specify at least one valid root, using --allowed-source-root=... .");
    }

    let mut ok = true;

    for file_name in file_names.iter() {
        match check_one_file(&options, file_name) {
            Ok(this_ok) => {
                ok &= this_ok;
            }
            Err(e) => {
                eprintln!("error : File failed checks: {}", file_name.display());
                eprintln!("error : {e:?}");
                ok = false;
            }
        }
    }

    if !ok {
        std::process::exit(1);
    }

    Ok(())
}

fn check_one_file(options: &OSCheckOptions, file_name: &Path) -> Result<bool> {
    let pdb = mspdb::Pdb::open(file_name)?;

    let modules = pdb.modules()?;
    let sources = pdb.sources()?;
    let modules_vec: Vec<_> = modules.iter().collect();

    let mut num_bad: u32 = 0;

    let mut sources_sorted: Vec<(u32, &BStr)> = sources.iter_sources().collect();
    sources_sorted.sort_unstable_by_key(|&(offset, _)| offset);
    sources_sorted.dedup_by_key(|&mut (offset, _)| offset);
    sources_sorted.sort_unstable_by_key(|(_, name)| *name);

    for &(source_name_offset, source_file) in sources_sorted.iter() {
        let source_file = source_file.to_str_lossy();
        if Path::new(&*source_file).is_relative() {
            // println!("Relative path is ok: {}", source_file);
            continue;
        }

        if options
            .allowed_source_root
            .iter()
            .any(|allowed_root| path_contains(allowed_root, &source_file))
        {
            // println!("Source file is ok: {}", source_file);
            continue;
        }

        if num_bad == 0 {
            eprintln!("error : PDB has errors: {}", file_name.display());
        }

        eprintln!("error : Source file is not under any permitted root: {source_file}");
        num_bad += 1;

        // Find one of the modules that includes this file.
        let mut num_modules_found = 0;
        for module_index in 0..sources.num_modules() {
            let module_name_offsets = sources.name_offsets_for_module(module_index)?;

            if module_name_offsets
                .iter()
                .any(|&i| i.get() == source_name_offset)
            {
                // Found one.
                let (module_name, obj_file) =
                    if let Some(module_info) = modules_vec.get(module_index) {
                        (module_info.module_name, module_info.obj_file)
                    } else {
                        ("(unknown)".into(), "(unknown)".into())
                    };
                eprintln!("    Source file is used by module # {module_index} :");
                eprintln!("        Module name:        {module_name}");
                eprintln!("        Module object file: {obj_file}");
                num_modules_found += 1;
                if num_modules_found == 5 {
                    eprintln!("(stopping)");
                    break;
                }
            }
        }

        eprintln!();
    }

    Ok(num_bad == 0)
}
