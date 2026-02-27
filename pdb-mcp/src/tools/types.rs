use crate::format;
use crate::server::PdbMcpServer;
use ms_pdb::types::{TypeData, TypeIndex};
use std::fmt::Write;

pub async fn find_type_impl(
    server: &PdbMcpServer,
    alias: String,
    name: String,
    max: Option<usize>,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let tpi = match pdb.read_type_stream() {
        Ok(t) => t,
        Err(e) => return format!("Error reading TPI: {e}"),
    };

    let max = max.unwrap_or(10);
    let lower_name = name.to_lowercase();
    let mut out = String::new();
    let mut found = 0usize;

    let type_index_begin = tpi.type_index_begin();
    let mut current_ti = type_index_begin;

    for ty in tpi.iter_type_records() {
        let ti = current_ti;
        current_ti.0 += 1;

        if let Ok(type_data) = ty.parse() {
            let type_name = type_data.name();
            if let Some(tn) = type_name {
                if tn.to_string().to_lowercase().contains(&lower_name) {
                    found += 1;
                    if found <= max {
                        writeln!(out, "  [0x{:x}] {:?}: {}", ti.0, ty.kind, tn).unwrap();
                        format_type_data_brief(&mut out, &type_data);
                    }
                }
            }
        }
    }

    let mut header = String::new();
    if found == 0 {
        writeln!(header, "No types found matching '{name}'.").unwrap();
    } else if found > max {
        writeln!(
            header,
            "Found {found} types matching '{name}' (showing {max}):"
        )
        .unwrap();
    } else {
        writeln!(header, "Found {found} types matching '{name}':").unwrap();
    }

    format!("{header}{out}")
}

pub async fn dump_type_impl(
    server: &PdbMcpServer,
    alias: String,
    type_index_str: String,
) -> String {
    let pdbs = server.pdbs.lock().await;
    let Some(open_pdb) = pdbs.get(&alias) else {
        return format!("Error: no open PDB with alias '{alias}'.");
    };

    let pdb = &open_pdb.pdb;

    let ti_value = match format::parse_number(&type_index_str) {
        Ok(v) => v,
        Err(e) => return format!("Invalid type index: {e}"),
    };

    let target_ti = TypeIndex(ti_value);

    // Check for primitive type
    if ti_value < 0x1000 {
        return format!(
            "TypeIndex 0x{ti_value:x} is a primitive type. Primitive types are not stored in the TPI stream."
        );
    }

    let tpi = match pdb.read_type_stream() {
        Ok(t) => t,
        Err(e) => return format!("Error reading TPI: {e}"),
    };

    let type_index_begin = tpi.type_index_begin();
    let mut current_ti = type_index_begin;

    for ty in tpi.iter_type_records() {
        if current_ti == target_ti {
            let mut out = String::new();
            writeln!(out, "Type [0x{:x}] leaf={:?}:", current_ti.0, ty.kind).unwrap();

            match ty.parse() {
                Ok(type_data) => {
                    if let Some(name) = type_data.name() {
                        writeln!(out, "  Name: {name}").unwrap();
                    }
                    format_type_data_detail(&mut out, &type_data);
                }
                Err(e) => {
                    writeln!(out, "  <parse error: {e}>").unwrap();
                }
            }

            return out;
        }
        current_ti.0 += 1;
    }

    format!(
        "TypeIndex 0x{ti_value:x} not found (TPI range: 0x{:x}..0x{:x}).",
        type_index_begin.0, current_ti.0
    )
}

fn format_type_data_brief(out: &mut String, type_data: &TypeData) {
    match type_data {
        TypeData::Struct(c) => {
            writeln!(
                out,
                "    size={} fields=0x{:x} props={:?}",
                c.length,
                c.fixed.field_list.get().0,
                c.fixed.property.get()
            )
            .unwrap();
        }
        TypeData::Union(u) => {
            writeln!(
                out,
                "    size={} fields=0x{:x} props={:?}",
                u.length,
                u.fixed.fields.get().0,
                u.fixed.property.get()
            )
            .unwrap();
        }
        TypeData::Enum(e) => {
            writeln!(
                out,
                "    underlying=0x{:x} fields=0x{:x} props={:?}",
                e.fixed.underlying_type.get().0,
                e.fixed.fields.get().0,
                e.fixed.property.get()
            )
            .unwrap();
        }
        TypeData::Proc(p) => {
            writeln!(
                out,
                "    return=0x{:x} params={} args=0x{:x}",
                p.return_value.get().0,
                p.num_params.get(),
                p.arg_list.get().0
            )
            .unwrap();
        }
        TypeData::Pointer(p) => {
            writeln!(out, "    referent=0x{:x}", p.fixed.ty.get().0).unwrap();
        }
        _ => {}
    }
}

