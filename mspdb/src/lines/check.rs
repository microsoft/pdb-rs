//! Code for checking invariants of C13 Line data

use super::*;
use crate::diag::Diags;
use anyhow::Result;
use std::collections::HashMap;

/// Checks invariants for C13 Line Data
pub fn check_line_data(
    diags: &mut Diags,
    module_index: usize,
    names: &crate::names::NamesStream<Vec<u8>>,
    sources: &crate::dbi::sources::DbiSourcesSubstream<'_>,
    c13_line_data_bytes: &[u8],
) -> Result<()> {
    if !diags.wants_error() {
        return Ok(());
    }

    let name_offsets_for_module = sources.name_offsets_for_module(module_index)?;

    let c13_line_data = LineData::new(c13_line_data_bytes);

    let checksums_subsection = c13_line_data.find_checksums();
    let has_checksums = checksums_subsection.is_some();
    let mut checksums_table: HashMap<u32, FileChecksum> = HashMap::new();
    if let Some(checksums) = &checksums_subsection {
        // Check that the file names referenced by the checksums are the same file names listed
        // in the DBI Sources Substream.

        for (i, ck) in checksums.iter().enumerate() {
            let name_from_checksum = names.get_string(ck.name())?;

            if let Some(&name_offset) = name_offsets_for_module.get(i) {
                let name_from_sources = sources.get_source_file_name_at(name_offset.get())?;

                if name_from_sources != name_from_checksum {
                    diags.error(format!(
                        "Invalid entry in DEBUG_S_FILE_CHECKSUMS. The file names do not match.\n\
                        File name from DBI Sources Substream: {name_from_sources:?}\n\
                        File name from DEBUG_S_FILE_CHECKSUMS (indirected through /names table): {name_from_checksum:?}"
                    ));
                }
            } else {
                diags.error("Invalid entry in DEBUG_S_FILE_CHECKSUMS. There are more entries in this array than there are in the corresponding section of the DBI Sources Substream.");
                // Stop, because all future iterations of this loop would report the same thing.
                break;
            }
        }

        for (ck_range, ck) in checksums.iter().with_ranges() {
            checksums_table.insert(ck_range.start as u32, ck);
        }
    }

    // Scan through the DEBUG_S_LINES sections and validate the file_offset values.
    // These point into the DEBUG_S_FILE_CHECKSUMS table.

    for subsection in c13_line_data.subsections() {
        match subsection.kind {
            SubsectionKind::LINES => {
                let lines = LinesSubsection::parse(subsection.data)?;
                for block in lines.blocks() {
                    if !diags.wants_error() {
                        break;
                    }

                    let file_index = block.header.file_index.get();

                    if has_checksums {
                        // file_index is a byte offset into the DEBUG_S_FILE_CHECKSUMS table.
                        if let Some(_file_checksum) = checksums_table.get(&file_index) {
                            // Good.
                        } else {
                            diags.error(format!("Invalid block entry in DEBUG_S_LINES. The block has file_index = 0x{file_index},\n\
                            but there is no entry in DEBUG_S_FILE_CHECKSUMS with that file_index."));
                        }
                    } else {
                        diags.error("DEBUG_S_LINES contains a block, but there is no DEBUG_S_FILE_CHECKSUMS subsection.");
                    }
                }
            }

            SubsectionKind::FILE_CHECKSUMS => {}
            SubsectionKind::STRING_TABLE => {
                diags.warning("Module has a DEBUG_S_STRINGTABLE section, which should not be true for a linked module.");
            }

            SubsectionKind::FRAMEDATA
            | SubsectionKind::INLINEELINES
            | SubsectionKind::CROSSSCOPEIMPORTS
            | SubsectionKind::CROSSSCOPEEXPORTS
            | SubsectionKind::IL_LINES
            | SubsectionKind::FUNC_MDTOKEN_MAP
            | SubsectionKind::TYPE_MDTOKEN_MAP
            | SubsectionKind::MERGED_ASSEMBLYINPUT
            | SubsectionKind::COFF_SYMBOL_RVA => {
                // Not sure what to do with this right now, but at least we recognize them.
            }

            SubsectionKind::SYMBOLS => {
                diags.warning("Module has a DEBUG_S_SYMBOLS section, which should not be true for a linked module.");
            }

            unknown_kind => {
                diags.warning(format!("Module has a C13 Line Data subsection whose kind is not recognized. Kind: 0x{:02x}", unknown_kind.0));
            }
        }
    }

    Ok(())
}
