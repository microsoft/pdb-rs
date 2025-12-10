use ms_pdb::coff::SectionCharacteristics;
use ms_pdb::dbi::SectionContributionsSubstream;
use ms_pdb::syms::{Data, Proc};
use tracing::warn;
use zerocopy::IntoBytes;

use crate::dump::sym::dump_sym;

use super::*;

pub(crate) fn dump_coff_groups(pdb: &Pdb) -> anyhow::Result<()> {
    let coff_groups = pdb.coff_groups()?;

    println!("COFF groups:");
    println!();

    for (i, group) in coff_groups.vec.iter().enumerate() {
        println!(
            "  [{i:4}]  {off_seg} + {size:08x}, {char:08x} : {name:<16}",
            off_seg = group.offset_segment,
            size = group.size,
            char = group.characteristics,
            name = group.name
        );
    }

    Ok(())
}

static SECTION_DESCRIPTIONS: &[(&str, &str)] = &[
    (".text", "executable code"),
    (".idata", "import tables"),
    (".rdata", "read-only data"),
    (".data", "read-write data"),
    (".pdata", "procedure description tables"),
    (".xdata", "exception unwinding data"),
];

fn section_description(name: &str) -> Option<&'static str> {
    SECTION_DESCRIPTIONS
        .iter()
        .find_map(|&(n, d)| if n == name { Some(d) } else { None })
}

/// Descriptions for COFF groups
///
/// # References
/// * [CRT initialization](https://learn.microsoft.com/en-us/cpp/c-runtime-library/crt-initialization?view=msvc-170)
static COFF_GROUP_DESCRIPTIONS: &[(&str, &str)] = &[
    // Keep these sorted
    (".00cfg", "Control-Flow Guard (CFG)"),
    (".CRT$XCA", "(__xc_a) C++ initializer function list: start"),
    (".CRT$XCAA", "pre-C++ initializers"),
    (".CRT$XCU", "debug code masquerading as CRT code"),
    (".CRT$XCU65534", ""),
    (".CRT$XCU65535", ""),
    (".CRT$XCZ", "(__xc_z) C++ initializer function list: end"),
    (".CRT$XDA", "(__xd_a) TLS initializer function list: start"),
    (".CRT$XDZ", "(__xd_z) TLS initializer function list: end"),
    (".CRT$XIA", "(__xi_a) C initializer function list: start"),
    (".CRT$XIAA", "pre-C initializers"),
    (".CRT$XIAB", "PGO initializers"),
    (".CRT$XIAC", "post-PGO initializers"),
    (".CRT$XIZ", "(__xi_z) C initializer function list: end"),
    (".CRT$XLA", "loader TLS callback list: start"),
    (".CRT$XLB", "(__zl_a) pointer to TLS callback array"),
    (".CRT$XLC", "(__xl_c) TLS initializers"),
    (".CRT$XLD", "(__xl_d) TLS destructors"),
    (".CRT$XLZ", "(__xl_z) loader TLS callback list: end"),
    (".CRT$XPA", "(__xp_a) C pre-terminator list: start"),
    (".CRT$XPTZ65535", ""),
    (".CRT$XPZ", "(__xp_z) C pre-terminator list: end"),
    (".CRT$XTA", "(__xt_a) terminator function list: start"),
    (".CRT$XTZ", "(__xt_z) terminator function list: end"),
    (".edata", ""),
    (".gehcont", "EHCONT guard"),
    (".gehcont$y", "EHCONT guard target"),
    (".gfids$y", "relates to Control Flow Guard (CFG)"),
    (".idata$2", "import descriptors"),
    (".idata$3", "import descriptors null terminator"),
    (".idata$4", "Import Name Table (INT)"),
    (".idata$5", "Import Address Table (IAT)"),
    (".idata$6", "import data strings"),
    (".rdata$CastGuardVftablesA", ""),
    (".rdata$CastGuardVftablesC", ""),
    (".rdata$T", ""),
    (".rdata$r", "RTTI read-only data"),
    (
        ".rdata$voltmd",
        "(__volatile_metadata) volatile metadata for CFG",
    ),
    (".rdata$zzzdbg", "read-only data 'dead' from PGO training"),
    (".rtc$IAA", "run-time checks (RTC) initializer list: start"),
    (".rtc$IZZ", "run-time checks (RTC) initializer list: end"),
    (".rtc$TAA", "run-time checks (RTC) terminator list: start"),
    (".rtc$TZZ", "run-time checks (RTC) terminator list: end"),
    (".text$di", ""),
    (".text$mn", "\"main\" code"),
    (".text$mn$00", ""),
    (".text$unlikely", "code believed to be cold"),
    (".text$x", "exception unwinding funclets (__finally, etc.)"),
    (".text$yd", "dynamic atexit destructors"),
    (".tls", "thread-local storage"),
    (".tls$", "thread-local storage"),
    (".tls$ZZZ", ""),
    (".xdata", ""),
];

