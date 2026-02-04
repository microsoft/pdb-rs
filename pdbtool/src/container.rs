use anyhow::Result;
use std::path::Path;

#[derive(clap::Parser)]
pub struct ContainerOptions {
    /// The PDB file or PDZ file to read.
    pdb: String,

    /// Show the chunks table (applicable only to PDZ)
    #[arg(long)]
    chunks: bool,

    /// Show the streams table
    #[arg(long)]
    streams: bool,
}

pub fn container_command(options: &ContainerOptions) -> Result<()> {
    let pdb = ms_pdb::Pdb::open(Path::new(&options.pdb))?;

    let container = pdb.container();
    match container {
        ms_pdb::Container::Msf(msf) => {
            println!("Container format: MSF (uncompressed)");
            println!("  Number of streams:           {:8}", pdb.num_streams());
            println!(
                "  Page size:                   {:8} bytes per page",
                u32::from(msf.page_size())
            );
            println!("  Number of pages:             {:8}", msf.num_total_pages());
            println!("  Number of pages * page size: {:8}", msf.nominal_size());
            println!("  Number of free pages:        {:8}", msf.num_free_pages());
        }

        ms_pdb::Container::Msfz(msfz) => {
            println!("Container format: MSFZ (compressed)");
            println!("  Number of streams:           {:8}", pdb.num_streams());
            println!("  Number of compressed chunks: {:8}", msfz.num_chunks());
            println!("  Number of stream fragments:  {:8}", msfz.num_fragments());

            if options.chunks {
                // Build a mapping of streams to chunks, so we can display it.
                // We need to know, for each chunk, which streams it contains.
                // Contains (chunk, stream) pairs.
                let mut chunks_to_streams: Vec<(u32, u32)> = Vec::new();

                for stream in 1..msfz.num_streams() {
                    if let Some(fragments) = msfz.stream_fragments(stream) {
                        for f in fragments {
                            if f.location.is_compressed() {
                                let chunk = f.location.compressed_first_chunk();
                                chunks_to_streams.push((chunk, stream));
                            }
                        }
                    }
                }

                chunks_to_streams.sort_unstable();

                println!();
                println!("Chunks table:");
                println!();

                println!("Chunk    | File       | Compressed | Uncompressed | Streams");
                println!("         | offset     | size       | size         |");
                println!("---------------------------------------------------------------");

                let mut j: usize = 0; // index into chunks_to_streams

                let mut streams_list = String::new();

                for (chunk_index, chunk) in msfz.chunks().iter().enumerate() {
                    use std::fmt::Write;

                    streams_list.clear();

                    while j < chunks_to_streams.len() && chunks_to_streams[j].0 < chunk_index as u32
                    {
                        j += 1;
                    }

                    // (start, last)
                    let mut current_range: Option<(u32, u32)> = None;

                    while j < chunks_to_streams.len()
                        && chunks_to_streams[j].0 == chunk_index as u32
                    {
                        let stream = chunks_to_streams[j].1;
                        j += 1;

                        match current_range {
                            // Extend the range, if possible
                            Some((range_start, range_last)) if range_last + 1 == stream => {
                                current_range = Some((range_start, stream));
                                continue;
                            }

                            Some((range_start, range_last)) => {
                                // Previous range could not be extended
                                if range_start != range_last {
                                    _ = write!(streams_list, "{range_start}-{range_last} ");
                                } else {
                                    _ = write!(streams_list, "{range_start} ");
                                }
                            }

                            None => {
                                current_range = Some((stream, stream));
                            }
                        }
                    }

                    if let Some((range_start, range_last)) = current_range {
                        if range_start != range_last {
                            _ = write!(streams_list, "{range_start}-{range_last} ");
                        } else {
                            _ = write!(streams_list, "{range_start} ");
                        }
                    }

                    println!(
                        "  {:6} | {:10} | {:10} | {:12} | {}",
                        chunk_index,
                        chunk.file_offset,
                        chunk.compressed_size,
                        chunk.uncompressed_size,
                        streams_list
                    );
                }
            }
        }
    }

    Ok(())
}
