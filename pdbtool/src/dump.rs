use crate::dump_utils::{HexDump, HexStr};
use anyhow::Result;
use ms_pdb::codeview::parser::Parser;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::dbi::optional_dbg::OptionalDebugHeaderStream;
use ms_pdb::dbi::{DbiSourcesSubstream, DbiStream, ModuleInfo};
use ms_pdb::names::NamesStream;
use ms_pdb::syms::{SymIter, SymKind};
use ms_pdb::tpi::TypeStreamKind;
use ms_pdb::types::TypeIndex;
use ms_pdb::{Pdb, Stream};
use std::fmt::Write;
use std::ops::Range;
use std::path::Path;
use tracing::error;

use self::sym::DumpSymsContext;
use self::types::dump_type_index_short;

mod lines;
mod names;
mod sources;
mod streams;
pub mod sym;
mod types;

#[derive(clap::Parser)]
pub struct DumpOptions {
    /// The PDB to dump
    pub pdb: String,

    #[arg(long)]
    pub lines_like_cvdump: bool,

    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    Names(names::DumpNamesOptions),
    Globals {
        max: Option<usize>,
        skip: Option<usize>,
        /// Show types
        #[arg(short, long)]
        types: bool,
    },
    /// Dump the Type Stream (TPI)
    Tpi(types::DumpTypeStreamOptions),
    /// Dump the Id Stream (TPI)
    Ipi(types::DumpTypeStreamOptions),

    /// Dump DBI header
    Dbi,

    /// Dump DBI Edit-and-Continue Substream
    DbiEnc,

    DbiTypeServerMap,

    /// Global Symbol Index. Loads the GSI and iterates through its hash records. For each one,
    /// finds the symbol record in the GSS and displays it.
    Gsi,
    /// Public Symbol Index. Loads the PSI and iterates through its hash records. For each one,
    /// finds the symbol record in the GSS and displays it.
    Psi,

    Modules(ModulesOptions),

    /// Dump the Stream Directory.
    Streams(streams::StreamsOptions),

    Lines(lines::LinesOptions),

    /// Dump the DBI Stream - Sources substream
    Sources(sources::SourcesOptions),

    SectionMap,

    /// Dump section contributions (quite large!)
    SectionContribs,

    /// Dump the PDB Info Stream
    Pdbi,

    ModuleSymbols(sym::DumpModuleSymbols),

    /// Dump the contents of a stream, or a subsection of it, using a hexadecimal dump format.
    /// By default, this will only show a portion of the stream; use `--len` to increase it.
    Hex {
        stream: String,
        #[arg(long)]
        offset: Option<String>,
        #[arg(long)]
        len: Option<String>,
    },
}

#[derive(clap::Parser)]
pub struct ModulesOptions {
    /// Filter results to those where the object name matches this regex.
    #[arg(long)]
    pub obj: Option<String>,

    /// Display a specific module by module number. This value is zero-based.
    pub module: Option<u32>,
}

