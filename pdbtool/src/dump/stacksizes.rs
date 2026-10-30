use anyhow::{Context, Result, bail};
use bstr::{BStr, BString};
use ms_pdb::Pdb;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::dbi::DbiStream;
use ms_pdb::syms::{Proc, SymData, SymIter, SymKind};
use std::io::Write;
use tracing::warn;

#[derive(clap::Parser)]
pub struct StackSizesOptions {
    /// Minimum frame size in bytes (inclusive).
    #[arg(long, default_value_t = 512)]
    pub threshold: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct StackSize {
    frame_size: u32,
    name: BString,
}

pub fn dump_stack_sizes(pdb: &Pdb, dbi: &DbiStream, options: StackSizesOptions) -> Result<()> {
    let mut entries = Vec::new();
    let mut modules = dbi.iter_modules();

    for (module_index, module) in modules.by_ref().enumerate() {
        if module.sym_size() == 0 {
            continue;
        }

        let module_stream = match pdb.read_module_stream(&module) {
            Ok(Some(s)) => s,
            Ok(None) => continue,
            Err(e) => {
                warn!("Failed to read module #{module_index} : {e:?}");
                continue;
            }
        };

        let symbols = match module_stream.sym_data() {
            Ok(s) => s,
            Err(e) => {
                warn!("Invalid symbol substream in module #{module_index} : {e:?}");
                continue;
            }
        };

        collect_stack_sizes(symbols, options.threshold, &mut entries)
            .with_context(|| format!("Failed to scan symbols in module #{module_index}"))?;
    }

    write_stack_sizes(entries, &mut std::io::stdout().lock())
}

fn collect_stack_sizes(data: &[u8], threshold: u32, entries: &mut Vec<StackSize>) -> Result<()> {
    let mut scopes: Vec<Option<&BStr>> = Vec::new();

    for (sym_range, symbol) in SymIter::new(data).with_ranges() {
        match symbol.kind {
            SymKind::S_GPROC32 | SymKind::S_LPROC32 => {
                if let Ok(proc) = symbol.parse_as::<Proc>() {
                    scopes.push(Some(proc.name));
                } else {
                    warn!("Found invalid proc record at {sym_range:?}");
                }
            }

            SymKind::S_FRAMEPROC => {
                if let Some(Some(name)) = scopes.last() {
                    if let Ok(SymData::FrameProc(frame)) = symbol.parse() {
                        let frame_size = frame.frame_size();
                        if frame_size >= threshold {
                            entries.push(StackSize {
                                frame_size,
                                name: BString::from(*name),
                            });
                        }
                    } else {
                        warn!("Found invalid S_FRAMEPROC");
                    }
                }
            }

            kind if kind.ends_scope() => {
                if scopes.pop().is_none() {
                    bail!("Unexpected end of symbol scope");
                }
            }

            kind if kind.starts_scope() || kind.is_proc() => scopes.push(None),
            _ => {}
        }
    }

    Ok(())
}

fn write_stack_sizes(mut entries: Vec<StackSize>, out: &mut impl Write) -> Result<()> {
    entries.sort_unstable_by(|a, b| {
        b.frame_size
            .cmp(&a.frame_size)
            .then_with(|| a.name.cmp(&b.name))
    });

    writeln!(out, "Frame size   | Procedure")?;
    writeln!(out, "-------------|----------------------")?;

    for entry in entries {
        writeln!(out, "{:>12} | {}", entry.frame_size, entry.name)?;
    }

    Ok(())
}