#[test]
fn test_coff_group_descriptions_sorted() {
    for w in COFF_GROUP_DESCRIPTIONS.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "group descriptions should be sorted: {} >= {}",
            w[0].0,
            w[1].0
        );
    }
}

fn coff_group_description(name: &str) -> Option<&'static str> {
    if let Ok(i) = COFF_GROUP_DESCRIPTIONS.binary_search_by(|&(n, _)| n.cmp(name)) {
        Some(COFF_GROUP_DESCRIPTIONS[i].1)
    } else {
        None
    }
}

/// A helper type for showing read/write/execute bits from section characteristics.
struct Rwx(SectionCharacteristics);

impl core::fmt::Display for Rwx {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.write_char(if self.0.is_read() { 'r' } else { '-' })?;
        f.write_char(if self.0.is_write() { 'w' } else { '-' })?;
        f.write_char(if self.0.is_exec() { 'x' } else { '-' })?;
        Ok(())
    }
}

#[derive(clap::Parser)]
pub struct DumpSectionsOptions {
    /// Show COFF groups (subsections within sections)
    #[arg(long)]
    pub groups: bool,

    /// Show section contributions, nested within sections. If `--groups` is also specified, then
    /// section contributions will be nested within COFF groups.
    #[arg(long)]
    pub contribs: bool,
}

#[derive(PartialOrd, Ord, Eq, PartialEq)]
struct ModuleSymEntry {
    off_seg: OffsetSegment,
    module_index: u16,
    /// The byte offset of this symbol record within the module's symbol stream.
    /// This includes the 4-byte offset, i.e. if this is the very first record in the stream
    /// then `byte_offset == 4`.
    byte_offset: u32,
}

