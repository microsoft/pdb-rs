use crate::server::PdbMcpServer;
use crate::undecorate;
use ms_pdb::lines::{FileChecksumsSubsection, LineData, LinesSubsection, SubsectionKind};
use ms_pdb::names::NameIndex;
use ms_pdb::syms::SymIter;
use std::fmt::Write;

/// Convert an RVA to section:offset using the section headers.
pub async fn rva_to_section_impl(server: &PdbMcpServer, alias: String, rva: u32) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;
    let sections = match pdb.section_headers() {
        Ok(s) => s,
        Err(e) => return format!("Error reading section headers: {e}"),
    };

    for (i, sh) in sections.iter().enumerate() {
        let sec_start = sh.virtual_address;
        let sec_end = sec_start + sh.physical_address_or_virtual_size;
        if rva >= sec_start && rva < sec_end {
            let offset = rva - sec_start;
            let name = sh.name();
            return format!(
                "RVA 0x{rva:x} → [{section}:{offset:08x}] (section {section} \"{name}\", offset 0x{offset:x})",
                section = i + 1,
            );
        }
    }

    format!("RVA 0x{rva:x} does not fall within any section.")
}

/// Convert section:offset to an RVA using the section headers.
pub async fn section_to_rva_impl(
    server: &PdbMcpServer,
    alias: String,
    section: u16,
    offset: u32,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;
    let sections = match pdb.section_headers() {
        Ok(s) => s,
        Err(e) => return format!("Error reading section headers: {e}"),
    };

    let idx = section as usize;
    if idx == 0 || idx > sections.len() {
        return format!(
            "Error: section {section} out of range (1..{}).",
            sections.len()
        );
    }

    let sh = &sections[idx - 1];
    let rva = sh.virtual_address + offset;
    let name = sh.name();
    format!("[{section}:{offset:08x}] → RVA 0x{rva:x} (section \"{name}\")")
}

/// Helper: convert RVA to (section_1based, offset) using section headers.
fn rva_to_sec_off(sections: &[ms_pdb::IMAGE_SECTION_HEADER], rva: u32) -> Option<(u16, u32)> {
    for (i, sh) in sections.iter().enumerate() {
        let sec_start = sh.virtual_address;
        let sec_end = sec_start + sh.physical_address_or_virtual_size;
        if rva >= sec_start && rva < sec_end {
            return Some(((i + 1) as u16, rva - sec_start));
        }
    }
    None
}

