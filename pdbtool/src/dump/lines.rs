use super::*;
use dump_utils::HexStr;
use mspdb::lines::{FileChecksum, FileChecksumsSubsection, LinesSubsection, SubsectionKind};
use std::collections::HashMap;

/// Dumps C13 Line Data for a given module.
#[derive(StructOpt)]
pub struct LinesOptions {
    /// The module index of the module to dump.
    #[structopt(long)]
    pub module: Option<usize>,
}

fn dump_module_lines(
    pdb: &Pdb,
    module_index: usize,
    module: &ModuleInfo,
    names: &mspdb::names::NamesStream<Vec<u8>>,
    sources: &mspdb::dbi::sources::DbiSourcesSubstream,
) -> Result<()> {
    println!("Module: {}", module.module_name());
    println!("    Obj file: {}", module.obj_file());

    if let Some(module_stream) = module.stream() {
        println!("    Stream: {}", module_stream,);
    } else {
        println!("    Stream: (none)");
        return Ok(());
    };

    if module.header().c11_byte_size.get() != 0 {
        println!("    *** Module has obsolete C11 line data, which is not supported ***");
        return Ok(());
    }

    if module.header().c13_byte_size.get() == 0 {
        println!("    Module has no line data.");
        return Ok(());
    }

    // unwrap() is ok because we tested module.stream() above.
    let module_stream = pdb.read_module_stream(module)?.unwrap();
    let c13_stream_offset = module_stream.c13_line_data_range().start as u32;
    let c13_line_data = module_stream.c13_line_data();

    let mut checksums: HashMap<u32, FileChecksum<'_>> = HashMap::new();

    if let Some(checksums_subsection) = c13_line_data.find_checksums() {
        for (range, checksum) in checksums_subsection.iter().with_ranges() {
            checksums.insert(range.start as u32, checksum);
        }
    }

    println!();

    let mut iter = c13_line_data.subsections().with_ranges();
    for (subsection_range, subsection) in iter.by_ref() {
        println!(
            "[{:08x}] Subsection: {:?}, len {}",
            c13_stream_offset + subsection_range.start as u32,
            subsection.kind,
            subsection.data.len()
        );

        match subsection.kind {
            SubsectionKind::LINES => {
                let contribution = mspdb::lines::LinesSubsection::parse(subsection.data)?;
                println!(
                    "    contribution: offset 0x{:x}, segment {}, size {}",
                    contribution.contribution.contribution_offset,
                    contribution.contribution.contribution_segment,
                    contribution.contribution.contribution_size
                );

                for block in contribution.blocks() {
                    println!(
                        "        block: file {}, num_lines {}",
                        block.header.file_index, block.header.num_lines
                    );
                    if let Some(checksum) = checksums.get(&block.header.file_index.get()) {
                        let name = names.get_string(checksum.name())?;
                        println!("            file: {}", name);
                    } else {
                        println!(
                            "            file: unknown: {}",
                            block.header.file_index.get()
                        );
                    }

                    print!("            lines: ");
                    for (i, line) in block.lines().iter().enumerate() {
                        if i != 0 {
                            print!(", ");
                        }

                        let line_num_start = line.line_num_start();
                        if mspdb::lines::is_jmc_line(line_num_start) {
                            print!("<no-step>");
                        } else {
                            print!("{}", line_num_start);
                        }
                    }
                    println!();
                }
            }

            SubsectionKind::FILE_CHECKSUMS => {
                let name_offsets_for_module = if module_index < sources.num_modules() {
                    sources.name_offsets_for_module(module_index)?
                } else {
                    &[]
                };

                let checksums = mspdb::lines::FileChecksumsSubsection {
                    bytes: subsection.data,
                };

                for (i, checksum) in checksums.iter().enumerate() {
                    let name = names.get_string(checksum.name())?;

                    println!(
                        "  checksum: file_offset {:08x}, kind {:?} : {:?} : {name}",
                        checksum.header.name.get(),
                        checksum.header.checksum_kind,
                        HexStr::new(checksum.checksum_data).packed()
                    );

                    if let Some(&name_offset) = name_offsets_for_module.get(i) {
                        let name2 = sources.get_source_file_name_at(name_offset.get())?;
                        if name != name2 {
                            println!("    different name: {}", name2);
                        }
                    } else {
                        println!("    index is out of range");
                    }
                }
            }

            _ => {
                println!("{:?}", HexDump::new(subsection.data).max(0x200));
            }
        }

        println!();
    }

    if !iter.inner().rest().is_empty() {
        println!();
        println!("Found unparsed data at the end:");
        println!("{:?}", HexDump::new(iter.inner().rest()).at(iter.pos()));
    }

    Ok(())
}

pub fn dump_lines(options: LinesOptions, p: &Pdb, dbi_stream: &DbiStream<Vec<u8>>) -> Result<()> {
    let names = p.names()?;
    let sources = dbi_stream.sources()?;

    if let Some(module_index) = options.module {
        if let Some(module) = dbi_stream.modules().iter().nth(module_index) {
            dump_module_lines(p, module_index, &module, names, &sources)?;
        } else {
            println!("There is no module with the requested index.");
        }
    } else {
        for (module_index, module) in dbi_stream.modules().iter().enumerate().take(20) {
            dump_module_lines(p, module_index, &module, names, &sources)?;
        }
    }

    Ok(())
}

pub fn dump_lines_like_cvdump(p: &Pdb, dbi_stream: &DbiStream<Vec<u8>>) -> Result<()> {
    let names_stream = p.names()?;

    println!("*** LINES");
    println!();

    for module in dbi_stream.modules().iter() {
        if module.module_name() == module.obj_file() {
            println!("** Module: \"{}\"", module.module_name());
        } else {
            println!(
                "** Module: \"{}\" from \"{}\"",
                module.module_name(),
                module.obj_file()
            );
        }
        println!();

        let Some(module_stream) = p.read_module_stream(&module)? else {
            continue;
        };
        let line_data = module_stream.c13_line_data();

        // Find the Checksums subsection. There should be at most one.
        let checksums_subsection_data = line_data.find_checksums_bytes();
        let checksums = if let Some(chk) = &checksums_subsection_data {
            Some(FileChecksumsSubsection { bytes: chk })
        } else {
            None
        };

        for subsection in line_data.subsections() {
            match subsection.kind {
                SubsectionKind::LINES => {
                    let lines = LinesSubsection::parse(subsection.data)?;
                    let contribution_offset = lines.contribution.contribution_offset.get();

                    for block in lines.blocks() {
                        if let Some(checksums) = &checksums {
                            if let Ok(f) = checksums.get_file(block.header.file_index.get()) {
                                let file_name = names_stream.get_string(f.name())?;
                                println!(
                                    "  {file_name} ({:?}: {:?})",
                                    f.header.checksum_kind,
                                    HexStr::new(f.checksum_data).packed()
                                );
                            } else {
                                println!("warning: failed to get file name");
                            }
                            println!();

                            const NUM_COLUMNS: usize = 4;
                            let mut column = 0;

                            for line in block.lines() {
                                print!(
                                    " {:6} {:08X}",
                                    line.line_num_start(),
                                    contribution_offset + line.offset.get()
                                );

                                column += 1;
                                if column == NUM_COLUMNS {
                                    println!();
                                    column = 0;
                                }
                            }
                            if column != 0 {
                                println!();
                            }
                            println!();
                        } else {
                            println!("warning: This module has no file checksums!");
                        }
                    }
                }

                _ => {}
            }
        }
    }

    Ok(())
}
