use crate::server::PdbMcpServer;
use crate::undecorate;
use bstr::BStr;
use ms_pdb::syms::{SymData, SymIter, SymKind};
use ms_pdb::types::{TypeData, TypeIndex};
use std::fmt::Write;

pub async fn get_proc_impl(
    server: &PdbMcpServer,
    alias: String,
    name: Option<String>,
    module: Option<u32>,
    offset: Option<u32>,
    do_undecorate: bool,
    show_params: bool,
    show_locals: bool,
    show_blocks: bool,
    show_inlinees: bool,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    // Resolve the function: either by name (GSI lookup) or by module+offset (direct)
    let (mod_index, sym_offset) = if let Some(name) = &name {
        // GSI lookup → S_PROCREF → get module index and offset
        match resolve_proc_ref(pdb, name) {
            Ok(Some((mi, so))) => (mi, so),
            Ok(None) => return format!("No procedure found with name '{name}'."),
            Err(e) => return format!("Error looking up '{name}': {e}"),
        }
    } else if let (Some(m), Some(o)) = (module, offset) {
        (m, o)
    } else {
        return "Error: provide either 'name' or both 'module' and 'offset'.".to_string();
    };

    // Get the module list to find the module info
    let modules = match pdb.modules() {
        Ok(m) => m,
        Err(e) => return format!("Error reading modules: {e}"),
    };

    let module_info = match modules.iter().nth(mod_index as usize) {
        Some(m) => m,
        None => return format!("Error: module index {mod_index} out of range."),
    };

    // Read the module symbol stream
    let sym_data = match pdb.read_module_symbols(&module_info) {
        Ok(d) => d,
        Err(e) => return format!("Error reading module symbols: {e}"),
    };

    if sym_data.is_empty() {
        return "Module has no symbols.".to_string();
    }

    let sym_bytes = zerocopy::IntoBytes::as_bytes(sym_data.as_slice());

    // Seek to the S_GPROC32/S_LPROC32 at the given offset
    let proc_bytes = match sym_bytes.get(sym_offset as usize..) {
        Some(b) => b,
        None => return format!("Error: offset {sym_offset} is past end of module symbol stream."),
    };

    // Parse the first symbol — should be a proc
    let mut iter = SymIter::new(proc_bytes);
    let Some(proc_sym) = iter.next() else {
        return "Error: no symbol at the given offset.".to_string();
    };

    if !proc_sym.kind.is_proc() {
        return format!(
            "Error: symbol at offset {sym_offset} is {:?}, not a procedure.",
            proc_sym.kind
        );
    }

    let proc_data = match proc_sym.parse_as::<ms_pdb::syms::Proc>() {
        Ok(p) => p,
        Err(e) => return format!("Error parsing procedure symbol: {e}"),
    };

    // Load TPI for type resolution (best-effort)
    let tpi = pdb.read_type_stream().ok();

    // Build the output
    let mut out = String::new();

    // Header: function name
    let raw_name = proc_data.name.to_string();
    if do_undecorate {
        writeln!(out, "{}", undecorate::format_with_undecoration(&raw_name)).unwrap();
    } else {
        writeln!(out, "{raw_name}").unwrap();
    }

    // Basic info (always shown)
    writeln!(out, "  Address:    {}", proc_data.fixed.offset_segment).unwrap();
    writeln!(out, "  Length:     0x{:x} bytes", proc_data.fixed.proc_len.get()).unwrap();
    writeln!(
        out,
        "  Debug:      start=0x{:x} end=0x{:x}",
        proc_data.fixed.debug_start.get(),
        proc_data.fixed.debug_end.get()
    )
    .unwrap();

    let type_index = proc_data.fixed.proc_type.get();
    write!(out, "  Type:       0x{:x}", type_index.0).unwrap();
    if let Some(tpi) = &tpi {
        let type_desc = describe_type_brief(tpi, type_index);
        if !type_desc.is_empty() {
            write!(out, " ({type_desc})").unwrap();
        }
    }
    writeln!(out).unwrap();

    let flags = proc_data.flags();
    if !flags.is_empty() {
        writeln!(out, "  Flags:      {flags:?}").unwrap();
    }

    writeln!(out, "  Module:     [{mod_index}] {}", module_info.module_name).unwrap();

    // Now walk the child symbols within this proc's scope
    if show_params || show_locals || show_blocks || show_inlinees {
        let mut params: Vec<(String, TypeIndex)> = Vec::new();
        let mut locals: Vec<(String, TypeIndex)> = Vec::new();
        let mut inlinees: Vec<(String, u32)> = Vec::new(); // (name_or_id, inlinee ItemId)
        let mut blocks: Vec<(u32, String, String)> = Vec::new(); // (depth, name, addr)
        let mut scope_depth: u32 = 0;

        // Iterate remaining symbols until the matching S_END
        for sym in iter {
            if sym.kind.ends_scope() {
                if scope_depth == 0 {
                    break; // This is the S_END matching our proc
                }
                scope_depth -= 1;
                continue;
            }

            if sym.kind.starts_scope() {
                // Process before incrementing depth
                if show_blocks && sym.kind == SymKind::S_BLOCK32 {
                    if let Ok(block) = sym.parse_as::<ms_pdb::syms::Block>() {
                        blocks.push((
                            scope_depth,
                            block.name.to_string(),
                            format!("{}", block.fixed.offset_segment),
                        ));
                    }
                }

                if show_inlinees
                    && (sym.kind == SymKind::S_INLINESITE || sym.kind == SymKind::S_INLINESITE2)
                {
                    if let Ok(site) = sym.parse_as::<ms_pdb::syms::InlineSite>() {
                        let inlinee_id = site.fixed.inlinee.get();
                        inlinees.push((format!("0x{inlinee_id:x}"), inlinee_id));
                    }
                }

                scope_depth += 1;
                continue;
            }

            // Only collect locals/params at depth 0 (direct children of the proc)
            // and depth > 0 if we're showing locals
            if sym.kind == SymKind::S_LOCAL {
                if let Ok(local) = sym.parse_as::<ms_pdb::syms::Local>() {
                    let is_param = (local.fixed.flags.get() & 1) != 0;
                    let ti = local.fixed.ty.get();
                    let name_str = local.name.to_string();

                    if is_param && show_params {
                        params.push((name_str, ti));
                    } else if !is_param && show_locals {
                        locals.push((name_str, ti));
                    }
                }
            }

            // S_REGREL32 can also indicate params/locals (older style)
            if (show_params || show_locals) && sym.kind == SymKind::S_REGREL32 {
                if let Ok(regrel) = sym.parse_as::<ms_pdb::syms::RegRel>() {
                    let ti = regrel.fixed.ty.get();
                    let name_str = regrel.name.to_string();
                    // Without S_LOCAL flags, we can't distinguish param vs local from S_REGREL32
                    // alone. In modern PDBs, S_LOCAL is used. Collect as locals.
                    if show_locals {
                        locals.push((name_str, ti));
                    }
                }
            }
        }

        // Output params
        if show_params && !params.is_empty() {
            writeln!(out, "  Params ({}):", params.len()).unwrap();
            for (name, ti) in &params {
                let type_str = if let Some(tpi) = &tpi {
                    describe_type_brief(tpi, *ti)
                } else {
                    format!("0x{:x}", ti.0)
                };
                writeln!(out, "    {name:30} : {type_str}").unwrap();
            }
        } else if show_params {
            writeln!(out, "  Params: (none)").unwrap();
        }

        // Output locals
        if show_locals && !locals.is_empty() {
            writeln!(out, "  Locals ({}):", locals.len()).unwrap();
            for (name, ti) in &locals {
                let type_str = if let Some(tpi) = &tpi {
                    describe_type_brief(tpi, *ti)
                } else {
                    format!("0x{:x}", ti.0)
                };
                writeln!(out, "    {name:30} : {type_str}").unwrap();
            }
        }

        // Output inlinees
        if show_inlinees && !inlinees.is_empty() {
            writeln!(out, "  Inlinees ({}):", inlinees.len()).unwrap();
            for (id_str, _item_id) in &inlinees {
                writeln!(out, "    ItemId {id_str}").unwrap();
            }
        }

        // Output blocks
        if show_blocks && !blocks.is_empty() {
            writeln!(out, "  Blocks ({}):", blocks.len()).unwrap();
            for (depth, name, addr) in &blocks {
                let indent = "  ".repeat(*depth as usize);
                let label = if name.is_empty() {
                    addr.clone()
                } else {
                    format!("{name} at {addr}")
                };
                writeln!(out, "    {indent}{label}").unwrap();
            }
        }
    }

    out
}

