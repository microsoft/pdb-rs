//! Compares two PDB (or PDZ) files to check that their stream directories and
//! stream contents are identical.

use anyhow::{Context, Result, bail};
use ms_pdb::{Pdb, RandomAccessFile};
use std::path::Path;
use tracing::{error, info};

/// Compares two PDB (or PDZ) files and verifies that their stream contents
/// are identical.
#[derive(clap::Parser)]
pub(crate) struct CompareOptions {
    /// The first PDB file or PDZ file to compare.
    pub(crate) first_file: String,

    /// The second PDB file or PDZ file to compare.
    pub(crate) second_file: String,

    /// Continue comparison even after finding differences (don't stop at first difference).
    #[arg(long)]
    pub continue_on_differences: bool,
}

pub(crate) fn command(options: CompareOptions) -> Result<()> {
    let first_path = Path::new(&options.first_file);
    let second_path = Path::new(&options.second_file);

    info!("Comparing files:");
    info!("  First:  {}", first_path.display());
    info!("  Second: {}", second_path.display());

    // Open both files
    let first_pdb = Pdb::open(first_path)
        .with_context(|| format!("Failed to open first file: {}", options.first_file))?;
    let second_pdb = Pdb::open(second_path)
        .with_context(|| format!("Failed to open second file: {}", options.second_file))?;

    let mut comparison_stats = ComparisonStats::default();
    let success = compare_pdbs(&first_pdb, &second_pdb, &options, &mut comparison_stats)?;

    // Print summary
    info!("Comparison completed:");
    info!("  Total streams compared:   {}", comparison_stats.streams_compared);
    info!("  Streams with differences: {}", comparison_stats.streams_different);
    info!("  Nil streams matched:      {}", comparison_stats.nil_streams_matched);
    
    if !success {
        info!("Result: Files are DIFFERENT");
        bail!("Files are different");
    } else {
        info!("Result: Files are IDENTICAL");
    }

    Ok(())
}

#[derive(Default)]
struct ComparisonStats {
    streams_compared: u32,
    streams_different: u32,
    nil_streams_matched: u32,
}

fn compare_pdbs(
    first_pdb: &Pdb<RandomAccessFile>,
    second_pdb: &Pdb<RandomAccessFile>,
    options: &CompareOptions,
    stats: &mut ComparisonStats,
) -> Result<bool> {
    let first_container = first_pdb.container();
    let second_container = second_pdb.container();

    // Compare number of streams
    let first_num_streams = first_container.num_streams();
    let second_num_streams = second_container.num_streams();

    if first_num_streams != second_num_streams {
        error!(
            "Files have different number of streams. First: {}, Second: {}",
            first_num_streams, second_num_streams
        );
        if !options.continue_on_differences {
            return Ok(false);
        }
    }

    let max_streams = std::cmp::max(first_num_streams, second_num_streams);
    let mut has_differences = first_num_streams != second_num_streams;

    let mut first_stream_data: Vec<u8> = Vec::new();
    let mut second_stream_data: Vec<u8> = Vec::new();

    // Compare each stream (starting from stream 1, as stream 0 is special)
    for stream in 1..=max_streams {
        let first_valid = stream < first_num_streams && first_container.is_stream_valid(stream);
        let second_valid = stream < second_num_streams && second_container.is_stream_valid(stream);

        // Check if both streams have same nil-ness
        if first_valid != second_valid {
            error!(
                "Stream {} has different validity. First: {}, Second: {}",
                stream, first_valid, second_valid
            );
            has_differences = true;
            stats.streams_different += 1;

            if !options.continue_on_differences {
                return Ok(false);
            }
            continue;
        }

        // If both streams are nil, they match
        if !first_valid && !second_valid {
            stats.nil_streams_matched += 1;
            continue;
        }

        // Both streams are valid, compare their contents
        stats.streams_compared += 1;

        // Check stream sizes first
        let first_size = first_container.stream_len(stream);
        let second_size = second_container.stream_len(stream);

        if first_size != second_size {
            error!(
                "Stream {} has different sizes. First: {} bytes, Second: {} bytes",
                stream, first_size, second_size
            );
            has_differences = true;
            stats.streams_different += 1;

            if !options.continue_on_differences {
                return Ok(false);
            }
            continue;
        }

        // Read and compare stream contents
        match compare_stream_contents(
            &first_container,
            &second_container,
            stream,
            &mut first_stream_data,
            &mut second_stream_data,
        ) {
            Ok(true) => {
                // Streams are identical
            }
            Ok(false) => {
                has_differences = true;
                stats.streams_different += 1;

                if !options.continue_on_differences {
                    return Ok(false);
                }
            }
            Err(e) => {
                error!("Failed to compare stream {}: {}", stream, e);
                has_differences = true;
                stats.streams_different += 1;

                if !options.continue_on_differences {
                    return Ok(false);
                }
            }
        }
    }

    Ok(!has_differences)
}

fn compare_stream_contents(
    first_container: &ms_pdb::Container<RandomAccessFile>,
    second_container: &ms_pdb::Container<RandomAccessFile>,
    stream: u32,
    first_stream_data: &mut Vec<u8>,
    second_stream_data: &mut Vec<u8>,
) -> Result<bool> {
    // Read first stream
    first_stream_data.clear();
    first_container.read_stream_to_vec_mut(stream, first_stream_data)?;

    // Read second stream
    second_stream_data.clear();
    second_container.read_stream_to_vec_mut(stream, second_stream_data)?;

    // Find the first different byte
    if let Some(byte_offset) = find_index_of_first_different_byte(first_stream_data, second_stream_data) {
        error!(
            "Stream {} differs at byte offset {} ({:#x})",
            stream, byte_offset, byte_offset
        );
    } else {
        error!("Stream {} has different content but no specific difference found", stream);
    }

    Ok(false)
}

fn find_index_of_first_different_byte(a: &[u8], b: &[u8]) -> Option<usize> {
    if a.len() != b.len() {
        return Some(0);
    }

    const BLOCK_SIZE: usize = 256;
    let mut offset = 0;

    // Compare in blocks for efficiency
    while offset + BLOCK_SIZE <= a.len() {
        let block_a = &a[offset..offset + BLOCK_SIZE];
        let block_b = &b[offset..offset + BLOCK_SIZE];

        if block_a != block_b {
            // Found difference in this block, find exact byte
            for i in 0..BLOCK_SIZE {
                if block_a[i] != block_b[i] {
                    return Some(offset + i);
                }
            }
        }
        offset += BLOCK_SIZE;
    }

    // Compare remaining bytes
    for i in offset..a.len() {
        if a[i] != b[i] {
            return Some(i);
        }
    }

    None
}