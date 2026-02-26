use crate::server::PdbMcpServer;
use ms_pdb::pdbi;
use std::fmt::Write;

pub async fn pdb_info_impl(server: &PdbMcpServer, alias: String) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;
    let pdbi_stream = pdb.pdbi();
    let binding_key = pdbi_stream.binding_key();

    let container = match pdb.container() {
        ms_pdb::Container::Msf(_) => "MSF (PDB)",
        ms_pdb::Container::Msfz(_) => "MSFZ (PDZ)",
    };

    let version_name = match pdbi_stream.version {
        pdbi::PDBI_VERSION_VC2 => "VC2 (1994)",
        pdbi::PDBI_VERSION_VC4 => "VC4 (1995)",
        pdbi::PDBI_VERSION_VC41 => "VC4.1 (1995)",
        pdbi::PDBI_VERSION_VC50 => "VC5.0 (1996)",
        pdbi::PDBI_VERSION_VC98 => "VC98 (1997)",
        pdbi::PDBI_VERSION_VC70_DEPRECATED => "VC70-deprecated (1999)",
        pdbi::PDBI_VERSION_VC70 => "VC70 (2000)",
        pdbi::PDBI_VERSION_VC80 => "VC80 (2003)",
        pdbi::PDBI_VERSION_VC110 => "VC110 (2009)",
        pdbi::PDBI_VERSION_VC140 => "VC140 (2014)",
        v => return format!("Unknown PDBI version: {v}"),
    };

    let mut out = String::new();
    writeln!(out, "PDB Information Stream (PDBI) — {}", open_pdb.path.display()).unwrap();
    writeln!(out, "  Container:     {container}").unwrap();
    writeln!(out, "  Version:       {version_name} (raw: {})", pdbi_stream.version).unwrap();
    writeln!(out, "  Signature:     0x{:08x}", pdbi_stream.signature).unwrap();
    writeln!(out, "  Age:           {}", pdbi_stream.age).unwrap();
    if let Some(guid) = pdbi_stream.unique_id {
        writeln!(out, "  GUID:          {guid}").unwrap();
    } else {
        writeln!(out, "  GUID:          (none — pre-VC70 PDB)").unwrap();
    }
    writeln!(out, "  Binding Key:   {:?}", binding_key).unwrap();
    writeln!(out, "  Streams:       {}", pdb.num_streams()).unwrap();

    // Features
    if pdbi_stream.features.is_empty() {
        writeln!(out, "  Features:      (none)").unwrap();
    } else {
        let features: Vec<String> = pdbi_stream
            .features
            .iter()
            .map(|f| {
                if *f == pdbi::FeatureCode::MINI_PDB {
                    "MINI_PDB (/DEBUG:FASTLINK)".to_string()
                } else {
                    format!("0x{:08x}", f.0)
                }
            })
            .collect();
        writeln!(out, "  Features:      {}", features.join(", ")).unwrap();
    }

    // Named streams
    let named = pdb.named_streams();
    if named.iter().next().is_some() {
        writeln!(out, "  Named Streams:").unwrap();
        for (name, &stream_idx) in named.iter() {
            let size = pdb.stream_len(stream_idx);
            writeln!(out, "    {name:40} → stream {stream_idx} ({size} bytes)").unwrap();
        }
    } else {
        writeln!(out, "  Named Streams: (none)").unwrap();
    }

    // DBI header basics
    let dbi = pdb.dbi_header();
    writeln!(out, "  Machine:       {:?}", pdb.machine()).unwrap();
    writeln!(out, "  DBI Age:       {}", dbi.age.get()).unwrap();

    out
}

