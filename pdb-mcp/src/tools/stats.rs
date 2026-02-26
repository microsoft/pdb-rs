use crate::server::PdbMcpServer;
use ms_pdb::types::Leaf;
use std::collections::HashMap;
use std::fmt::Write;

pub async fn pdb_stats_impl(
    server: &PdbMcpServer,
    alias: String,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;
    let mut out = String::new();

    writeln!(out, "PDB Statistics — {}", open_pdb.path.display()).unwrap();

    // Stream sizes
    let num_streams = pdb.num_streams();
    let mut total_stream_bytes: u64 = 0;
    for i in 0..num_streams {
        total_stream_bytes += pdb.stream_len(i);
    }
    writeln!(out, "  Total streams:      {num_streams}").unwrap();
    writeln!(out, "  Total stream bytes: {total_stream_bytes}").unwrap();

    // Module count
    if let Ok(modules) = pdb.modules() {
        let count = modules.iter().count();
        writeln!(out, "  Modules:            {count}").unwrap();
    }

    // TPI stats
    if let Ok(tpi) = pdb.read_type_stream() {
        let mut leaf_counts: HashMap<Leaf, usize> = HashMap::new();
        let mut total_types = 0usize;
        for ty in tpi.iter_type_records() {
            *leaf_counts.entry(ty.kind).or_default() += 1;
            total_types += 1;
        }
        writeln!(out, "  TPI records:        {total_types}").unwrap();

        let mut sorted: Vec<_> = leaf_counts.iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(a.1));
        writeln!(out, "  TPI top leaves:").unwrap();
        for (leaf, count) in sorted.iter().take(10) {
            writeln!(out, "    {:?}: {count}", leaf).unwrap();
        }
    }

    // IPI stats
    if let Ok(ipi) = pdb.read_ipi_stream() {
        let mut total_items = 0usize;
        for _ in ipi.iter_type_records() {
            total_items += 1;
        }
        writeln!(out, "  IPI records:        {total_items}").unwrap();
    }

    // GSS stats
    if let Ok(gss) = pdb.gss() {
        let mut total_syms = 0usize;
        for _ in gss.iter_syms() {
            total_syms += 1;
        }
        writeln!(out, "  GSS symbols:        {total_syms}").unwrap();
    }

    out
}