pub(crate) fn dump_sections(pdb: &Pdb, options: DumpSectionsOptions) -> anyhow::Result<()> {
    let section_headers = pdb.section_headers()?;

    let modules = pdb.modules()?;
    let modules: Vec<ModuleInfo<'_>> = modules.iter().collect();

    // Read modules for all symbols.
    let mut module_symbols: Vec<Vec<u32>> = Vec::with_capacity(modules.len());
    for module in modules.iter() {
        let syms = pdb.read_module_symbols(module)?;
        module_symbols.push(syms);
    }

    // Scan the symbols for each module. For those symbols that have a [section:offset],
    // add an entry to this table.
    let mut indexed_module_syms: Vec<ModuleSymEntry> = Vec::new();

    for (module_index, module_syms) in module_symbols.iter().enumerate() {
        let mut syms_iter = SymIter::new(module_syms.as_bytes()).with_ranges();
        syms_iter.inner_mut().skip_module_prefix();

        for (range, sym) in syms_iter {
            let off_seg: OffsetSegment;
            match sym.kind {
                SymKind::S_GPROC32 | SymKind::S_LPROC32 => match sym.parse_as::<Proc>() {
                    Ok(proc) => off_seg = proc.fixed.offset_segment,
                    Err(_) => {
                        warn!(
                            module_index,
                            symbol_offset = range.start,
                            "failed to decode symbol record"
                        );
                        continue;
                    }
                },
                SymKind::S_GDATA32 | SymKind::S_LDATA32 => match sym.parse_as::<Data>() {
                    Ok(proc) => off_seg = proc.header.offset_segment,
                    Err(_) => {
                        warn!(
                            module_index,
                            symbol_offset = range.start,
                            "failed to decode symbol record"
                        );
                        continue;
                    }
                },
                _ => continue,
            }

            indexed_module_syms.push(ModuleSymEntry {
                off_seg,
                byte_offset: range.start as u32,
                module_index: module_index as u16,
            });
        }
    }

    // Sort the indexed symbols by section, then offset
    indexed_module_syms.sort_unstable_by_key(|ms| ms.off_seg.as_u64());

    let mut indexed_module_syms_iter = indexed_module_syms.iter().peekable();

    // The COFF groups should be sorted by offset_segment.
    let coff_groups = if options.groups {
        Some(pdb.coff_groups()?)
    } else {
        None
    };
    let mut groups_iter = coff_groups.map(|groups| groups.vec.iter().peekable());

    // Section contribs should also be sorted by offset_segment.
    let section_contribs = if options.contribs {
        Some(pdb.read_section_contributions()?)
    } else {
        None
    };
    let contribs = if let Some(ref contribs_bytes) = section_contribs {
        SectionContributionsSubstream::parse(contribs_bytes.as_bytes()).ok()
    } else {
        None
    };
    let mut contribs_iter = contribs.map(|c| c.contribs.iter().peekable());

    let tpi = pdb.read_type_stream()?;
    let ipi = pdb.read_ipi_stream()?;
    let mut dump_syms_context = DumpSymsContext::new(pdb.arch()?, &tpi, &ipi);
    dump_syms_context.show_record_offsets = false;

    // (group_offset, group_size) describe the entire section or COFF group (subsection) that we
    // are dumping.
    // section_rva is the rva of the beginning of the entire section, not the group, so you'll need
    // to add group_offset to section_rva to get the group_rva.
    let mut show_contribs_in = |section_num: u16,
                                section_rva: u32,
                                group_offset: u32,
                                group_size: u32| {
        let Some(ref mut contribs_iter) = contribs_iter else {
            return;
        };

        let Some(group_offset_end) = group_offset.checked_add(group_size) else {
            warn!("group offset / size exceed limits");
            return;
        };

        // Walk through the contrib records for this COFF group (which is potentially the entire
        // section). For each contrib record that we find, also search the per-module symbols
        // for symbols that lie within the contribution.

        while let Some(contrib) = contribs_iter.peek() {
            let contrib_section = contrib.section.get();

            // Discard records that will never match. In theory we should never have any such records.
            if contrib_section < section_num {
                _ = contribs_iter.next();
                continue;
            }

            // Check to see if we're done with this section.
            if contrib_section > section_num {
                break;
            }

            let contrib_offset = contrib.offset.get() as u32;
            if contrib_offset < group_offset {
                // Fast forward to the relevant region. As above (with the section index), this
                // should really never happen.
                _ = contribs_iter.next();
                continue;
            }

            // Stop if we hit an offset that is outside of this COFF group (or section).
            if contrib_offset >= group_offset_end {
                break;
            }

            let contrib_size_i32: i32 = contrib.size.get();
            if contrib_size_i32 < 0 {
                // Contributions should never be this large.
                break;
            }
            let contrib_size: u32 = contrib_size_i32 as u32;
            let Some(contrib_offset_end) = contrib_offset.checked_add(contrib_size) else {
                warn!("section contribution record exceeds limits");
                break;
            };

            let contrib_module_index = contrib.module_index.get();

            // Accept this contribution record.
            _ = contribs_iter.next();

            // This should really never occur, but be paranoid and double-check.
            // Section contributions should always contribute to exactly one COFF group;
            // they shouldn't cross COFF group boundaries.
            if contrib_offset_end > group_offset_end {
                warn!("section contribution record extends beyond end of COFF group");
                break;
            }

            let module_name = if let Some(module) = modules.get(contrib_module_index as usize) {
                module.module_name().to_str_lossy()
            } else {
                "??".into()
            };

            println!(
                "c {off_seg} rva: {contrib_rva:08x} + {contrib_size:08x} : module {contrib_module_index} - {module_name}",
                off_seg = OffsetSegment::new(contrib_offset, section_num),
                contrib_rva = section_rva + contrib_offset
            );

            // Next, find any module symbols that are within the contribution record we just found.
            // The symbol records must fall within [contrib_offset..contrib_offset + contrib_size].

            while let Some(ms) = indexed_module_syms_iter.peek() {
                let ms_segment = ms.off_seg.segment();
                if ms_segment < section_num {
                    _ = indexed_module_syms_iter.next();
                    continue;
                }

                if ms_segment > section_num {
                    break;
                }

                let ms_offset = ms.off_seg.offset();
                if ms_offset < contrib_offset {
                    _ = indexed_module_syms_iter.next();
                    continue;
                }

                if ms_offset >= contrib_offset_end {
                    break;
                }

                // println!("    {} related symbol: mod {}", ms.off_seg, ms.module_index);

                // We know that module_index and byte_offset are valid, since we built this table, above.
                let this_module_symbols: &[u32] = &module_symbols[ms.module_index as usize];
                let this_sym_bytes: &[u8] =
                    &this_module_symbols.as_bytes()[ms.byte_offset as usize..];
                let sym = SymIter::one(this_sym_bytes).unwrap();
                let mut sym_text = String::new();
                dump_syms_context.scope_depth = 0;
                if dump_sym(
                    &mut sym_text,
                    &mut dump_syms_context,
                    ms.byte_offset,
                    sym.kind,
                    sym.data,
                )
                .is_ok()
                {
                    println!("    ... {}", sym_text.trim_ascii());
                }

                _ = indexed_module_syms_iter.next();
            }
        }
    };

    for (i, section) in section_headers.iter().enumerate() {
        let section_num = (i + 1) as u16;
        let section_name = section.name();
        let section_rva = section.virtual_address;

        println!(
            "s {off_seg} rva: {section_rva:08x} + {vsize:08x} : {rwx} : {section_name:<8}     {description}",
            vsize = section.physical_address_or_virtual_size,
            off_seg = OffsetSegment::new(0, section_num),
            rwx = Rwx(section.characteristics),
            description = section_description(&section_name.to_str_lossy()).unwrap_or("")
        );

        if let Some(ref mut groups_iter) = groups_iter {
            while let Some(g) = groups_iter.peek() {
                let desc = coff_group_description(&g.name).unwrap_or("");
                if g.offset_segment.segment.get() != section_num {
                    break;
                }
                let group_virtual_address = section
                    .virtual_address
                    .wrapping_add(g.offset_segment.offset());

                println!(
                    "g {off_seg} rva: {group_virtual_address:08x} + {vsize:08x} : {rwx} :     {name:<30}  {desc}",
                    off_seg = g.offset_segment,
                    rwx = Rwx(section.characteristics),
                    vsize = g.size,
                    name = g.name
                );

                // If requested, show contribs in this section.
                show_contribs_in(section_num, section_rva, g.offset_segment.offset(), g.size);

                // Advance the groups iterator
                _ = groups_iter.next();
            }

            println!();
        } else if options.contribs {
            // No COFF groups, so process all contribs in this section.
            show_contribs_in(
                section_num,
                section_rva,
                0,
                section.physical_address_or_virtual_size,
            );
        }
    }

    Ok(())
}

