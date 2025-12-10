use super::*;
use ms_pdb::names::NameIndex;
use ms_pdb::tpi::TypeStreamKind;
use ms_pdb::types::fields::Field;
use ms_pdb::types::primitive::dump_primitive_type_index;
use ms_pdb::types::{ItemId, Leaf, TypeData, TypeIndex, UdtProperties, BUILD_INFO_ARG_NAMES};

#[derive(clap::Parser)]
pub struct DumpTypeStreamOptions {
    /// Skip this many type records before beginning the dump.
    #[arg(long)]
    pub skip: Option<usize>,

    /// Stop after this many records have been dumped.
    #[arg(long)]
    pub max: Option<usize>,

    /// Show a hex dump of each type record.
    #[arg(long)]
    pub show_bytes: bool,

    /// Show the value of `TypeIndex` references.
    #[arg(long)]
    pub show_type_indexes: bool,
}

pub fn dump_type_stream(
    type_stream_kind: TypeStreamKind,
    type_stream: &ms_pdb::tpi::TypeStream<Vec<u8>>, // records to decode and display
    dump_type_index: &mut dyn FnMut(&mut dyn std::fmt::Write, TypeIndex) -> anyhow::Result<()>,
    dump_item: &mut dyn FnMut(&mut dyn std::fmt::Write, ItemId) -> anyhow::Result<()>,
    names: Option<&NamesStream<Vec<u8>>>,
    options: &DumpTypeStreamOptions,
) -> anyhow::Result<()> {
    println!("Type Stream");
    println!("-----------");
    println!();

    let Some(header) = type_stream.header() else {
        println!("Stream is empty (no header)");
        return Ok(());
    };

    println!(
        "type_index_begin = 0x{:08x}",
        type_stream.type_index_begin().0
    );
    println!(
        "type_index_end =   0x{:08x}",
        type_stream.type_index_end().0
    );
    println!(
        "Number of types:   0x{n:08x} {n:8}",
        n = type_stream.num_types()
    );

    println!(
        "Number of hash buckets: {n} 0x{n:x}",
        n = header.num_hash_buckets.get()
    );
    println!("Hash key size: {n} 0x{n:x}", n = header.hash_key_size.get());

    println!("{:#?}", type_stream.header());

    let index_prefix = match type_stream_kind {
        TypeStreamKind::TPI => 'T',
        TypeStreamKind::IPI => 'I',
    };

    let type_index_begin = type_stream.type_index_begin();
    let mut iter = type_stream.iter_type_records().with_ranges();
    let mut next_type_index = type_index_begin;

    if let Some(skip) = options.skip {
        // We have to brute-force the iterator, since there is no way to seek to a specific type record.
        for _ in 0..skip {
            let item = iter.next();
            if item.is_none() {
                break;
            }
            next_type_index.0 += 1;
        }
    }

    let mut num_found: usize = 0;

    let mut out = String::new();

    let type_stream_start = type_stream.type_records_range().start;

    for (record_range, ty) in iter {
        out.clear();

        dump_type_record(
            &mut out,
            dump_type_index,
            dump_item,
            index_prefix,
            names,
            record_range.start + type_stream_start,
            next_type_index,
            ty.kind,
            ty.data,
            options,
        )?;

        print!("{out}");

        next_type_index.0 += 1;

        num_found += 1;
        if let Some(max) = options.max {
            if num_found >= max {
                break;
            }
        }
    }

    Ok(())
}

