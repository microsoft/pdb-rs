use bstr::BStr;
use ms_pdb::syms::{SymData, SymKind};
use std::fmt::Write;

/// Format a symbol record into a human-readable string.
pub fn format_sym(kind: SymKind, data: &[u8], undecorate: bool) -> String {
    let mut out = String::new();
    write!(out, "{kind:?}: ").unwrap();

    match SymData::parse(kind, data) {
        Ok(sym_data) => {
            format_sym_data(&mut out, kind, &sym_data, undecorate);
        }
        Err(e) => {
            write!(out, "<parse error: {e}>").unwrap();
        }
    }

    out
}

/// Conditionally undecorate a name for display.
fn display_name(name: &BStr, undecorate: bool) -> String {
    let name_str = name.to_string();
    if undecorate {
        crate::undecorate::format_with_undecoration(&name_str)
    } else {
        name_str
    }
}

fn format_sym_data(out: &mut String, _kind: SymKind, sym_data: &SymData, undecorate: bool) {
    match sym_data {
        SymData::Pub(pub_data) => {
            write!(
                out,
                "{} flags=0x{:08x} {}",
                pub_data.fixed.offset_segment,
                pub_data.fixed.flags.get(),
                display_name(pub_data.name, undecorate),
            )
            .unwrap();
        }
        SymData::Udt(udt) => {
            write!(out, "type=0x{:x} {}", udt.type_.0, display_name(udt.name, undecorate)).unwrap();
        }
        SymData::Constant(c) => {
            write!(out, "type=0x{:x} {} = {}", c.type_.0, display_name(c.name, undecorate), c.value).unwrap();
        }
        SymData::Data(d) => {
            write!(
                out,
                "{} type=0x{:x} {}",
                d.header.offset_segment, d.header.type_.0, display_name(d.name, undecorate)
            )
            .unwrap();
        }
        SymData::Proc(p) => {
            write!(
                out,
                "{} type=0x{:x} len={} {}",
                p.fixed.offset_segment,
                p.fixed.proc_type.get().0,
                p.fixed.proc_len.get(),
                display_name(p.name, undecorate),
            )
            .unwrap();
        }
        SymData::RefSym2(r) => {
            write!(
                out,
                "mod={} offset=0x{:x} {}",
                r.header.module_index.get(),
                r.header.symbol_offset.get(),
                display_name(r.name, undecorate),
            )
            .unwrap();
        }
        SymData::ThreadData(ts) => {
            write!(
                out,
                "{} type=0x{:x} {}",
                ts.header.offset_segment, ts.header.type_.0, display_name(ts.name, undecorate)
            )
            .unwrap();
        }
        _ => {
            if let Some(name) = sym_data.name() {
                write!(out, "{}", display_name(name, undecorate)).unwrap();
            } else {
                write!(out, "<...>").unwrap();
            }
        }
    }
}

/// Format a BStr for display, lossy.
pub fn bstr_display(b: &BStr) -> String {
    b.to_string()
}

/// Parse a number that might be hex (0x prefix) or decimal.
pub fn parse_number(s: &str) -> anyhow::Result<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        Ok(u32::from_str_radix(hex, 16)?)
    } else {
        Ok(s.parse()?)
    }
}
