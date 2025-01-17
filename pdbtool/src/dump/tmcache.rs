use super::*;
use mspdb::tmcache::TMCache;

// TODO: This code is a work in progress. Lots of stuff doesn't work right.
pub fn dump_tmcache(p: &Pdb) -> Result<()> {
    println!("*** TM CACHE");
    println!();

    let Some(tmcache) = p.read_tmcache()? else {
        println!("This PDB does not contain a TMCache.");
        return Ok(());
    };

    // #0000 checksum = 6A60F8F1B52F29C4, cache = #0010
    println!("** Modules");
    if false {
        for (i, m) in tmcache.module_table.iter().enumerate() {
            println!(
                "    #{:04X} checksum = {:016X}, cache = #{:04X}",
                i, m.checksum, m.tm_index
            );
        }
    }

    println!();

    for (i, stream_index) in tmcache.tm_table.iter().enumerate() {
        let Some(stream) = Stream::new(*stream_index) else {
            println!("*** Invalid stream index");
            continue;
        };

        println!("Cache #{:04X}", i);

        let mut r = p.get_stream_reader(stream.into())?;
        let tm = mspdb::tmcache::TMCache::read(&mut r)?;

        match &tm {
            TMCache::Tmts(tmts) => {
                println!("TMTS");
                println!("     TYPE ({})", tmts.ti_mapped_to.len());

                const CHUNK_SIZE: usize = 8;
                for (i, chunk) in tmts.ti_mapped_to.chunks(CHUNK_SIZE).enumerate() {
                    print!("   {:08X}:", 0x1000 + i * CHUNK_SIZE);
                    for to in chunk.iter() {
                        print!(" {:08X}", *to);
                    }
                    println!();
                }

                println!();
                println!("    Func ID to TI mapping");
                println!();

                const K: usize = 4;
                for (i, chunk) in tmts.id_mapped_to.chunks(K).enumerate() {
                    for &to in chunk.iter() {
                        print!("  {:08X}->{:08X}", 0x1000 + i * K, to);
                    }
                    println!();
                }
            }

            _ => {}
        }

        println!();
    }

    Ok(())
}
