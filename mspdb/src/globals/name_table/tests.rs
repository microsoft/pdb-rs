use pretty_hex::PrettyHex;

use super::*;

fn test_names(names: &[String]) -> NameTable {
    println!();

    let num_buckets = 0x1000;

    let mut builder = NameTableBuilder::new(num_buckets);

    for (i, name) in names.iter().enumerate() {
        let symbol_offset = i as i32 + 1;
        builder.push(BStr::new(name), symbol_offset);

        let entry = builder.hash_records.last().unwrap();
        println!(
            "  {i:4} : hash 0x{:08x}, symbol_offset 0x{:08x}, name: {name:?}",
            entry.hash, entry.symbol_offset as u32
        );
    }

    // Add two entries that test our requirements during decoding.
    builder.push("bad_symbol_zero_offset".into(), 0);
    builder.push("bad_symbol_negative_offset".into(), -1);

    let prepared_info = builder.prepare();

    let mut encoded_bytes = vec![0u8; prepared_info.table_size_bytes];
    builder.encode(&prepared_info, &mut encoded_bytes);
    println!("Encoded name table:\n{:?}", encoded_bytes.hex_dump());

    // Decode the table.
    let rt_table = NameTable::parse(num_buckets, 0, &encoded_bytes)
        .expect("Expected table to decode successfully");

    println!("Hash records in decoded table:");
    for (i, hr) in rt_table.hash_records.iter().enumerate() {
        println!("  {i:4} : symbol_offset 0x{:08x}", hr.offset.get() as u32);
    }

    println!("Non-empty buckets:");
    for i in 0..rt_table.hash_buckets.len() - 1 {
        let start = rt_table.hash_buckets[i];
        let end = rt_table.hash_buckets[i + 1];
        if start != end {
            println!("  {i:4} : {:4} .. {:4}", start, end);
        }
    }

    println!("Checking names:");

    // Make sure that all of the names can be found in the table.
    for name in names.iter() {
        let bucket = rt_table.hash_records_for_name(BStr::new(name));
        println!(
            "searching for {name:?}, num entries in bucket = {}",
            bucket.len()
        );

        let mut num_found: u32 = 0;
        for entry in bucket {
            let symbol_offset = entry.offset.get();
            assert!(symbol_offset > 0);
            assert!(symbol_offset as usize - 1 < names.len());
            if names[symbol_offset as usize - 1] == *name {
                num_found += 1;
            }
        }

        assert_eq!(
            num_found, 1,
            "expected to find {name:?} in the table exactly once"
        );
    }

    rt_table
}

#[test]
fn build_empty() {
    test_names(&[]);
}

// Verify that the code that checks for a record offset <= 0 is working.
#[test]
fn build_and_check_bad_names() {
    let names = test_names(&[]);
    let gss = GlobalSymbolStream::new(Vec::new());
    let name_opt = names
        .find_symbol(&gss, "bad_symbol_negative_offset".into())
        .unwrap();
    assert!(name_opt.is_none());
}

#[test]
fn build_simple() {
    let names = vec![
        "achilles".to_string(),
        "castor".to_string(),
        "pollux".to_string(),
    ];
    test_names(&names);
}

#[test]
fn build_many() {
    let mut names: Vec<String> = Vec::new();
    for i in 0..100 {
        names.push(format!("name{i}"));
    }

    test_names(&names);
}
