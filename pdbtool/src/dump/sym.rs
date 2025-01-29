use super::*;
use anyhow::bail;
use dump_utils::indent;
use mspdb::syms::SymData;
use mspdb::tpi::TypeStream;
use tracing::warn;

pub fn dump_sym(
    out: &mut String,
    context: &mut DumpSymsContext<'_>,
    record_offset: u32,
    kind: SymKind,
    data: &[u8],
) -> anyhow::Result<()> {
    use super::types::dump_item_short as item_ref;
    use super::types::dump_type_index_short as ty_ref;

    if context.scope_depth == 0 && kind.starts_scope() {
        writeln!(out)?;
    }

    if context.show_record_offsets {
        write!(out, "{:08x} : ", record_offset)?;
    }

    if context.scope_depth > 0 {
        write!(out, "{}", indent(context.scope_depth * 2))?;
    }

    write!(out, "{:?}: ", kind)?;

    match SymData::parse(kind, data)? {
        SymData::Pub(pub_data) => {
            write!(
                out,
                "{}, flags: {:08x}, {}",
                pub_data.fixed.offset_segment,
                pub_data.fixed.flags.get(),
                pub_data.name
            )?;
        }

        SymData::Udt(udt_data) => {
            ty_ref(out, context, udt_data.type_)?;
            write!(out, " {}", udt_data.name)?;
        }

        SymData::Constant(constant_data) => {
            ty_ref(out, context, constant_data.type_)?;
            write!(out, " {} = {}", constant_data.name, constant_data.value)?;
        }

        SymData::ManagedConstant(constant_data) => {
            write!(out, "Token 0x{:x}", constant_data.token)?;
            write!(out, " {} = {}", constant_data.name, constant_data.value)?;
        }

        SymData::RefSym2(sym_ref) => {
            write!(
                out,
                "({}, {:08x}) {}",
                sym_ref.header.module_index.get(),
                sym_ref.header.symbol_offset.get(),
                sym_ref.name
            )?;
        }

        SymData::Data(data) => {
            write!(out, "{} ", data.header.offset_segment,)?;
            ty_ref(out, context, data.header.type_.get())?;
            write!(out, " {}", data.name)?;
        }

        SymData::ThreadData(thread_storage) => {
            write!(
                out,
                "{}, Type: 0x{:04X}, {}",
                thread_storage.header.offset_segment,
                thread_storage.header.type_.0,
                thread_storage.name
            )?;
        }

        SymData::ObjName(obj_name) => {
            write!(out, "sig: 0x{:08x} {}", obj_name.signature, obj_name.name)?;
        }

        SymData::Compile3(compile3) => {
            write!(out, "{}", compile3.name)?;
        }

        SymData::Proc(proc) => {
            write!(
                out,
                "{} ..+ 0x{:x}, ",
                proc.fixed.offset_segment, proc.fixed.proc_len
            )?;
            ty_ref(out, context, proc.fixed.proc_type.get())?;
            write!(out, " {}", proc.name)?;
        }

        SymData::ManagedProc(proc) => {
            write!(out, "Token 0x{:x} {}", proc.fixed.token.get(), proc.name)?;
        }

        SymData::End => {}

        SymData::Unknown => {
            write!(out, "Unknown")?;
        }

        SymData::Annotation(ann) => {
            writeln!(out, "{}", ann.fixed.offset)?;
            for s in ann.iter_strings() {
                writeln!(out, "    {}", s)?;
            }
        }

        SymData::FrameProc(_) => {}

        SymData::RegRel(reg_rel) => {
            write!(
                out,
                "reg(0x{:x})+0x{:x}, ",
                reg_rel.fixed.register.get(),
                reg_rel.fixed.offset.get()
            )?;
            ty_ref(out, context, reg_rel.fixed.ty.get())?;
            write!(out, " {}", reg_rel.name)?;
        }

        SymData::Block(block) => {
            write!(out, "length: 0x{:x}", block.fixed.length.get())?;

            if !block.name.is_empty() {
                write!(out, " name: {}", block.name)?;
            }
        }

        SymData::Local(local) => {
            ty_ref(out, context, local.fixed.ty.get())?;
            write!(out, " {}", local.name)?;
        }

        SymData::DefRangeFramePointerRel(def_range) => {
            write!(
                out,
                "bp+ 0x{:x}, {} ..+ 0x{:x}",
                def_range.fixed.offset_to_frame_pointer,
                def_range.fixed.range.start,
                def_range.fixed.range.range_size.get()
            )?;
            if !def_range.gaps.is_empty() {
                write!(out, ", num_gaps: {}", def_range.gaps.len())?;
            }
        }

        SymData::Trampoline(_) => {}

        SymData::UsingNamespace(ns) => {
            write!(out, "using {}", ns.namespace)?;
        }

        SymData::BuildInfo(b) => {
            item_ref(out, context, b.item)?;
        }

        SymData::InlineSite(site) => {
            item_ref(out, context, site.fixed.inlinee.get())?;
        }

        SymData::InlineSite2(site) => {
            item_ref(out, context, site.fixed.inlinee.get())?;
        }

        SymData::InlineSiteEnd => {}

        SymData::DefRangeRegister(r) => {
            write!(out, "register: 0x{:x}", r.fixed.reg)?;
        }

        SymData::DefRangeRegisterRel(r) => {
            write!(
                out,
                "base register: 0x{:x}, base pointer offset: {}",
                r.fixed.base_reg, r.fixed.base_pointer_offset
            )?;
        }

        SymData::DefRangeSubFieldRegister(_) => {}

        SymData::DefRangeFramePointerRelFullScope(r) => {
            write!(out, "frame pointer offset: {}", r.frame_pointer_offset)?;
        }

        SymData::Label(label) => {
            write!(out, "{} : {}", label.fixed.offset_segment, label.name)?;
        }

        SymData::FunctionList(funcs) => {
            if !funcs.funcs.is_empty() {
                writeln!(out)?;
                for f in funcs.funcs.iter() {
                    item_ref(out, context, f.get())?;
                    writeln!(out)?;
                }
            }
        }

        SymData::FrameCookie(_) => {}

        SymData::CallSiteInfo(site) => {
            write!(out, "{} ", site.offset)?;
            ty_ref(out, context, site.func_type.get())?;
        }

        SymData::HeapAllocSite(site) => {
            write!(out, "{} ", site.offset)?;
            ty_ref(out, context, site.func_type.get())?;
        }
    }

    writeln!(out)?;

    if kind.starts_scope() {
        context.scope_depth += 1;
    }

    if kind.ends_scope() {
        if context.scope_depth > 0 {
            context.scope_depth -= 1;
        } else {
            warn!("scope depth is mismatched");
        }
    }

    Ok(())
}