pub async fn pdb_streams_impl(server: &PdbMcpServer, alias: String) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;
    let num_streams = pdb.num_streams();
    let named = pdb.named_streams();

    // Build a reverse map: stream_index → name
    let mut name_map = std::collections::HashMap::new();
    for (name, &idx) in named.iter() {
        name_map.insert(idx, name.clone());
    }

    // Well-known stream names
    let well_known = |idx: u32| -> &str {
        match idx {
            0 => "Old Stream Directory",
            1 => "PDB (PDBI)",
            2 => "TPI",
            3 => "DBI",
            4 => "IPI",
            _ => "",
        }
    };

    let mut out = String::new();
    writeln!(out, "Streams ({num_streams} total):").unwrap();
    writeln!(out, "  {:>5}  {:>12}  {}", "Index", "Size", "Name").unwrap();
    writeln!(out, "  {:>5}  {:>12}  {}", "-----", "----", "----").unwrap();

    for i in 0..num_streams {
        let size = pdb.stream_len(i);
        let name = name_map
            .get(&i)
            .map(|s| s.as_str())
            .unwrap_or(well_known(i));
        writeln!(out, "  {:>5}  {:>12}  {}", i, size, name).unwrap();
    }

    out
}

pub async fn read_stream_impl(
    server: &PdbMcpServer,
    alias: String,
    stream: String,
    offset: Option<u64>,
    length: Option<u64>,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    // Resolve stream by index or name
    let stream_idx: u32 = if let Ok(idx) = stream.parse::<u32>() {
        idx
    } else {
        // Search named streams
        match pdb.named_streams().get(&stream) {
            Some(idx) => idx,
            None => {
                // Try well-known names
                match stream.to_lowercase().as_str() {
                    "pdb" | "pdbi" => 1,
                    "tpi" => 2,
                    "dbi" => 3,
                    "ipi" => 4,
                    _ => return format!("Error: stream '{stream}' not found. Use a numeric index or a named stream name from pdb_streams."),
                }
            }
        }
    };

    if stream_idx >= pdb.num_streams() {
        return format!("Error: stream index {stream_idx} out of range (0..{}).", pdb.num_streams());
    }

    let stream_size = pdb.stream_len(stream_idx);
    let offset = offset.unwrap_or(0);
    let length = length.unwrap_or_else(|| stream_size.saturating_sub(offset).min(4096));

    if offset >= stream_size {
        return format!("Error: offset {offset} is past end of stream (size={stream_size}).");
    }

    let actual_len = length.min(stream_size - offset) as usize;
    // Cap at 64KB to avoid blowing context
    let actual_len = actual_len.min(65536);

    let reader = match pdb.get_stream_reader(stream_idx) {
        Ok(r) => r,
        Err(e) => return format!("Error reading stream {stream_idx}: {e}"),
    };

    let mut buf = vec![0u8; actual_len];
    use ms_pdb::ReadAt;
    match reader.read_at(&mut buf, offset) {
        Ok(bytes_read) => buf.truncate(bytes_read),
        Err(e) => return format!("Error reading stream data: {e}"),
    }

    let mut out = String::new();
    writeln!(out, "Stream {stream_idx}: size={stream_size}, reading offset={offset} length={}", buf.len()).unwrap();

    // Try to interpret as UTF-8 text
    if let Ok(text) = std::str::from_utf8(&buf) {
        if text.chars().all(|c| !c.is_control() || c == '\n' || c == '\r' || c == '\t') {
            writeln!(out, "Content (text, {} bytes):", buf.len()).unwrap();
            out.push_str(text);
            if !text.ends_with('\n') {
                out.push('\n');
            }
            return out;
        }
    }

    // Fall back to hex dump
    writeln!(out, "Content (hex, {} bytes):", buf.len()).unwrap();
    for (i, chunk) in buf.chunks(16).enumerate() {
        let addr = offset + (i * 16) as u64;
        write!(out, "  {:08x}: ", addr).unwrap();
        for (j, &byte) in chunk.iter().enumerate() {
            if j == 8 { write!(out, " ").unwrap(); }
            write!(out, "{:02x} ", byte).unwrap();
        }
        // Pad if short
        for j in chunk.len()..16 {
            if j == 8 { write!(out, " ").unwrap(); }
            write!(out, "   ").unwrap();
        }
        write!(out, " |").unwrap();
        for &byte in chunk {
            let c = if byte.is_ascii_graphic() || byte == b' ' { byte as char } else { '.' };
            write!(out, "{c}").unwrap();
        }
        writeln!(out, "|").unwrap();
    }

    out
}
