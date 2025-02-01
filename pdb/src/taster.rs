//! Determines whether a given file header is a PDB/MSF file, PDBZ/MSFZ file, or a Portable PDB file.

use sync_file::ReadAt;

/// Enumerates the kind of PDBs files that are recognized.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum Flavor {
    /// An ordinary PDB file.
    Pdb,
    /// A compressed PDB (PDZ) file.
    Pdz,
    /// A "Portable PDB" file.
    PortablePdb,
}

/// Determines whether a given file header is a PDB/MSF file, PDBZ/MSFZ file, or a Portable PDB file.
pub fn what_flavor<F: ReadAt>(f: &F) -> Result<Option<Flavor>, std::io::Error> {
    let mut header = [0u8; 0x100];
    let _n = f.read_at(&mut header, 0)?;
    if ms_pdb_msf::is_file_header_msf(&header) {
        Ok(Some(Flavor::Pdb))
    } else if ms_pdb_msfz::is_header_msfz(&header) {
        Ok(Some(Flavor::Pdz))
    } else if is_header_portable_pdb(&header) {
        Ok(Some(Flavor::PortablePdb))
    } else {
        Ok(None)
    }
}

fn is_header_portable_pdb(header: &[u8]) -> bool {
    header.len() >= 24 && header[16..24] == *b"PDB v1.0"
}
