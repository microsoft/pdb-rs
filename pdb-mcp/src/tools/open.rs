use crate::server::{OpenPdb, PdbMcpServer};
use std::fmt::Write;
use std::path::Path;

pub async fn open_pdb_impl(server: &PdbMcpServer, path: String, alias: Option<String>) -> String {
    let path_obj = Path::new(&path);

    let alias = alias.unwrap_or_else(|| {
        path_obj
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("pdb")
            .to_string()
    });

    // Check if alias already in use
    {
        let pdbs = server.pdbs.lock().await;
        if pdbs.contains_key(&alias) {
            return format!("Error: alias '{alias}' is already in use. Close it first or use a different alias.");
        }
    }

    let pdb = match ms_pdb::Pdb::open(path_obj) {
        Ok(pdb) => pdb,
        Err(e) => return format!("Error opening PDB: {e}"),
    };

    let mut out = String::new();

    // Gather basic info before moving into the map
    let pdbi = pdb.pdbi();
    let binding_key = pdbi.binding_key();
    let machine = pdb.machine();
    let num_streams = pdb.num_streams();
    let container = match pdb.container() {
        ms_pdb::Container::Msf(_) => "MSF (PDB)",
        ms_pdb::Container::Msfz(_) => "MSFZ (PDZ)",
    };

    writeln!(out, "Opened '{alias}' — {path}").unwrap();
    writeln!(out, "  GUID:       {}", binding_key.guid).unwrap();
    writeln!(out, "  Age:        {}", binding_key.age).unwrap();
    writeln!(out, "  Machine:    {machine:?}").unwrap();
    writeln!(out, "  Container:  {container}").unwrap();
    writeln!(out, "  Streams:    {num_streams}").unwrap();

    let mut pdbs = server.pdbs.lock().await;
    pdbs.insert(
        alias,
        OpenPdb {
            pdb,
            path: path_obj.to_path_buf(),
        },
    );

    out
}

pub async fn close_pdb_impl(server: &PdbMcpServer, alias: String) -> String {
    let mut pdbs = server.pdbs.lock().await;
    if pdbs.remove(&alias).is_some() {
        format!("Closed '{alias}'.")
    } else {
        let available: Vec<_> = pdbs.keys().cloned().collect();
        format!(
            "Error: no open PDB with alias '{alias}'. Open PDBs: {}",
            if available.is_empty() {
                "(none)".to_string()
            } else {
                available.join(", ")
            }
        )
    }
}

pub async fn list_pdbs_impl(server: &PdbMcpServer) -> String {
    let pdbs = server.pdbs.lock().await;
    if pdbs.is_empty() {
        return "No PDB files are currently open.".to_string();
    }

    let mut out = String::new();
    writeln!(out, "Open PDB files:").unwrap();
    for (alias, open_pdb) in pdbs.iter() {
        let machine = open_pdb.pdb.machine();
        writeln!(out, "  {alias}: {} ({machine:?})", open_pdb.path.display()).unwrap();
    }
    out
}
