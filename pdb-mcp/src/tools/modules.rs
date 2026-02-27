use crate::format;
use crate::server::PdbMcpServer;
use ms_pdb::syms::SymIter;
use regex::bytes::Regex as BytesRegex;
use std::fmt::Write;

pub async fn list_modules_impl(
    server: &PdbMcpServer,
    alias: String,
    module_name_regex: Option<String>,
    obj_file_regex: Option<String>,
    max: Option<usize>,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let modules = match pdb.modules() {
        Ok(m) => m,
        Err(e) => return format!("Error reading modules: {e}"),
    };

    let name_rx = match module_name_regex.as_deref().map(BytesRegex::new) {
        Some(Ok(rx)) => Some(rx),
        Some(Err(e)) => return format!("Invalid module_name_regex: {e}"),
        None => None,
    };

    let obj_rx = match obj_file_regex.as_deref().map(BytesRegex::new) {
        Some(Ok(rx)) => Some(rx),
        Some(Err(e)) => return format!("Invalid obj_file_regex: {e}"),
        None => None,
    };

    let max = max.unwrap_or(100);
    let mut out = String::new();
    let mut matched = 0usize;
    let mut total = 0usize;

    for (idx, module) in modules.iter().enumerate() {
        total += 1;

        if let Some(rx) = &name_rx {
            if !rx.is_match(module.module_name) {
                continue;
            }
        }
        if let Some(rx) = &obj_rx {
            if !rx.is_match(module.obj_file) {
                continue;
            }
        }

        matched += 1;
        if matched <= max {
            writeln!(
                out,
                "[{idx:>5}] {} (obj: {}) files={} sym_size={}",
                format::bstr_display(module.module_name),
                format::bstr_display(module.obj_file),
                module.header.source_file_count.get(),
                module.header.sym_byte_size.get(),
            )
            .unwrap();
        }
    }

    let mut header = String::new();
    if matched > max {
        writeln!(
            header,
            "Showing {max} of {matched} matching modules ({total} total). Use tighter regex or increase max."
        )
        .unwrap();
    } else {
        writeln!(header, "{matched} matching modules ({total} total).").unwrap();
    }

    format!("{header}{out}")
}

pub async fn module_symbols_impl(
    server: &PdbMcpServer,
    alias: String,
    module: String,
    undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let modules = match pdb.modules() {
        Ok(m) => m,
        Err(e) => return format!("Error reading modules: {e}"),
    };

    // Find the module by index or name substring
    let module_info = if let Ok(idx) = module.parse::<usize>() {
        modules.iter().nth(idx)
    } else {
        let lower = module.to_lowercase();
        modules
            .iter()
            .find(|m| m.module_name.to_string().to_lowercase().contains(&lower))
    };

    let Some(module_info) = module_info else {
        return format!("Module not found: '{module}'.");
    };

    let sym_data = match pdb.read_module_symbols(&module_info) {
        Ok(d) => d,
        Err(e) => return format!("Error reading module symbols: {e}"),
    };

    if sym_data.is_empty() {
        return "Module has no symbols.".to_string();
    }

    let sym_bytes = zerocopy::IntoBytes::as_bytes(sym_data.as_slice());

    let mut out = String::new();
    writeln!(
        out,
        "Symbols for module: {}",
        format::bstr_display(module_info.module_name)
    )
    .unwrap();

    let mut count = 0usize;
    for sym in SymIter::for_module_syms(sym_bytes) {
        writeln!(
            out,
            "  {}",
            format::format_sym(sym.kind, sym.data, undecorate)
        )
        .unwrap();
        count += 1;
        if count >= 500 {
            writeln!(out, "  ... (truncated at {count} symbols)").unwrap();
            break;
        }
    }
    writeln!(out, "Total: {count} symbols shown.").unwrap();

    out
}

pub async fn module_source_files_impl(
    server: &PdbMcpServer,
    alias: String,
    module: String,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let modules = match pdb.modules() {
        Ok(m) => m,
        Err(e) => return format!("Error reading modules: {e}"),
    };

    // Find the module by index or name substring
    let (mod_idx, module_info) = if let Ok(idx) = module.parse::<usize>() {
        match modules.iter().enumerate().nth(idx) {
            Some((i, m)) => (i, m),
            None => return format!("Module index {idx} out of range."),
        }
    } else {
        let lower = module.to_lowercase();
        match modules
            .iter()
            .enumerate()
            .find(|(_, m)| m.module_name.to_string().to_lowercase().contains(&lower))
        {
            Some((i, m)) => (i, m),
            None => return format!("Module not found: '{module}'."),
        }
    };

    // Read the DBI sources substream
    let dbi_sources_bytes = match pdb.read_sources_data() {
        Ok(d) => d,
        Err(e) => return format!("Error reading DBI sources: {e}"),
    };

    let sources = match ms_pdb::dbi::DbiSourcesSubstream::parse(&dbi_sources_bytes) {
        Ok(s) => s,
        Err(e) => return format!("Error parsing DBI sources: {e}"),
    };

    let mut out = String::new();
    writeln!(
        out,
        "Source files for module [{}]: {}",
        mod_idx,
        format::bstr_display(module_info.module_name)
    )
    .unwrap();

    match sources.name_offsets_for_module(mod_idx) {
        Ok(offsets) => {
            if offsets.is_empty() {
                writeln!(out, "  (no source files)").unwrap();
            } else {
                for offset in offsets {
                    match sources.get_source_file_name_at(offset.get()) {
                        Ok(name) => {
                            writeln!(out, "  {}", format::bstr_display(name)).unwrap();
                        }
                        Err(e) => {
                            writeln!(out, "  <error reading file name: {e:?}>").unwrap();
                        }
                    }
                }
            }
        }
        Err(e) => {
            writeln!(out, "  Error getting file offsets: {e}").unwrap();
        }
    }

    out
}
