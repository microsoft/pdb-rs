use crate::server::PdbMcpServer;
use std::fmt::Write;

pub async fn section_headers_impl(
    server: &PdbMcpServer,
    alias: String,
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

    let mut out = String::new();
    writeln!(out, "Section Headers ({} sections):", sections.len()).unwrap();
    writeln!(
        out,
        "  {:>4}  {:8}  {:>12}  {:>12}  {:>10}",
        "#", "Name", "VirtAddr", "VirtSize", "Chars"
    )
    .unwrap();

    for (i, sh) in sections.iter().enumerate() {
        let name = sh.name();
        writeln!(
            out,
            "  {:>4}  {:8}  0x{:08x}    0x{:08x}    0x{:08x}",
            i + 1,
            name,
            sh.virtual_address,
            sh.physical_address_or_virtual_size,
            sh.characteristics.0,
        )
        .unwrap();
    }

    out
}

pub async fn coff_groups_impl(
    server: &PdbMcpServer,
    alias: String,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let groups = match pdb.coff_groups() {
        Ok(g) => g,
        Err(e) => return format!("Error reading COFF groups: {e}"),
    };

    let mut out = String::new();
    writeln!(out, "COFF Groups ({} groups):", groups.vec.len()).unwrap();

    for g in &groups.vec {
        writeln!(
            out,
            "  {} seg:off={} size=0x{:x} chars=0x{:08x}",
            g.name, g.offset_segment, g.size, g.characteristics,
        )
        .unwrap();
    }

    out
}
