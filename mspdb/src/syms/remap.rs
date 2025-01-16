//! Algorithm for remapping type indexes found in symbol streams.

use crate::parser::ParserMut;
use crate::syms::{iter_one_sym_type_refs_mut, SymIterMut, SymKind};
use crate::types::{TypeIndex, TypeIndexLe};
use anyhow::bail;
use dump_utils::HexDump;
use log::{debug, error, warn};
use zerocopy::U32;

use super::RefSym2Fixed;

/// Scans through a symbol stream and edits all `TypeIndex` values that are found in the symbols.
/// Each `TypeIndex` is remapped using `type_map`.
pub fn remap_type_indexes_in_symbol_stream(
    syms_data: &mut [u8],
    description: &str,
    type_index_begin: TypeIndex,
    type_map: &[TypeIndex],
) -> anyhow::Result<()> {
    let mut num_errors: u64 = 0;
    let mut num_types_remapped: u64 = 0;
    let mut num_ipi_type_refs_ignored: u64 = 0;
    let mut num_bogus_local_type_refs_ignored: u64 = 0;
    let mut num_syms_found: u32 = 0;
    let mut sym_data_buffer: Vec<u8> = Vec::with_capacity(0x1000);

    for sym in SymIterMut::new(syms_data) {
        num_syms_found += 1;

        // TODO: This copy is only here for diagnostics
        sym_data_buffer.clear();
        sym_data_buffer.extend_from_slice(sym.data);

        iter_one_sym_type_refs_mut(sym.kind, sym.data, |type_index: &mut TypeIndexLe| {
            let ti: TypeIndex = type_index.get();
            if ti < type_index_begin {
                // No need to remap primitives and nil.
                return;
            }

            // TypeIndex with high bit set refer to records in the IPI stream.
            // Records in the IPI stream do not contain type references to each other,
            // so we ignore them for now.
            if ti.0 >= 0x8000_0000 {
                if sym.kind == SymKind::S_LOCAL {
                    num_bogus_local_type_refs_ignored += 1;
                    return;
                }

                num_ipi_type_refs_ignored += 1;
                if num_ipi_type_refs_ignored < 5 {
                    warn!(
                        "Failed to remap TypeIndex (because IPI).  TypeIndex = 0x{:x}, kind: {:?}\n{:?}",
                        ti.0,
                        sym.kind,
                        HexDump::new(&sym_data_buffer)
                    );
                }
                return;
            }

            let ti_rel = ti.0 - type_index_begin.0;
            if let Some(remapped) = type_map.get(ti_rel as usize) {
                *type_index = TypeIndexLe(U32::from(remapped.0));
                num_types_remapped += 1;
            } else {
                if num_errors < 20 {
                    error!(
                        "Failed to remap TypeIndex.  TypeIndex = 0x{:x}, kind: {:?}\n{:?}",
                        ti.0,
                        sym.kind,
                        HexDump::new(&sym_data_buffer)
                    );
                }
                num_errors += 1;
            }
        })?;
    }

    debug!("Finished remapping symbol stream.");
    debug!("Number of symbols found: {num_syms_found}");
    debug!(
        "Number of types remapped in symbol stream: {}",
        num_types_remapped
    );

    if num_errors != 0 {
        error!("Number of times failed to remap symbol because a type index was out of range: {num_errors}");
        bail!("Failed to remap at least one type within a symbol stream");
    }

    if false {
        if num_ipi_type_refs_ignored != 0 {
            warn!("[{description}] Number of TypeIndex values that were ignored because they point into IPI: {num_ipi_type_refs_ignored}");
        }
    }

    if num_bogus_local_type_refs_ignored != 0 {
        debug!("[{description}] Number of TypeIndex values that were ignored because they look like bogus S_LOCAL records: {num_bogus_local_type_refs_ignored}");
    }

    Ok(())
}

/// Iterates the symbols in `symbols` and remaps any `module_index` field using `module_index_mapping`.
#[allow(dead_code)]
pub(crate) fn remap_module_indexes_in_symbol_stream<F>(
    symbols: &mut [u8],
    mut module_index_mapping: F,
) -> anyhow::Result<()>
where
    F: FnMut(u16) -> u16,
{
    for sym in SymIterMut::new(symbols) {
        let mut p = ParserMut::new(sym.data);

        match sym.kind {
            SymKind::S_PROCREF
            | SymKind::S_LPROCREF
            | SymKind::S_TOKENREF
            | SymKind::S_ANNOTATIONREF => {
                // These symbol records all use the RefSym2 layout. There is a module_index field in
                // the RefSym2 header and we do not need the variable-length tail.
                let header: &mut RefSym2Fixed = p.get_mut()?;

                // module_index is 1-based.
                let old = header.module_index.get();
                if old == 0 {
                    bail!("{:?} symbol contains a module_index field that is zero (which is not allowed)", sym.kind);
                }
                let new = module_index_mapping(old - 1) + 1;
                header.module_index = new.into();
            }

            _ => {}
        }
    }

    Ok(())
}