pub struct DumpSymsContext<'a> {
    pub scope_depth: u32,
    pub type_stream: &'a TypeStream<Vec<u8>>,
    pub show_record_offsets: bool,
    pub show_type_index: bool,
    pub ipi: &'a TypeStream<Vec<u8>>,
}

impl<'a> DumpSymsContext<'a> {
    pub fn new(type_stream: &'a TypeStream<Vec<u8>>, ipi: &'a TypeStream<Vec<u8>>) -> Self {
        Self {
            scope_depth: 0,
            type_stream,
            show_record_offsets: true,
            show_type_index: false,
            ipi,
        }
    }
}

pub fn dump_globals(
    p: &Pdb,
    skip_opt: Option<usize>,
    max_opt: Option<usize>,
    show_bytes: bool,
) -> anyhow::Result<()> {
    println!("Global symbols:");
    let gss = p.gss()?;
    let tpi = p.read_type_stream()?;
    let ipi = p.read_ipi_stream()?;
    dump_symbol_stream(
        &tpi,
        &ipi,
        &gss.stream_data,
        skip_opt,
        max_opt,
        0,
        show_bytes,
    )?;
    Ok(())
}

pub fn dump_symbol_stream(
    type_stream: &TypeStream<Vec<u8>>,
    ipi: &TypeStream<Vec<u8>>,
    symbol_records: &[u8],
    skip_opt: Option<usize>,
    max_opt: Option<usize>,
    stream_offset: u32,
    show_bytes: bool,
) -> anyhow::Result<()> {
    let mut iter = SymIter::new(symbol_records).with_ranges();

    // We have to manually decode all the records that we are skipping;
    // there is no index structure.
    if let Some(skip) = skip_opt {
        for _ in 0..skip {
            if iter.next().is_none() {
                break;
            }
        }
    }

    let mut num_found = 0;
    let mut out = String::new();
    let mut context = DumpSymsContext::new(type_stream, ipi);

    for (record_range, sym) in iter {
        out.clear();

        dump_sym(
            &mut out,
            &mut context,
            stream_offset + record_range.start as u32,
            sym.kind,
            sym.data,
        )?;
        print!("{}", out);

        if show_bytes {
            let record_bytes = &symbol_records[record_range.clone()];
            println!(
                "{:?}",
                HexDump::new(record_bytes).at(stream_offset as usize + record_range.start)
            );
        }

        num_found += 1;
        if let Some(max) = max_opt {
            if num_found >= max {
                break;
            }
        }
    }

    Ok(())
}