pub fn dump_main(options: DumpOptions) -> anyhow::Result<()> {
    let p = ms_pdb::Pdb::open(Path::new(&options.pdb))?;

    let dbi_stream = p.read_dbi_stream()?;

    if options.lines_like_cvdump {
        lines::dump_lines_like_cvdump(&p, &dbi_stream)?;
    }

    match options.subcommand {
        Subcommand::Globals { skip, max, types } => {
            sym::dump_globals(&p, skip, max, false, types)?;
        }

        Subcommand::Names(args) => {
            names::dump_names(&p, args)?;
        }

        Subcommand::Dbi => {
            dump_dbi(&p)?;
        }

        Subcommand::DbiEnc => {
            let enc = dbi_stream.edit_and_continue();
            println!("Edit-and-Continue substream:");
            if enc.is_empty() {
                println!("(empty)");
            } else {
                println!("{}", HexDump::new(enc).max(0x1000));
                let enc_names = NamesStream::parse(enc)?;
                for (range, name) in enc_names.iter().with_ranges() {
                    println!("[{:08x}] {}", range.start, name);
                }
            }
        }

        Subcommand::DbiTypeServerMap => {
            let tsm = dbi_stream.type_server_map();
            if tsm.is_empty() {
                println!("(empty)");
            } else {
                println!("{:?}", HexDump::new(tsm).max(0x1000));
            }
        }

        Subcommand::Pdbi => dump_pdbi(&p)?,

        Subcommand::ModuleSymbols(args) => sym::dump_module_symbols(&p, args)?,

        Subcommand::Tpi(opts) => {
            let type_stream = p.read_type_stream()?;
            let id_stream = p.read_ipi_stream()?;
            let type_dump_syms_context = DumpSymsContext::new(&type_stream, &id_stream);
            types::dump_type_stream(
                TypeStreamKind::TPI,
                &type_stream,
                &mut |out, t| dump_type_index_short(out, &type_dump_syms_context, t),
                &mut |_out, _id| Ok(()),
                None,
                &opts,
            )?;
        }

        Subcommand::Ipi(opts) => {
            let type_stream = p.read_type_stream()?;
            let id_stream = p.read_ipi_stream()?;
            let type_dump_syms_context = DumpSymsContext::new(&type_stream, &id_stream);
            let id_dump_syms_context = DumpSymsContext::new(&id_stream, &id_stream); // TODO: not even remotely right

            let names = p.names()?;
            types::dump_type_stream(
                TypeStreamKind::IPI,
                &id_stream,
                &mut |out, t| dump_type_index_short(out, &type_dump_syms_context, t),
                &mut |out, id| dump_type_index_short(out, &id_dump_syms_context, TypeIndex(id)),
                Some(names),
                &opts,
            )?;
        }
        Subcommand::Gsi => sym::dump_gsi(&p)?,
        Subcommand::Psi => sym::dump_psi(&p)?,
        Subcommand::Lines(args) => lines::dump_lines(args, &p, &dbi_stream)?,
        Subcommand::Streams(args) => streams::dump_streams(&p, args)?,
        Subcommand::Modules(args) => dump_modules(&p, &dbi_stream, args)?,
        Subcommand::Sources(args) => sources::dump_dbi_sources(&dbi_stream, args)?,
        Subcommand::SectionContribs => dump_section_contribs(&dbi_stream)?,
        Subcommand::SectionMap => dump_section_map(&p, &dbi_stream)?,

        Subcommand::Hex {
            stream,
            offset,
            len,
        } => {
            let mut offset = if let Some(offset_str) = &offset {
                str_to_u32(offset_str)? as usize
            } else {
                0
            };

            let mut len = if let Some(len_str) = &len {
                str_to_u32(len_str)? as usize
            } else {
                0x200
            };

            let (stream_index, stream_range_opt) = crate::save::get_stream_index(&p, &stream)?;
            let stream_data = p.read_stream_to_vec(stream_index)?;

            if let Some(r) = stream_range_opt {
                println!("range = {r:?}");
                len = len.min(r.len());
                offset += r.start;
            };

            if let Some(bytes) = stream_data.get(offset..) {
                println!("{:?}", HexDump::new(bytes).max(len).at(offset).header(true));
            } else {
                println!("Offset 0x{offset:x} ({offset}) is out of range for the stream size.");
                println!("Stream length: 0x{len:x} ({len}).", len = stream_data.len());
            }
        }
    }

    Ok(())
}

fn dump_section_contribs(dbi_stream: &DbiStream<Vec<u8>>) -> anyhow::Result<()> {
    println!("*** SECTION CONTRIBUTIONS");
    println!();

    println!("  Imod  Address        Size      Characteristics");

    let section_contribs = dbi_stream.section_contributions()?;
    for contrib in section_contribs.contribs.iter() {
        println!(
            "  {:04X} {:04X}:{:08X}  {:08X}  {:08X}",
            contrib.module_index.get() + 1,
            contrib.section.get(),
            contrib.offset.get(),
            contrib.size.get(),
            contrib.characteristics.get()
        );
    }

    Ok(())
}

fn dump_pdbi(pdb: &Pdb) -> Result<()> {
    let pdbi = pdb.pdbi();

    let binding_key = pdbi.binding_key();
    println!("PDBI version: 0x{0:08x}  {0}", pdbi.version());
    println!();
    println!("Binding key:");
    println!("    Unique ID: {}", binding_key.guid.braced());
    println!("    Age: {}", binding_key.age);
    println!(
        "    symsrv file.ptr path: {:?}{}",
        HexStr::new(binding_key.guid.as_bytes()).packed(),
        binding_key.age.wrapping_sub(1)
    );

    Ok(())
}