/// The big one: address → module, function, source file, line number.
pub async fn addr_to_line_impl(
    server: &PdbMcpServer,
    alias: String,
    rva: Option<u32>,
    section: Option<u16>,
    offset: Option<u32>,
    do_undecorate: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    // Step 1: Resolve to section:offset
    let sections = match pdb.section_headers() {
        Ok(s) => s,
        Err(e) => return format!("Error reading section headers: {e}"),
    };

    let (sec, off) = if let (Some(s), Some(o)) = (section, offset) {
        (s, o)
    } else if let Some(rva_val) = rva {
        match rva_to_sec_off(sections, rva_val) {
            Some(so) => so,
            None => return format!("RVA 0x{rva_val:x} does not fall within any section."),
        }
    } else {
        return "Error: provide either 'rva' or both 'section' and 'offset'.".to_string();
    };

    // Compute RVA for display
    let rva_val = if let Some(r) = rva {
        r
    } else {
        let idx = sec as usize;
        if idx > 0 && idx <= sections.len() {
            sections[idx - 1].virtual_address + off
        } else {
            0
        }
    };

    let mut out = String::new();
    writeln!(out, "Address [{sec}:{off:08x}] (RVA 0x{rva_val:x}):").unwrap();

    // Step 2: Section contribution lookup → module index
    let dbi = match pdb.read_dbi_stream() {
        Ok(d) => d,
        Err(e) => return format!("Error reading DBI stream: {e}"),
    };

    let contribs = match dbi.section_contributions() {
        Ok(c) => c,
        Err(e) => return format!("Error reading section contributions: {e}"),
    };

    let contrib = match contribs.find(sec, off as i32) {
        Some(c) => c,
        None => {
            writeln!(out, "  No section contribution found at this address.").unwrap();
            return out;
        }
    };

    let mod_index = contrib.module_index.get() as u32;
    writeln!(out, "  Module:     [{mod_index}]").unwrap();

    // Get module name
    let modules = match pdb.modules() {
        Ok(m) => m,
        Err(e) => {
            writeln!(out, "  Error reading modules: {e}").unwrap();
            return out;
        }
    };

    if let Some(module_info) = modules.iter().nth(mod_index as usize) {
        writeln!(
            out,
            "  Module:     [{mod_index}] {}",
            module_info.module_name
        )
        .unwrap();

        // Step 3: Scan module symbols for enclosing procedure
        let sym_data = match pdb.read_module_symbols(&module_info) {
            Ok(d) => d,
            Err(e) => {
                writeln!(out, "  Error reading module symbols: {e}").unwrap();
                return out;
            }
        };

        if !sym_data.is_empty() {
            let sym_bytes = zerocopy::IntoBytes::as_bytes(sym_data.as_slice());
            find_enclosing_proc(&mut out, sym_bytes, sec, off, do_undecorate);
        }

        // Step 4: Read C13 line data and find the matching line
        let modi = match pdb.read_module_stream(&module_info) {
            Ok(Some(m)) => m,
            Ok(None) => {
                writeln!(out, "  No module stream.").unwrap();
                return out;
            }
            Err(e) => {
                writeln!(out, "  Error reading module stream: {e}").unwrap();
                return out;
            }
        };

        let c13_bytes = modi.c13_line_data_bytes();
        if c13_bytes.is_empty() {
            writeln!(out, "  No C13 line data.").unwrap();
            return out;
        }

        let line_data = LineData::new(c13_bytes);

        // Find the FILE_CHECKSUMS subsection for file name resolution
        let checksums = line_data.find_checksums();

        // Load names stream for resolving NameIndex → file path
        let names = pdb.names().ok();

        // Scan LINES subsections
        let mut found_line = false;
        for subsection in line_data.subsections() {
            if subsection.kind != SubsectionKind::LINES {
                continue;
            }

            let lines_sub = match LinesSubsection::parse(subsection.data) {
                Ok(l) => l,
                Err(_) => continue,
            };

            let contrib_sec = lines_sub.contribution.segment.get();
            let contrib_off = lines_sub.contribution.offset.get();
            let contrib_size = lines_sub.contribution.size.get();

            // Check if our address falls within this contribution
            if contrib_sec != sec {
                continue;
            }
            if off < contrib_off || off >= contrib_off + contrib_size {
                continue;
            }

            let relative_off = off - contrib_off;

            for block in lines_sub.blocks() {
                let lines = block.lines();
                if lines.is_empty() {
                    continue;
                }

                // Find the line record with the largest offset <= relative_off
                let mut best_line: Option<&ms_pdb::lines::LineRecord> = None;
                for line_rec in lines {
                    if line_rec.offset.get() <= relative_off {
                        best_line = Some(line_rec);
                    } else {
                        break; // Lines are sorted by offset
                    }
                }

                if let Some(line_rec) = best_line {
                    // Resolve file name
                    let file_name =
                        resolve_file_name(&checksums, &names, block.header.file_index.get());

                    let line_num = line_rec.line_num_start();
                    let line_off = relative_off - line_rec.offset.get();

                    writeln!(out, "  File:       {file_name}").unwrap();
                    writeln!(out, "  Line:       {line_num}").unwrap();
                    if line_off > 0 {
                        writeln!(out, "  Offset:     +0x{line_off:x} bytes from line start")
                            .unwrap();
                    }

                    found_line = true;
                    break;
                }
            }

            if found_line {
                break;
            }
        }

        if !found_line {
            writeln!(out, "  No line data for this address.").unwrap();
        }
    }

    out
}

/// Scan module symbol bytes to find the S_GPROC32/S_LPROC32 that contains [section:offset].
fn find_enclosing_proc(
    out: &mut String,
    sym_bytes: &[u8],
    section: u16,
    offset: u32,
    do_undecorate: bool,
) {
    for sym in SymIter::for_module_syms(sym_bytes) {
        if !sym.kind.is_proc() {
            continue;
        }

        if let Ok(proc_data) = sym.parse_as::<ms_pdb::syms::Proc>() {
            let proc_sec = proc_data.fixed.offset_segment.segment();
            let proc_off = proc_data.fixed.offset_segment.offset();
            let proc_len = proc_data.fixed.proc_len.get();

            if proc_sec == section && offset >= proc_off && offset < proc_off + proc_len {
                let name = proc_data.name.to_string();
                let display = if do_undecorate {
                    undecorate::format_with_undecoration(&name)
                } else {
                    name
                };
                let func_offset = offset - proc_off;
                writeln!(out, "  Function:   {display}").unwrap();
                writeln!(
                    out,
                    "  Func addr:  [{section}:{proc_off:08x}] len=0x{proc_len:x}"
                )
                .unwrap();
                if func_offset > 0 {
                    writeln!(out, "  Func off:   +0x{func_offset:x}").unwrap();
                }
                return;
            }
        }
    }

    writeln!(
        out,
        "  Function:   (not found — address may be in data or thunk)"
    )
    .unwrap();
}

/// Resolve a file_index (from BlockHeader) → file name string.
fn resolve_file_name(
    checksums: &Option<FileChecksumsSubsection<'_>>,
    names: &Option<&ms_pdb::names::NamesStream<Vec<u8>>>,
    file_index: u32,
) -> String {
    let Some(checksums) = checksums else {
        return format!("(file_index=0x{file_index:x}, no checksums subsection)");
    };

    let file = match checksums.get_file(file_index) {
        Ok(f) => f,
        Err(_) => return format!("(file_index=0x{file_index:x}, lookup failed)"),
    };

    let name_index = NameIndex(file.header.name.get());

    let Some(names) = names else {
        return format!("(name_index={}, no names stream)", name_index.0);
    };

    match names.get_string(name_index) {
        Ok(name) => name.to_string(),
        Err(_) => format!("(name_index={}, lookup failed)", name_index.0),
    }
}
