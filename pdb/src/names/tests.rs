use super::*;

#[rustfmt::skip]
static NAMES_DATA: &[u8] = &[
    /* 0x0000 */ 0xfe, 0xef, 0xfe, 0xef,                 // signature
    /* 0x0004 */ 1, 0, 0, 0,                             // version
    /* 0x0008 */ 0x18, 0, 0, 0,                          // strings_size
    /* 0x000c */ 0,                                      // empty string
    /* 0x000d */ b'f', b'o', b'o', b'.', b'c', 0,        // (ni 0x0001) "foo.c\0" (len 6)
    /* 0x0013 */ b'b', b'a', b'r', b'.', b'r', b's', 0,  // (ni 0x0007) "bar.rs\0" (len 7)
    /* 0x001a */ b'm', b'a', b'i', b'n', b'.', b'c', 0,  // (ni 0x000e) "main.c\0" (len 7)
    /* 0x0021 */ 0, 0, 0,                                // padding bytes
    /* 0x0024 */ 0, 0, 0, 0,                             // num_hashes
    /* 0x0028 */                                         // hashes (none!)
    /* 0x0028 */ 3, 0, 0, 0,                             // num_strings
];

#[test]
fn test_basic() {
    let names = NamesStream::parse(&NAMES_DATA).unwrap();
    assert_eq!(names.get_string(NameIndex(1)).unwrap(), "foo.c");
    assert_eq!(names.get_string(NameIndex(7)).unwrap(), "bar.rs");
    assert_eq!(names.get_string(NameIndex(0xe)).unwrap(), "main.c");

    // Sort the name table.  After sorting, we should have:
    //      old ni 0x0007, new ni 0x0001 - "bar.rs"
    //      old ni 0x0001, new ni 0x0008 - "foo.c"
    //      old ni 0x000e, new ni 0x000e - "main.c"
    let (mapping, new_names_bytes) = names.rebuild();
    let new_names = NamesStream::parse(new_names_bytes).unwrap();

    assert_eq!(new_names.get_string(NameIndex(1)).unwrap(), "bar.rs");
    assert_eq!(new_names.get_string(NameIndex(8)).unwrap(), "foo.c");
    assert_eq!(new_names.get_string(NameIndex(0xe)).unwrap(), "main.c");

    assert_eq!(mapping.map_old_to_new(NameIndex(7)).unwrap(), NameIndex(1));
    assert_eq!(mapping.map_old_to_new(NameIndex(1)).unwrap(), NameIndex(8));
    assert_eq!(
        mapping.map_old_to_new(NameIndex(0xe)).unwrap(),
        NameIndex(0xe)
    );
}

#[test]
fn rebuild() {
    println!("parsing old names table");
    let old_names = NamesStream::parse(NAMES_DATA).unwrap();

    println!("rebuilding names table");
    let (remapping, new_names_bytes) = old_names.rebuild();
    assert!(!remapping.table.is_empty());
    assert_eq!(remapping.table[0], (NameIndex(0), NameIndex(0)));

    // The old_name_index values should be strictly increasing.
    for w in remapping.table.windows(2) {
        assert!(
            w[0].0 < w[1].0,
            "The old_name_index values should be strictly increasing."
        );
    }

    println!("parsing new names table");
    let new_names = NamesStream::parse(new_names_bytes.as_slice())
        .expect("expected rebuild Names table to successfully parse");

    // All entries in remapping should be valid in both old and new table.
    // The string value should be equal, for both.
    println!("validating mapping");
    for &(old_index, new_index) in remapping.table.iter() {
        let old_str = match old_names.get_string(old_index) {
            Ok(s) => s,
            Err(_) => panic!("Did not find mapping for {old_index} in old name table"),
        };

        let new_str = match new_names.get_string(new_index) {
            Ok(s) => s,
            Err(_) => panic!("Did not find mapping for {new_index} in new name table"),
        };

        assert_eq!(
            old_str, new_str,
            "old_index = {old_index}, new_index = {new_index}"
        );
    }

    // Rebuilding the names table _again_ should produce the exact same bytes.
    let (roundtrip_remapping, roundtrip_names_bytes) = new_names.rebuild();

    // We do not expect the remapping table to be the same, but we do expect the old/new values to be the same.
    assert_eq!(remapping.table.len(), roundtrip_remapping.table.len());
    for (i, &(old_name_index, new_name_index)) in roundtrip_remapping.table.iter().enumerate() {
        assert_eq!(old_name_index, new_name_index, "i = {i}");
    }

    assert_eq!(
        roundtrip_names_bytes, new_names_bytes,
        "Round-trip name table should be identical."
    );
}
