use anyhow::{bail, Result};
use std::path::PathBuf;

#[derive(clap::Parser)]
pub struct PdbList {
    /// The set of PDB files to read. This can contain globs, e.g. `*.pdb` or `foo\**\*.pdb`.
    pub pdbs: Vec<String>,
}

impl PdbList {
    pub fn get_paths(&self) -> Result<Vec<PathBuf>> {
        let paths = self.get_paths_empty_ok()?;
        if paths.is_empty() {
            bail!("This command requires that you specify one or more PDB files.");
        }
        Ok(paths)
    }

    pub fn get_paths_empty_ok(&self) -> Result<Vec<PathBuf>> {
        let mut file_names = Vec::new();
        for file_name_or_glob in self.pdbs.iter() {
            if file_name_or_glob.contains(['?', '*']) {
                for f in glob::glob(file_name_or_glob)? {
                    let f = f?;
                    if f.is_file() {
                        file_names.push(f);
                    }
                }
            } else {
                file_names.push(PathBuf::from(file_name_or_glob));
            }
        }

        Ok(file_names)
    }
}
