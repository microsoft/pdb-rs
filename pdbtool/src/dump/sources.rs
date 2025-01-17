use super::*;

#[derive(StructOpt)]
pub struct SourcesOptions {
    /// Show all files
    #[structopt(long)]
    pub files: bool,

    /// Show all modules and their source files. This is the default, if no other options
    /// are specified.
    #[structopt(long)]
    pub modules: bool,

    /// Show one specific file, by index.
    #[structopt(long, short)]
    pub file: Option<u32>,

    /// Show indexes (name offsets, etc.).
    #[structopt(long)]
    pub indexes: bool,
}

pub fn dump_dbi_sources(
    dbi_stream: &DbiStream<Vec<u8>>,
    mut options: SourcesOptions,
) -> anyhow::Result<()> {
    let sources_substream = DbiSourcesSubstream::parse(dbi_stream.source_info())?;

    if !options.files && !options.modules && options.file.is_none() {
        options.modules = true;
    }

    let mut module_infos: Vec<ModuleInfo> = Vec::new();
    let modules_substream = dbi_stream.modules();
    module_infos.extend(modules_substream.iter());

    let mut file_name_offsets: Vec<u32> = sources_substream
        .file_name_offsets()
        .iter()
        .map(|x| x.get())
        .collect();
    file_name_offsets.sort_unstable();
    file_name_offsets.dedup();

    println!("DBI File Info substream:");
    println!(
        "Number of modules:      {:8}",
        sources_substream.num_modules()
    );
    println!(
        "Number of sources:      {:8} (unique)",
        file_name_offsets.len()
    );
    println!(
        "Number of file offsets: {:8} (not unique)",
        sources_substream.file_name_offsets().len()
    );
    println!();

    if options.files {
        for &name_offset in file_name_offsets.iter() {
            let name = sources_substream.get_source_file_name_at(name_offset)?;
            if options.indexes {
                println!("  [{name_offset:08x}] : {name}");
            } else {
                println!("  {name}");
            }
        }
        println!();
    }

    if let Some(file_index) = options.file {
        if let Some(&offset) = sources_substream
            .file_name_offsets()
            .get(file_index as usize)
        {
            println!("File name offset: 0x{:x}", offset);
            let file_name = sources_substream.get_source_file_name_at(offset.get())?;
            println!("{}", file_name);
        } else {
            println!(
                "File index {file_index} is out of range. Number of files: {}",
                sources_substream.file_name_offsets().len()
            );
        }
    }

    let num_modules = module_infos.len();
    if num_modules != sources_substream.num_modules() {
        error!("Number of modules is wrong");
    }

    if options.modules {
        for (module_index, module_info) in module_infos.iter().enumerate() {
            if options.indexes {
                println!("Module #{module_index} : {}", module_info.module_name());
            } else {
                println!("Module: {}", module_info.module_name());
            }
            println!("    object: {}", module_info.obj_file());

            for name_offset in sources_substream.name_offsets_for_module(module_index)? {
                match sources_substream.get_source_file_name_at(name_offset.get()) {
                    Ok(name) => {
                        if options.indexes {
                            println!("    [{:08x}] : {}", name_offset.get(), name);
                        } else {
                            println!("    {}", name);
                        }
                    }
                    Err(e) => {
                        error!("{}", e);
                    }
                }
            }

            println!();
        }
    }

    Ok(())
}