fn dump_modules(pdb: &Pdb, dbi: &DbiStream, args: ModulesOptions) -> Result<()> {
    let modules = dbi.modules();

    let mut num_modules: u32 = 0;

    let obj_rx = if let Some(obj_filter) = args.obj.as_ref() {
        Some(regex::bytes::Regex::new(obj_filter)?)
    } else {
        None
    };

    let modules_records_start = dbi.substreams.modules_bytes.start;

    for (module_index, (module_record_range, module)) in modules.iter().with_ranges().enumerate() {
        if let Some(mi) = args.module {
            if module_index != mi as usize {
                continue;
            }
        }

        if let Some(obj_rx) = &obj_rx {
            if !obj_rx.is_match(module.obj_file()) {
                continue;
            }
        }

        println!("Module #{} : {}", module_index, module.module_name());
        println!(
            "    [{:08x} .. {:08x}] Module Info record in DBI Stream",
            modules_records_start + module_record_range.start,
            modules_records_start + module_record_range.end
        );
        println!("    {}", module.obj_file());
        if let Some(stream) = module.stream() {
            println!("    Stream: {}", stream);
            let sym_start = 0;
            let sym_byte_size = module.header().sym_byte_size.get();
            let sym_end = sym_byte_size;
            let c11_byte_size = module.header().c11_byte_size.get();
            let c11_start = sym_byte_size;
            let c11_end = sym_end + c11_byte_size;
            let c13_byte_size = module.header().c13_byte_size.get();
            let c13_start = sym_byte_size + c11_byte_size;
            let c13_end = c13_start + c13_byte_size;
            println!("        [{sym_start:08x} .. {sym_end:08x}] module symbols");
            if c11_byte_size != 0 {
                println!("        [{c11_start:08x} .. {c11_end:08x}] c11 line data");
            }
            if c13_byte_size != 0 {
                println!("        [{c13_start:08x} .. {c13_end:08x}] c13 line data");
            }
            let sym_stream_len = pdb.stream_len(stream);
            if sym_stream_len > c13_byte_size as u64 {
                println!("        [{c13_end:08x} .. {sym_stream_len:08x}] global refs");
            }
        } else {
            println!("    Stream: (none)");
        }

        let h = module.header();
        println!(
            "    section_contr.module_index: {}",
            h.section_contrib.module_index.get()
        );
        println!(
            "    pdb_file_path_name_index: {}",
            h.pdb_file_path_name_index.get()
        );
        println!("    source_file_count: {}", h.source_file_count.get());
        println!(
            "    source_file_name_index: {}",
            h.source_file_name_index.get()
        );

        if h.unused1.get() != 0 {
            println!("    unused1: 0x{0:08x} {0:10}", h.unused1.get());
        }
        if h.unused2.get() != 0 {
            println!("    unused2: 0x{0:08x} {0:10}", h.unused2.get());
        }
        println!();

        num_modules += 1;
    }

    println!("Number of modules found: {num_modules}");

    Ok(())
}

fn dump_section_map(_p: &Pdb, dbi_stream: &DbiStream) -> Result<()> {
    use ms_pdb::dbi::section_map::SectionMapEntryFlags;

    let section_map = dbi_stream.section_map()?;

    println!(
        "Number of entries in section map: {}",
        section_map.entries.len()
    );

    for (i, entry) in section_map.entries.iter().enumerate() {
        println!(
            "  {:6} : section_name {:04x}, class_name {:04x}, offset {:08x}, length {:08x}, flags {:04x} {:?}",
            i,
            entry.section_name.get(),
            entry.class_name.get(),
            entry.offset.get(),
            entry.section_length.get(),
            entry.flags.get(),
            SectionMapEntryFlags::from_bits_truncate(entry.flags.get())
        );
    }

    Ok(())
}

fn str_to_u32(s: &str) -> anyhow::Result<u32> {
    if let Some(after) = s.strip_prefix("0x") {
        Ok(u32::from_str_radix(after, 16)?)
    } else {
        Ok(s.parse()?)
    }
}

fn dump_dbi(pdb: &Pdb) -> Result<()> {
    let header = pdb.dbi_header();

    println!("Signature: 0x{:08x}", header.signature.get());
    println!(
        "Version:   0x{version:08x}  {version}",
        version = header.version.get()
    );
    println!("Age:       0x{age:08x}  {age}", age = header.age.get());
    println!();
    println!("Global Symbols:");
    println!(
        "    Global Symbol Stream (GSS):  {:?}",
        header.global_symbol_stream.get()
    );
    println!(
        "    Global Symbol Index Stream (GSI): {:?}",
        header.global_symbol_index_stream.get()
    );
    println!(
        "    Public Symbol Index Stream (PSI): {:?}",
        header.public_symbol_index_stream.get()
    );

    println!("Substreams:");

    let subs = pdb.dbi_substreams();

    let show_sub = |range: &Range<usize>, name: &str| {
        println!(
            "    [{:08x} .. {:08x}] size 0x{:08x} : {name}",
            range.start,
            range.end,
            range.len()
        );
    };

    show_sub(&subs.modules_bytes, "Modules");
    show_sub(&subs.section_contributions_bytes, "Section Contributions");
    show_sub(&subs.section_map_bytes, "Section Map");
    show_sub(&subs.source_info, "Sources");
    show_sub(&subs.type_server_map, "Type Server Map");
    show_sub(&subs.optional_debug_header_bytes, "Optional Debug Headers");
    show_sub(&subs.edit_and_continue, "Edit-and-Continue");

    Ok(())
}
