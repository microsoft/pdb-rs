//! Checks invariants of symbols and symbol streams

use super::*;
use crate::syms::BlockHeader;
use crate::{diag::Diags, utils::iter::IteratorWithRangesExt};
use anyhow::{bail, Result};
use log::debug;

struct Scope {
    /// byte offset within syms_data of the record which started this scope
    start_offset: u32,

    /// the kind of symbol that started this scope
    kind: SymKind,

    /// Byte offset of the `S_END` record which closes this scope. This value is read from
    /// the record that starts the scope. When we encounter the `S_END` record, we verify
    /// that the offsets match.
    end_offset: Option<u32>,
}

/// Check invariants for a symbol stream.
pub fn check_symbol_stream(
    diags: &mut Diags,
    stream_kind: SymbolStreamKind,
    stream: u32,
    syms_data: &[u8],
) -> Result<()> {
    debug!("Checking symbol stream ({stream_kind:?})");

    let stream_offset = stream_kind.stream_offset() as u32;

    let mut scope_locations: Vec<u32> = Vec::new();
    let mut scopes: Vec<Scope> = Vec::new();

    for (sym_range, sym) in SymIter::new(syms_data).with_ranges() {
        if !diags.wants_error() {
            break;
        }

        let sym_kind = sym.kind;
        let sym_pos = sym_range.start;
        let pos_in_stream = stream_offset + sym_pos as u32;

        if sym_range.len() % 4 != 0 {
            if let Some(e) = diags.error_opt("Record length is not 4-byte aligned") {
                e.stream_at(stream, pos_in_stream);
            }
        }

        if scopes.is_empty() {
            scope_locations.push(sym_pos as u32);
        }

        match sym.kind {
            SymKind::S_LPROC32_DPC_ID | SymKind::S_GPROC32_ID => {
                if let Some(e) = diags.error_opt(format!(
                    "Found symbol using {sym_kind:?}, which is not understood / supported."
                )) {
                    e.stream_at(stream, pos_in_stream);
                }
            }
            _ => {}
        }

        if sym.kind.starts_block() {
            let mut p = Parser::new(sym.data);
            let block: &BlockHeader = p.get()?;

            if let Some(last_scope) = scopes.last() {
                if block.p_parent.get() != last_scope.start_offset + stream_offset {
                    if let Some(e) = diags.error_opt(
                        format!("Found symbol kind {sym_kind:?} embedded in another symbol scope ({:?}), with an incorrect parent index.\n\
                            Index of parent record: 0x{:x}\n\
                            Parent pointer field of this record: 0x{:x}",
                            last_scope.kind,
                            last_scope.start_offset + stream_offset,
                            block.p_parent.get()))
                    {
                        e.stream_at(stream, pos_in_stream);
                    }
                }
            } else {
                // If a symbol starts a root symbol scope, then its parent pointer should be zero.
                if block.p_parent.get() != 0 {
                    if let Some(e) = diags.error_opt(format!(
                        "Found symbol {sym_kind:?} at root scope with a non-zero pparent value"
                    )) {
                        e.stream_at(stream, pos_in_stream);
                    }
                }
            }

            scopes.push(Scope {
                start_offset: sym_pos as u32,
                kind: sym.kind,
                end_offset: Some(block.p_end.get()),
            });
        }

        if sym.kind.ends_scope() {
            let Some(ending_scope) = scopes.pop() else {
                bail!("(at 0x{pos_in_stream:x}) symbol ends a scope, but we're not inside a scope");
            };

            // Verify that the "end" offset in the ending scope points to this S_END record.
            if let Some(expected_end) = ending_scope.end_offset {
                if expected_end != sym_pos as u32 + stream_offset {
                    error!(
                        "Found S_END record, but it was not at the offset that was expected.\n\
                         Expected value: 0x{:x}, actual value: 0x{:x}",
                        expected_end,
                        sym_pos as u32 + stream_offset
                    );
                }
            } else {
                // cannot verify
            }
        }
    }

    if !scopes.is_empty() {
        bail!("Symbol stream ended without closing a symbol scope");
    }

    Ok(())
}