pub fn dump_type_record(
    out: &mut dyn std::fmt::Write,
    ty_ref_in: &mut dyn FnMut(&mut dyn std::fmt::Write, TypeIndex) -> anyhow::Result<()>,
    dump_item: &mut dyn FnMut(&mut dyn std::fmt::Write, ItemId) -> anyhow::Result<()>,
    index_prefix: char,
    names: Option<&NamesStream<Vec<u8>>>,
    record_offset: usize,
    type_index: TypeIndex,
    kind: Leaf,
    data: &[u8],
    options: &DumpTypeStreamOptions,
) -> anyhow::Result<()> {
    let mut ty_ref = |out: &mut dyn std::fmt::Write, ty: TypeIndex| -> anyhow::Result<()> {
        if options.show_type_indexes {
            write!(out, "{ty:?} ")?;
        }

        ty_ref_in(out, ty)
    };

    write!(
        out,
        "[{record_offset:08x}] {index_prefix}#{:08x} [{:04x}] {kind:?} : ",
        type_index.0, kind.0,
    )?;

    let mut p = Parser::new(data);

    fn out_udt_props(out: &mut dyn std::fmt::Write, props: UdtProperties) -> std::fmt::Result {
        if props.fwdref() {
            out.write_str(" fwdref")?;
        }
        Ok(())
    }

    match TypeData::parse(kind, &mut p)? {
        TypeData::Array(t) => {
            ty_ref(out, t.fixed.element_type.get())?;
            write!(out, "[{}]", t.len)?;
        }

        TypeData::Struct(t) => {
            out_udt_props(out, t.fixed.property.get())?;
            write!(out, " {}", t.name)?;
            let field_list = t.fixed.field_list.get();
            if let Some(unique_name) = t.unique_name {
                if unique_name != t.name {
                    write!(out, " (unique: {unique_name})")?;
                }
            }
            if field_list.0 != 0 {
                write!(out, " fields: ")?;
                ty_ref(out, field_list)?;
            }
        }

        TypeData::Enum(t) => {
            out_udt_props(out, t.fixed.property.get())?;
            write!(out, " {}", t.name)?;
            if let Some(unique_name) = t.unique_name {
                if unique_name != t.name {
                    write!(out, " (unique: {unique_name})")?;
                }
            }
        }

        TypeData::Union(t) => {
            out_udt_props(out, t.fixed.property.get())?;
            write!(out, " {}", t.name)?;
            if let Some(unique_name) = t.unique_name {
                if unique_name != t.name {
                    write!(out, " (unique: {unique_name})")?;
                }
            }
        }

        TypeData::Unknown => {
            write!(out, "<UNKNOWN>")?;
        }

        TypeData::Pointer(t) => {
            let attr = t.fixed.attr();
            if attr.r#const() {
                write!(out, "const ")?;
            }
            if attr.volatile() {
                write!(out, "volatile ")?;
            }
            if attr.unaligned() {
                write!(out, "unaligned ")?;
            }
            write!(out, "* ")?;

            ty_ref(out, t.fixed.ty.get())?;
        }

        TypeData::Modifier(t) => {
            if t.is_const() {
                write!(out, "const ")?;
            }
            if t.is_unaligned() {
                write!(out, "unaligned ")?;
            }
            if t.is_volatile() {
                write!(out, "volatile ")?;
            }
            ty_ref(out, t.underlying_type.get())?;
        }

        TypeData::Bitfield(t) => {
            ty_ref(out, t.underlying_type.get())?;
            write!(out, " : {}", t.length)?;
            if t.position != 0 {
                write!(out, " at bit {}", t.position)?;
            }
        }

        TypeData::MemberFunc(t) => {
            write!(out, "    ret: ")?;
            ty_ref(out, t.return_value.get())?;
            write!(out, " class: ")?;
            ty_ref(out, t.class.get())?;
            if t.this.get().0 != 0 {
                write!(out, " this: ")?;
                ty_ref(out, t.this.get())?;
            }
        }

        TypeData::Proc(t) => {
            write!(out, "ret: ")?;
            ty_ref(out, t.return_value.get())?;
            write!(out, " args: ")?;
            ty_ref(out, t.arg_list.get())?;
        }

        TypeData::VTableShape(t) => {
            write!(out, "num_slots: {}", t.count)?;
        }

        TypeData::FieldList(fields) => {
            writeln!(out)?;
            for field in fields.iter() {
                match field {
                    Field::Member(m) => {
                        write!(out, "    at {} : {} ", m.offset, m.name)?;
                        ty_ref(out, m.ty)?;
                        writeln!(out)?;
                    }
                    Field::Enumerate(en) => {
                        writeln!(out, "    {} = {}", en.name, en.value)?;
                    }
                    Field::Method(m) => {
                        writeln!(out, "    {}() - (method group)", m.name)?;
                    }
                    Field::OneMethod(m) => {
                        writeln!(out, "    {}()", m.name)?;
                    }
                    Field::StaticMember(sm) => writeln!(out, "    static {}", sm.name)?,
                    Field::NestedType(nt) => writeln!(out, "    nested {}", nt.name)?,
                    _ => {
                        writeln!(out, "??")?;
                    }
                }
            }
        }

        TypeData::MethodList(_t) => {}

        TypeData::ArgList(t) => {
            write!(out, "num_args: {}", t.args.len())?;
            for &arg in t.args.iter() {
                write!(out, ", ")?;
                ty_ref(out, arg.get())?;
            }
        }

        TypeData::Alias(t) => {
            write!(out, "{} - ", t.name)?;
            ty_ref(out, t.utype)?;
        }

        TypeData::FuncId(t) => {
            writeln!(out, "name: {:?}", t.name)?;

            if t.fixed.scope.get() != 0 {
                write!(out, "    scope: ")?;
                dump_item(out, t.fixed.scope.get())?;
                writeln!(out)?;
            }

            write!(out, "    func: ")?;
            ty_ref(out, t.fixed.func_type.get())?;
        }

        TypeData::MFuncId(t) => {
            writeln!(out, "name: {:?}", t.name)?;

            let parent = t.fixed.parent_type.get();
            write!(out, "    parent: ")?;
            ty_ref(out, parent)?;
            writeln!(out)?;

            let func = t.fixed.func_type.get();
            write!(out, "    func: ")?;
            ty_ref(out, func)?;
            writeln!(out)?;
        }

        TypeData::StringId(t) => {
            writeln!(out, "name: {:?}", t.name)?;
            write!(out, "    ")?;
            dump_item(out, t.id)?;
            writeln!(out)?;
        }

        TypeData::UdtModSrcLine(t) => {
            let src = NameIndex(t.src.get());
            writeln!(out, " module: {}", t.imod.get())?;

            write!(out, "    ")?;
            ty_ref(out, t.ty.get())?;
            writeln!(out)?;

            let line = t.line.get();
            if let Some(names) = names {
                if let Ok(s) = names.get_string(src) {
                    writeln!(out, "    (line {line:6}) {s}")?;
                } else {
                    writeln!(out, "    (line {line:6}) ?? {src:?}")?;
                }
            }
        }

        TypeData::UdtSrcLine(t) => {
            let src = NameIndex(t.src.get());
            writeln!(out)?;

            write!(out, "    ")?;
            ty_ref(out, t.ty.get())?;
            writeln!(out)?;

            let line = t.line.get();
            if let Some(names) = names {
                if let Ok(s) = names.get_string(src) {
                    writeln!(out, "    (line {line:6}) {s}")?;
                } else {
                    writeln!(out, "    (line {line:6}) ?? {src:?}")?;
                }
            }
        }

        TypeData::SubStrList(t) => {
            writeln!(out, "n = {}", t.ids.len())?;
            for (n, id) in t.ids.iter().enumerate() {
                writeln!(out, "[{n:3}] I#{:08x}", id.get())?;
                dump_item(out, id.get())?;
                writeln!(out)?;
            }
        }

        TypeData::BuildInfo(build_info) => {
            writeln!(out)?;

            for (i, a) in build_info.args.iter().enumerate() {
                if let Some(name) = BUILD_INFO_ARG_NAMES.get(i) {
                    write!(out, "    {name} = ")?;
                } else {
                    write!(out, "    ??{i} = ")?;
                }
                dump_item(out, a.get())?;
                writeln!(out)?;
            }
        }

        TypeData::VFTable(vftable) => {
            if vftable.path.get().0 != 0 {
                write!(out, "path: ")?;
                ty_ref(out, vftable.path.get())?;
                write!(out, " ")?;
            }

            if vftable.root.get().0 != 0 {
                write!(out, "root: ")?;
                ty_ref(out, vftable.root.get())?;
                write!(out, " ")?;
            }
        }
    }

    writeln!(out)?;

    if options.show_bytes {
        write!(out, "{:?}", HexDump::new(data))?;
        writeln!(out)?;
    }

    Ok(())
}