fn format_type_data_detail(out: &mut String, type_data: &TypeData) {
    match type_data {
        TypeData::Struct(c) => {
            writeln!(out, "  Kind:       Class/Struct").unwrap();
            writeln!(out, "  Size:       {}", c.length).unwrap();
            writeln!(out, "  Fields:     0x{:x}", c.fixed.field_list.get().0).unwrap();
            writeln!(out, "  Properties: {:?}", c.fixed.property.get()).unwrap();
            if c.fixed.derivation_list.get().0 != 0 {
                writeln!(out, "  Derived:    0x{:x}", c.fixed.derivation_list.get().0).unwrap();
            }
            if c.fixed.vtable_shape.get().0 != 0 {
                writeln!(out, "  VShape:     0x{:x}", c.fixed.vtable_shape.get().0).unwrap();
            }
        }
        TypeData::Union(u) => {
            writeln!(out, "  Kind:       Union").unwrap();
            writeln!(out, "  Size:       {}", u.length).unwrap();
            writeln!(out, "  Fields:     0x{:x}", u.fixed.fields.get().0).unwrap();
            writeln!(out, "  Properties: {:?}", u.fixed.property.get()).unwrap();
        }
        TypeData::Enum(e) => {
            writeln!(out, "  Kind:       Enum").unwrap();
            writeln!(out, "  Underlying: 0x{:x}", e.fixed.underlying_type.get().0).unwrap();
            writeln!(out, "  Fields:     0x{:x}", e.fixed.fields.get().0).unwrap();
            writeln!(out, "  Properties: {:?}", e.fixed.property.get()).unwrap();
        }
        TypeData::Proc(p) => {
            writeln!(out, "  Kind:       Procedure").unwrap();
            writeln!(out, "  Return:     0x{:x}", p.return_value.get().0).unwrap();
            writeln!(out, "  Params:     {}", p.num_params.get()).unwrap();
            writeln!(out, "  ArgList:    0x{:x}", p.arg_list.get().0).unwrap();
        }
        TypeData::MemberFunc(mf) => {
            writeln!(out, "  Kind:       MemberFunction").unwrap();
            writeln!(out, "  Return:     0x{:x}", mf.return_value.get().0).unwrap();
            writeln!(out, "  Class:      0x{:x}", mf.class.get().0).unwrap();
            writeln!(out, "  ThisType:   0x{:x}", mf.this.get().0).unwrap();
            writeln!(out, "  Params:     {}", mf.num_params.get()).unwrap();
            writeln!(out, "  ArgList:    0x{:x}", mf.arg_list.get().0).unwrap();
        }
        TypeData::Pointer(p) => {
            writeln!(out, "  Kind:       Pointer").unwrap();
            writeln!(out, "  Referent:   0x{:x}", p.fixed.ty.get().0).unwrap();
        }
        TypeData::Array(a) => {
            writeln!(out, "  Kind:       Array").unwrap();
            writeln!(out, "  Element:    0x{:x}", a.fixed.element_type.get().0).unwrap();
            writeln!(out, "  Index:      0x{:x}", a.fixed.index_type.get().0).unwrap();
            writeln!(out, "  Size:       {}", a.len).unwrap();
        }
        TypeData::Modifier(m) => {
            writeln!(out, "  Kind:       Modifier").unwrap();
            writeln!(out, "  Modified:   0x{:x}", m.underlying_type.get().0).unwrap();
        }
        _ => {
            writeln!(
                out,
                "  (detailed display not implemented for this leaf kind)"
            )
            .unwrap();
        }
    }
}
