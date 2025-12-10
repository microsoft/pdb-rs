use anyhow::{Context, Result, bail};
use bstr::ByteSlice;
use ms_pdb::{BStr, Pdb};

#[derive(clap::Parser)]
pub struct AddSrcOptions {
    /// The PDB to modify.
    pub pdb: String,

    /// A list of source files to embed into the PDB.
    pub source_files: Vec<String>,

    /// A list of directories (path prefixes). If this list contains any values, then
    /// this tool will scan the sources table within the PDB and look for any source file
    /// that was compiled and is underneath any path in the `under` list. If so, then the
    /// source file will be read and embedded into the PDB.
    ///
    /// For example: `pdbtool add-src foo.pdb --under=d:\some\dir`
    #[arg(long)]
    pub under: Vec<String>,
}

pub fn command(options: AddSrcOptions) -> Result<()> {
    if options.source_files.is_empty() && options.under.is_empty() {
        bail!("You must specify at least one source file to add to the PDB.");
    }

    let mut pdb = ms_pdb::Pdb::modify(options.pdb.as_ref())?;

    for source_file in options.source_files.iter() {
        let (fake_source_file, real_source_file): (&str, &str) =
            if let Some(s) = source_file.split_once('=') {
                s
            } else {
                (source_file, source_file)
            };

        embed_source_file(&mut pdb, fake_source_file, real_source_file)?;
    }

    if !options.under.is_empty() {
        let sources = pdb.sources()?;

        // Build a list of the unique source files. This should really be moved into Pdb.
        let mut source_files: Vec<(u32, &BStr)> = sources.iter_sources().collect();
        source_files.sort_unstable_by_key(|&(offset, _name)| offset);
        source_files.dedup();

        // Scan each source file.
        for &(_, file_name) in source_files.iter() {
            let Ok(file_name) = file_name.to_str() else {
                // icky file name
                continue;
            };

            for under in options.under.iter() {
                if strip_prefix_ignore_ascii_case(under, file_name).is_some() {
                    println!("prefix matched: {under} : {file_name}");
                }
            }
        }
    }

    pdb.flush_all()?;
    let committed = pdb.msf_mut_err()?.commit()?;
    if committed {
        println!("Changes successfully committed to PDB.");
    } else {
        println!(
            "No changes were written to disk. The given files are already embedded in the PDB."
        );
    }

    Ok(())
}

/// Embeds a source file into the PDB.
///
/// `file_name` specifies the path to the file to embed. This function uses `file_name` to open
/// the file and read its contents.
///
/// `path_within_pdb` specifies the file name to use within the PDB. This can be different from
/// `file_name`. For example, source root directories (or object directories) can be chopped off
/// and replaced with a well-known prefix.
///
/// If there is already a source file embedded in the PDB with the same file name (as specified by `path_within_pdb`), then this will update
/// the existing stream instead of modifying it.
fn embed_source_file(pdb: &mut Pdb, path_within_pdb: &str, file_name: &str) -> Result<()> {
    let file_contents =
        std::fs::read(file_name).with_context(|| format!("Failed to open file: {file_name}"))?;
    if pdb.add_embedded_source(path_within_pdb, &file_contents)? {
        println!("{file_name} : embedded");
    } else {
        println!("{file_name} : already embedded (no change)");
    }

    Ok(())
}

fn strip_prefix_ignore_ascii_case<'a>(prefix: &str, s: &'a str) -> Option<&'a str> {
    if s.is_char_boundary(prefix.len()) {
        let (lo, hi) = s.split_at(prefix.len());
        if prefix.eq_ignore_ascii_case(lo) {
            Some(hi)
        } else {
            None
        }
    } else {
        None
    }
}