/// Resolve a function name via GSI → S_PROCREF → (module_index, symbol_offset)
fn resolve_proc_ref(
    pdb: &ms_pdb::Pdb,
    name: &str,
) -> anyhow::Result<Option<(u32, u32)>> {
    let gss = pdb.gss()?;
    let gsi = pdb.gsi()?;

    let sym = match gsi.find_symbol(&gss, BStr::new(name.as_bytes()))? {
        Some(s) => s,
        None => return Ok(None),
    };

    // The GSI should return S_PROCREF or S_LPROCREF for procedures
    match SymData::parse(sym.kind, sym.data)? {
        SymData::RefSym2(ref_sym) => {
            // module_index in RefSym2 is 1-based
            let mod_index = ref_sym.header.module_index.get() as u32;
            let mod_index = if mod_index > 0 { mod_index - 1 } else { 0 };
            let sym_offset = ref_sym.header.symbol_offset.get();
            Ok(Some((mod_index, sym_offset)))
        }
        _ => {
            // Not a proc ref — might be S_UDT, S_CONSTANT, etc.
            Ok(None)
        }
    }
}

/// Brief type description from a TypeIndex, using the TPI stream.
fn describe_type_brief(tpi: &ms_pdb::tpi::TypeStream<Vec<u8>>, ti: TypeIndex) -> String {
    if tpi.is_primitive(ti) {
        // Use the primitive type display
        use ms_pdb::types::primitive::dump_primitive_type_index;
        let mut s = String::new();
        let _ = dump_primitive_type_index(&mut s, ti);
        return s;
    }

    let record = match tpi.record(ti) {
        Ok(r) => r,
        Err(_) => return format!("0x{:x}", ti.0),
    };

    match record.parse() {
        Ok(TypeData::Struct(t)) => t.name.to_string(),
        Ok(TypeData::Enum(t)) => format!("enum {}", t.name),
        Ok(TypeData::Union(t)) => format!("union {}", t.name),
        Ok(TypeData::Alias(t)) => t.name.to_string(),
        Ok(TypeData::Pointer(p)) => {
            let referent = describe_type_brief(tpi, p.fixed.ty.get());
            format!("{referent}*")
        }
        Ok(TypeData::Modifier(m)) => {
            let base = describe_type_brief(tpi, m.underlying_type.get());
            let attrs = m.attributes.get();
            let is_const = (attrs & 1) != 0;
            let is_volatile = (attrs & 2) != 0;
            let mut prefix = String::new();
            if is_const {
                prefix.push_str("const ");
            }
            if is_volatile {
                prefix.push_str("volatile ");
            }
            format!("{prefix}{base}")
        }
        Ok(TypeData::Proc(p)) => {
            let ret = describe_type_brief(tpi, p.return_value.get());
            format!("{ret}(*)({} params)", p.num_params.get())
        }
        Ok(TypeData::Array(a)) => {
            let elem = describe_type_brief(tpi, a.fixed.element_type.get());
            format!("{elem}[{}]", a.len)
        }
        Ok(_) => format!("0x{:x}", ti.0),
        Err(_) => format!("0x{:x}", ti.0),
    }
}