/// Displays the symbols for a specific module.
#[derive(clap::Parser, Debug)]
pub struct DumpModuleSymbols {
    /// The module to dump
    pub module_index: u32,

    /// Skip this many symbol records before beginning the dump.
    #[arg(long)]
    pub skip: Option<usize>,

    /// Stop after this many symbol records have been displayed.
    #[arg(long)]
    pub max: Option<usize>,

    /// Dump the hex bytes of each symbol record.
    #[arg(long)]
    pub bytes: bool,

    /// Show the contents of the Global Refs section.
    #[arg(long)]
    pub global_refs: bool,
}

pub fn dump_module_symbols(pdb: &Pdb, options: DumpModuleSymbols) -> anyhow::Result<()> {
    let dbi = pdb.read_dbi_stream()?;

    let Some(module) = dbi.iter_modules().nth(options.module_index as usize) else {
        bail!(
            "Could not find a module with index #{}",
            options.module_index
        );
    };

    let Some(module_stream) = pdb.read_module_stream(&module)? else {
        bail!("Module does not have a module stream (no symbols for module)");
    };

    let tpi = pdb.read_type_stream()?;
    let ipi = pdb.read_ipi_stream()?;

    dump_symbol_stream(
        &tpi,
        &ipi,
        module_stream.sym_data(),
        options.skip,
        options.max,
        4,
        options.bytes,
    )?;

    println!();

    if options.global_refs {
        println!("Global Refs");
        println!("-----------");
        println!();

        let module_global_refs = module_stream.global_refs();
        if !module_global_refs.is_empty() {
            let gss = pdb.gss()?;

            let mut out = String::new();
            let mut context = DumpSymsContext::new(&tpi, &ipi);

            for &global_ref in module_global_refs.iter() {
                let global_ref = global_ref.get();
                // global_ref is an index into the GSS

                let global_sym = gss.get_sym_at(global_ref)?;
                dump_sym(
                    &mut out,
                    &mut context,
                    global_ref,
                    global_sym.kind,
                    global_sym.data,
                )?;
                print!("{}", out);
            }
        } else {
            println!("(none)");
        }
    }

    Ok(())
}

pub fn dump_gsi(p: &Pdb) -> Result<()> {
    let gsi = p.gsi()?;
    let gss = p.gss()?;
    let tpi = p.read_type_stream()?;
    let ipi = p.read_ipi_stream()?;

    println!("*** GLOBALS");
    println!();

    let mut context = DumpSymsContext::new(&tpi, &ipi);

    let mut out = String::new();
    for sym in gsi.names().iter(gss) {
        out.clear();
        // TODO: show the correct record offset, instead of 0
        dump_sym(&mut out, &mut context, 0, sym.kind, sym.data)?;
        println!("{}", out);
    }

    println!();

    Ok(())
}

pub fn dump_psi(p: &Pdb) -> Result<()> {
    let psi = p.read_psi()?;
    let gss = p.gss()?;
    let tpi = p.read_type_stream()?;
    let ipi = p.read_ipi_stream()?;

    println!("*** PUBLICS");
    println!();

    let mut context = DumpSymsContext::new(&tpi, &ipi);

    let mut out = String::new();
    for sym in psi.names().iter(gss) {
        out.clear();
        // TODO: show the correct record offset, instead of 0
        dump_sym(&mut out, &mut context, 0, sym.kind, sym.data)?;
        println!("{}", out);
    }

    println!();

    Ok(())
}
