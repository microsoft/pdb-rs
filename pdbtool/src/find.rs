use crate::dump::sym::dump_sym;
use anyhow::Result;
use bstr::BStr;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::syms::SymData;
use std::path::Path;

/// Searches the DBI Section Contributions table.
#[derive(clap::Parser)]
pub struct FindOptions {
    /// The PDB to search.
    pub pdb: String,

    /// COFF section index to search
    pub section: u16,

    /// The symbol name or contribution offset to search.
    pub name: String,
}

pub fn find_command(options: &FindOptions) -> Result<()> {
    let pdb = ms_pdb::Pdb::open(Path::new(&options.pdb))?;

    let dbi = pdb.read_dbi_stream()?;
    let contribs = dbi.section_contributions()?;

    if let Some(hex_str) = options.name.strip_prefix("0x") {
        let addr = u32::from_str_radix(hex_str, 0x10)?;

        if let Some(contrib) = contribs.find(options.section, addr as i32) {
            println!("Found contribution record:\n{contrib:#?}");
            println!("Module index = {}", contrib.module_index.get());
        } else {
            println!("No symbol found.");
        }
    } else {
        println!("name lookups are nyi");
    }

    Ok(())
}

/// Searches the TPI Stream for a given type.
#[derive(clap::Parser)]
pub struct FindNameOptions {
    /// The PDB to search.
    pub pdb: String,

    /// The type name to search for.
    pub name: String,

    /// Indicates that `name` is a regex.
    #[arg(long, short)]
    pub regex: bool,
}

pub fn find_name_command(options: &FindNameOptions) -> Result<()> {
    use crate::dump::sym::DumpSymsContext;

    let pdb = ms_pdb::Pdb::open(Path::new(&options.pdb))?;
    let arch = pdb.arch()?;
    let tpi = pdb.read_type_stream()?;
    let ipi = pdb.read_ipi_stream()?;
    let mut context = DumpSymsContext::new(arch, &tpi, &ipi);

    let gss = pdb.read_gss()?;

    if options.regex {
        let rx = regex::bytes::Regex::new(&options.name)?;

        let mut found_any = false;

        for (record_range, sym) in gss.iter_syms().with_ranges() {
            let sym_data = SymData::parse(sym.kind, sym.data)?;

            if let Some(sym_name) = sym_data.name() {
                if rx.is_match(sym_name) {
                    let mut out = String::new();
                    dump_sym(
                        &mut out,
                        &mut context,
                        record_range.start as u32,
                        sym.kind,
                        sym.data,
                    )?;
                    print!("{out}");
                    found_any = true;
                }
            }
        }

        if !found_any {
            println!("No matches found.");
        }
    } else {
        let gsi = pdb.read_gsi()?;
        if let Some(sym) = gsi.find_symbol(&gss, BStr::new(&options.name))? {
            let mut out = String::new();
            dump_sym(&mut out, &mut context, 0, sym.kind, sym.data)?;
            print!("{out}");
            return Ok(());
        }
        println!("Symbol not found.");
    }

    Ok(())
}
