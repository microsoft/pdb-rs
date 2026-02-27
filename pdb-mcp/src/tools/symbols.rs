use crate::format;
use crate::server::PdbMcpServer;
use bstr::BStr;
use ms_pdb::syms::SymData;
use regex::bytes::Regex as BytesRegex;
use std::fmt::Write;

pub async fn find_global_impl(
    server: &PdbMcpServer,
    alias: String,
    name: String,
    undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    match pdb.find_global_by_name(BStr::new(name.as_bytes())) {
        Ok(Some(sym)) => {
            format!(
                "GSI match: {}",
                format::format_sym(sym.kind, sym.data, undecorate)
            )
        }
        Ok(None) => format!("No global symbol found with name '{name}'."),
        Err(e) => format!("Error searching GSI: {e}"),
    }
}

pub async fn find_public_impl(
    server: &PdbMcpServer,
    alias: String,
    name: String,
    undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    match pdb.find_public_by_name(BStr::new(name.as_bytes())) {
        Ok(Some(pub_sym)) => {
            let sym_name = pub_sym.name.to_string();
            let display = if undecorate {
                crate::undecorate::format_with_undecoration(&sym_name)
            } else {
                sym_name
            };
            format!(
                "PSI match: S_PUB32 {} flags=0x{:08x} {}",
                pub_sym.fixed.offset_segment,
                pub_sym.fixed.flags.get(),
                display,
            )
        }
        Ok(None) => format!("No public symbol found with name '{name}'."),
        Err(e) => format!("Error searching PSI: {e}"),
    }
}

pub async fn find_public_by_addr_impl(
    server: &PdbMcpServer,
    alias: String,
    section: u16,
    offset: u32,
    undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let gss = match pdb.gss() {
        Ok(g) => g,
        Err(e) => return format!("Error loading GSS: {e}"),
    };

    let psi = match pdb.psi() {
        Ok(p) => p,
        Err(e) => return format!("Error loading PSI: {e}"),
    };

    match psi.find_symbol_by_addr(gss, section, offset) {
        Ok(Some((pub_sym, distance))) => {
            let sym_name = pub_sym.name.to_string();
            let display = if undecorate {
                crate::undecorate::format_with_undecoration(&sym_name)
            } else {
                sym_name
            };
            let mut out = format!(
                "PSI addr match: S_PUB32 {} flags=0x{:08x} {}",
                pub_sym.fixed.offset_segment,
                pub_sym.fixed.flags.get(),
                display,
            );
            if distance > 0 {
                write!(out, " (offset +0x{distance:x} from symbol start)").unwrap();
            }
            out
        }
        Ok(None) => format!("No public symbol found at section {section}, offset 0x{offset:x}."),
        Err(e) => format!("Error searching PSI address map: {e}"),
    }
}

pub async fn search_symbols_impl(
    server: &PdbMcpServer,
    alias: String,
    pattern: String,
    max: Option<usize>,
    undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let rx = match BytesRegex::new(&pattern) {
        Ok(rx) => rx,
        Err(e) => return format!("Invalid regex: {e}"),
    };

    let gss = match pdb.gss() {
        Ok(g) => g,
        Err(e) => return format!("Error loading GSS: {e}"),
    };

    let max = max.unwrap_or(50);
    let mut out = String::new();
    let mut found = 0usize;
    let mut total_scanned = 0usize;

    for sym in gss.iter_syms() {
        total_scanned += 1;

        if let Ok(sym_data) = SymData::parse(sym.kind, sym.data) {
            if let Some(name) = sym_data.name() {
                if rx.is_match(name) {
                    found += 1;
                    if found <= max {
                        writeln!(
                            out,
                            "  {}",
                            format::format_sym(sym.kind, sym.data, undecorate)
                        )
                        .unwrap();
                    }
                }
            }
        }
    }

    let mut header = String::new();
    if found > max {
        writeln!(
            header,
            "Found {found} matches (showing {max}). Scanned {total_scanned} symbols."
        )
        .unwrap();
    } else {
        writeln!(
            header,
            "Found {found} matches. Scanned {total_scanned} symbols."
        )
        .unwrap();
    }

    format!("{header}{out}")
}