// recursive
pub fn dump_type_index_short(
    out: &mut dyn std::fmt::Write,
    context: &super::sym::DumpSymsContext,
    type_index: TypeIndex,
) -> anyhow::Result<()> {
    if context.type_stream.is_primitive(type_index) {
        dump_primitive_type_index(out, type_index)?;
        return Ok(());
    }

    let type_record = context.type_stream.record(type_index)?;
    let kind = type_record.kind;
    let data = type_record.data;

    if context.show_type_index {
        write!(out, "T#{:08x} ", type_index.0)?;
    }

    write!(out, "{kind:?} : ")?;

    let ty_ref = |out: &mut dyn std::fmt::Write,
                  context: &super::sym::DumpSymsContext,
                  ref_ti: TypeIndex| {
        let _ = dump_type_index_short(out, context, ref_ti);
    };

    let mut p = Parser::new(data);

    match TypeData::parse(kind, &mut p)? {
        TypeData::Array(t) => {
            ty_ref(out, context, t.fixed.element_type.get());
            write!(out, "[{}]", t.len)?;
        }

        TypeData::Struct(t) => write!(out, "{}", t.name)?,
        TypeData::Enum(t) => write!(out, "{}", t.name)?,
        TypeData::Union(t) => write!(out, "{}", t.name)?,
        TypeData::Unknown => write!(out, "<UNKNOWN>")?,

        TypeData::Pointer(t) => {
            let attr = t.fixed.attr();
            if attr.r#const() {
                write!(out, "const ")?;
            }
            if attr.volatile() {
                write!(out, "volatile ")?;
            }
            if attr.unaligned() {
                write!(out, "unaligned ")?;
            }

            ty_ref(out, context, t.fixed.ty.get());
        }

        TypeData::Modifier(t) => {
            if t.is_const() {
                write!(out, "const ")?;
            }
            if t.is_unaligned() {
                write!(out, "unaligned ")?;
            }
            if t.is_volatile() {
                write!(out, "volatile ")?;
            }
            ty_ref(out, context, t.underlying_type.get());
        }

        TypeData::MemberFunc(_t) => {}
        TypeData::Proc(_t) => {}
        TypeData::VTableShape(_t) => {}
        TypeData::FieldList(_t) => {}
        TypeData::MethodList(_t) => {}
        TypeData::ArgList(t) => write!(out, "num_args: {}", t.args.len())?,
        TypeData::Alias(t) => write!(out, "{}", t.name)?,

        TypeData::VFTable(_) => {}

        _ => {
            write!(out, "error: unexpected record kind in TPI stream")?;
        }
    }

    Ok(())
}

