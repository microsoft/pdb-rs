use super::*;
use tracing::warn;

#[derive(Debug)]
#[allow(dead_code)]
enum StreamUsage {
    OldStreamDir, // 0
    PDB,          // 1
    TPI,          // 2
    DBI,          // 3
    IPI,          // 4
    ModuleInfo {
        module_name: String,
        obj_name: String,
    },
    Named {
        name: String,
    },
    GlobalSymbolStream,
    GlobalSymbolIndex,
    PublicSymbolStream,
    OptionalDebugHeader {
        which: usize,
        whichs: Option<OptionalDebugHeaderStream>,
    },
    TypeStreamHashStream {
        parent_stream: Stream,
    },
    TypeStreamAuxHashStream {
        parent_stream: Stream,
    },
    TMCache {
        cache_slot: u32,
    },
}

#[derive(StructOpt)]
pub struct StreamsOptions {
    /// Show the blocks assigned to each stream
    #[structopt(long)]
    pages: bool,

    /// Show only this stream (name or index)
    stream: Option<String>,
}

pub fn dump_streams(p: &Pdb, options: StreamsOptions) -> anyhow::Result<()> {
    let num_streams = p.num_streams();

    let mut streams_usage: Vec<Option<StreamUsage>> =
        (0..num_streams as usize).map(|_| None).collect();

    streams_usage[0] = Some(StreamUsage::OldStreamDir);
    streams_usage[Stream::PDB.index()] = Some(StreamUsage::PDB);
    streams_usage[Stream::TPI.index()] = Some(StreamUsage::TPI);
    streams_usage[Stream::DBI.index()] = Some(StreamUsage::DBI);
    streams_usage[Stream::IPI.index()] = Some(StreamUsage::IPI);

    let mut add_stream_usage = |stream_opt: Option<u32>, usage: StreamUsage| {
        let Some(stream_index) = stream_opt else {
            return;
        };

        if let Some(slot) = streams_usage.get_mut(stream_index as usize) {
            if let Some(existing_usage) = slot.as_ref() {
                error!(
                    "Stream index #{} has conflicting usages.\n  Usage #1: {:?}\n  Usage #2: {:?}",
                    stream_index, existing_usage, usage
                );
            } else {
                *slot = Some(usage);
            }
        } else {
            error!(
                "Stream index #{} is invalid (is out of range).  Usage is invalid: {:?}",
                stream_index, usage
            );
        }
    };

    let dbi = p.read_dbi_stream()?;

    add_stream_usage(
        dbi.header().global_stream_index().ok(),
        StreamUsage::GlobalSymbolIndex,
    );
    add_stream_usage(
        dbi.header().sym_record_stream().ok(),
        StreamUsage::GlobalSymbolStream,
    );
    add_stream_usage(
        dbi.header().public_stream_index().ok(),
        StreamUsage::PublicSymbolStream,
    );

    if let Some(tpi_header) = p.tpi_header()?.header() {
        add_stream_usage(
            tpi_header.hash_stream_index.get(),
            StreamUsage::TypeStreamHashStream {
                parent_stream: Stream::TPI,
            },
        );
        add_stream_usage(
            tpi_header.hash_aux_stream_index.get(),
            StreamUsage::TypeStreamAuxHashStream {
                parent_stream: Stream::TPI,
            },
        );
    }

    if let Some(ipi_header) = p.ipi_header()?.header() {
        add_stream_usage(
            ipi_header.hash_stream_index.get(),
            StreamUsage::TypeStreamHashStream {
                parent_stream: Stream::IPI,
            },
        );
        add_stream_usage(
            ipi_header.hash_aux_stream_index.get(),
            StreamUsage::TypeStreamAuxHashStream {
                parent_stream: Stream::IPI,
            },
        );
    }

    let pdb_info = p.pdbi();

    for (name, stream) in pdb_info.named_streams().iter() {
        add_stream_usage(
            Some(*stream),
            StreamUsage::Named {
                name: name.to_string(),
            },
        );
    }

    for module in dbi.modules().iter() {
        add_stream_usage(
            module.stream(),
            StreamUsage::ModuleInfo {
                module_name: module.module_name().to_string(),
                obj_name: module.obj_file().to_string(),
            },
        );
    }

    let optional_debug_header = dbi.optional_debug_header()?;
    for (i, stream) in optional_debug_header.iter_streams() {
        add_stream_usage(
            Some(stream),
            StreamUsage::OptionalDebugHeader {
                which: i,
                whichs: OptionalDebugHeaderStream::try_from(i).ok(),
            },
        );
    }

    let one_stream: Option<u32> = if let Some(stream_name) = &options.stream {
        Some(crate::save::get_stream_index(p, stream_name)?.0)
    } else {
        None
    };

    match p.read_tmcache() {
        Ok(Some(tmcache)) => {
            // (stream_index, cache_slot)
            let mut tm_cache_streams: Vec<(u32, u32)> = Vec::with_capacity(tmcache.tm_table.len());
            for (i, &s) in tmcache.tm_table.iter().enumerate() {
                tm_cache_streams.push((s as u32, i as u32));
            }

            tm_cache_streams.sort_unstable();
            tm_cache_streams.dedup_by_key(|i| i.0); // de-dup by stream index

            for &(s, cache_slot) in tm_cache_streams.iter() {
                add_stream_usage(Some(s), StreamUsage::TMCache { cache_slot });
            }
        }

        Ok(None) => {}

        Err(e) => {
            warn!("Failed to read TMCache stream: {e:?}");
        }
    }

    let mut num_streams_unknown_usage: u32 = 0;

    for (stream_index, usage_opt) in streams_usage.iter().enumerate() {
        let stream_index = stream_index as u32;

        // Filter out streams, if desired.
        if let Some(s) = one_stream {
            if stream_index != s {
                continue;
            }
        }

        let stream_size = p.stream_len(stream_index);
        if let Some(usage) = usage_opt {
            println!(
                "Stream #{stream_index:6} : (size {stream_size:10}) {:?}",
                usage
            );
            if p.is_stream_valid(stream_index) {
            } else {
                println!("   error: Stream is nil");
            }
        } else {
            if p.is_stream_valid(stream_index) {
                println!("Stream #{stream_index:6} : (size {stream_size:10}) UNKNOWN USAGE");
                num_streams_unknown_usage += 1;
            } else {
                println!("Stream #{stream_index} is nil");
            }
        }

        if options.pages {
            if let Some(msf) = p.msf() {
                let (_stream_len, stream_pages) = msf.stream_size_and_pages(stream_index)?;
                println!("    Pages: {:?}", DumpRangesSucc::new(stream_pages));
            }
        }
    }

    if num_streams_unknown_usage != 0 {
        println!(
            "Number of streams with unknown usage: {}",
            num_streams_unknown_usage
        );
    }

    Ok(())
}
