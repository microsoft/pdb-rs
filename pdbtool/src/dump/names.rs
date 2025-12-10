use super::*;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::hash;
use ms_pdb::names::NameIndex;

#[derive(clap::Parser)]
pub struct DumpNamesOptions {
    #[arg(long)]
    max: Option<usize>,

    /// Show hex offsets (NameIndex) for each string.
    #[arg(long)]
    show_offsets: bool,

    /// Show the contents of the name hash table
    #[arg(long)]
    show_hashes: bool,
}

pub fn dump_names(pdb: &Pdb, options: DumpNamesOptions) -> anyhow::Result<()> {
    let names_stream = pdb.names()?;
    let names_stream_index = pdb.named_stream_err(ms_pdb::names::NAMES_STREAM_NAME)?;
    println!("Names Stream Index: {names_stream_index}");

    println!(
        "Number of names in table (as declared in the stream): {:6}",
        names_stream.num_strings
    );
    println!(
        "Number of hash entries:                               {:6}",
        names_stream.num_hashes
    );

    println!();
    println!("Strings:");
    println!();

    for (i, (range, name)) in names_stream.iter().with_ranges().enumerate() {
        if let Some(max) = options.max {
            if i >= max {
                println!("(stopping because we reached max)");
                break;
            }
        }

        if options.show_offsets {
            println!("[{:08x}] {name:?}", range.start);
        } else {
            println!("{name:?}");
        }
    }

    if options.show_hashes {
        println!();
        println!("Hash buckets:");
        println!();

        let hashes = names_stream.hashes();
        let mut num_hashes_good: usize = 0;
        let mut num_hashes_bad: usize = 0;
        let mut num_hashes_unused: usize = 0;
        let mut probing_hash_base: u32 = 0;

        for (i, &ni) in hashes.iter().enumerate() {
            if ni.get() == 0 {
                println!("  hash 0x{i:08x} : none");
                num_hashes_unused += 1;
                probing_hash_base = i as u32 + 1;
                continue;
            }

            let s = names_stream.get_string(NameIndex(ni.get()))?;
            let computed_hash = hash::hash_mod_u32(s, names_stream.num_hashes as u32);
            println!("  hash 0x{i:08x} : computed hash 0x{computed_hash:08x} : {s}");

            let hash_is_good = computed_hash == i as u32
                || (computed_hash >= probing_hash_base && computed_hash < i as u32);
            if hash_is_good {
                num_hashes_good += 1;
            } else {
                num_hashes_bad += 1;
            }
        }

        println!();
        println!("Number of hashes that are correct:    {num_hashes_good:8}");
        println!("Number of hashes that are wrong:      {num_hashes_bad:8}");
        println!("Number of hash slots that are unused: {num_hashes_unused:8}");

        let num_hashes_used = num_hashes_good + num_hashes_bad;
        if num_hashes_used == names_stream.num_strings {
            println!("Number of hashes used is equal to total number of strings (good).");
        } else {
            println!(
                "error: Number of hashes used is {}, which is not equal to the total number of strings ({}).",
                num_hashes_used, names_stream.num_strings
            );
        }
    }
    Ok(())
}