// recursive
pub fn dump_item_short(
    out: &mut dyn std::fmt::Write,
    context: &super::sym::DumpSymsContext,
    item: ItemId,
) -> anyhow::Result<()> {
    if item == 0 {
        write!(out, "(nil)")?;
        return Ok(());
    }
    if context.ipi.is_primitive(TypeIndex(item)) {
        write!(out, "(error: item = 0x{item:x})")?;
        return Ok(());
    }

    let item_record = match context.ipi.record(TypeIndex(item)) {
        Ok(r) => r,
        Err(e) => {
            write!(out, "(error: {e:?})")?;
            return Ok(());
        }
    };
    let kind = item_record.kind;
    let data = item_record.data;

    if context.show_type_index {
        write!(out, "I#{item:08x} ")?;
    }

    write!(out, "{kind:?} : ")?;

    let ty_ref = |out: &mut dyn std::fmt::Write,
                  context: &super::sym::DumpSymsContext,
                  ref_ti: TypeIndex| {
        let _ = dump_type_index_short(out, context, ref_ti);
    };

    let mut p = Parser::new(data);

    match TypeData::parse(kind, &mut p)? {
        TypeData::UdtModSrcLine(t) => {
            write!(out, "src: 0x{:08x}, line {}, ", t.src.get(), t.line.get())?;
            ty_ref(out, context, t.ty.get());
        }

        TypeData::UdtSrcLine(t) => {
            write!(out, "src: 0x{:08x}, line {}, ", t.src.get(), t.line.get())?;
            ty_ref(out, context, t.ty.get());
        }

        TypeData::FuncId(t) => write!(out, "{:?}", t.name)?,
        TypeData::MFuncId(t) => write!(out, "{:?}", t.name)?,
        TypeData::StringId(t) => write!(out, "{:?}", t.name)?,
        TypeData::SubStrList(_) => {}
        TypeData::BuildInfo(_) => {}

        _ => {
            write!(out, "error: unexpected record kind in IPI stream")?;
        }
    }

    Ok(())
}