pub(crate) fn dump_section_contribs(
    pdb: &Pdb,
    dbi_stream: &DbiStream<Vec<u8>>,
) -> anyhow::Result<()> {
    let coff_groups = pdb.coff_groups()?;
    let modules = pdb.modules()?;
    let modules: Vec<ModuleInfo<'_>> = modules.iter().collect();

    println!("*** SECTION CONTRIBUTIONS");
    println!();

    println!("  Imod  Address        Size      Characteristics");

    let section_contribs = dbi_stream.section_contributions()?;
    for contrib in section_contribs.contribs.iter() {
        let group_name = if let Some(group) = coff_groups.find_group_at(OffsetSegment::new(
            contrib.offset.get() as u32,
            contrib.section.get(),
        )) {
            &group.name
        } else {
            "--"
        };

        let module_name: Cow<'_, str> =
            if let Some(module) = modules.get(contrib.module_index.get() as usize) {
                module.module_name.to_str_lossy()
            } else {
                Cow::Borrowed("??")
            };
        let module_file_name: &str = if let Some((_, after)) = module_name.rsplit_once(['\\', '/'])
        {
            after
        } else {
            &module_name
        };

        println!(
            "  {module_index:04X} {section:04X}:{offset:08X}  {size:08X}  {characteristics:08X}  {group_name:<20}  mod: {module_file_name}",
            module_index = contrib.module_index.get() + 1,
            section = contrib.section.get(),
            offset = contrib.offset.get(),
            size = contrib.size.get(),
            characteristics = contrib.characteristics.get(),
        );
    }

    Ok(())
}
