- [PDB Information Stream (Fixed Stream #1)](#pdb-information-stream-fixed-stream-1)
  - [Version](#version)
  - [Named Streams](#named-streams)
    - [Simple decoding](#simple-decoding)
    - [Encoding](#encoding)
    - [Name index table (obsolete)](#name-index-table-obsolete)
  - [Feature Codes](#feature-codes)
  - [Example](#example)
- [References](#references)

# PDB Information Stream (Fixed Stream #1)

The PDB Information Stream contains a variety of global information. It is
generally the first stream that tools read when they open a PDB file. It
contains:

* The binding key (GUID and age) link the PDB to its corresponding executable
  image. This is what tools like WinDbg use to verify that a PDB describes a
  loaded module.
* The version of the MSVC tools that generated this PDB.
* The Named Stream Map.
* The Feature List.

The PDB Information Stream is usually fairly small.

```
// sizeof = dynamic
struct PdbStream {
    PdbStreamHeader header;
    NamedStreams named_streams;
    uint32_t feature_codes[];
}

// sizeof = dynamic (depends on version)
struct PdbStreamHeader {
    uint32_t version;
    uint32_t signature;
    uint32_t age;
    GUID unique_guid;           // present only in version PDBImpvVC70 and later; see below
}
```

## Version

The version field identifies how to interpret many of the data structures in this file. This version number changes very infrequently; changing the version number generally means that all existing code needs to be updated to read the new PDB format.

Known version numbers (from pdb.h in [MS-PDB]):

Name             | Value (decimal) | Has GUID? | Notes
-----------------|-----------------|-----------|-----------
`PDBImpvVC2`     | 19941610        | No        | The PDBI contains only the `PdbStreamHeader`. There is no named streams table after it. This version is no longer supported by MS tools.
`PDBImpvVC4`     | 19950623        | No        | 
`PDBImpvVC41`    | 19950814        | No        | 
`PDBImpvVC50`    | 19960307        | No        | 
`PDBImpvVC98`    | 19970604        | No        | 
`PDBImpvVC70Dep` | 19990604        | No        | Deprecated VC 7.0 version
`PDBImpvVC70`    | 20000404        | Yes       | 
`PDBImpvVC80`    | 20030901        | Yes       | 
`PDBImpvVC110`   | 20091201        | Yes       | 
`PDBImpvVC140`   | 20140508        | Yes       | 

Before VC70, `PdbStreamHeader` did not contain a GUID field.

See <https://github.com/microsoft/microsoft-pdb/blob/master/langapi/include/pdb.h> for version number definitions.

## Named Streams

The Named Stream Map immediately follows the `PdbStreamHeader` and has this structure:

```
struct NamedStreams {
    uint32_t keys_size;                  // The size of the keys data, in bytes
    uint8_t keys_data[keys_size];        // String data for the keys
    uint32_t num_names;                  // The number of entries in hash_entries.
    uint32_t hash_size;                  // The number of entries in the hash table, in memory.
    uint32_t present_mask_size;          // The number of values in present_mask
    uint32_t present_mask[num_present];  // Bit mask of the values that are "present"
    uint32_t deleted_mask_size;          // The number of values in deleted_mask
    uint32_t deleted_mask[num_deleted];  // Bit mask of the values that are "deleted"
    HashEntry hash_entries[name_count];

    // The following are obsolete. It is expected that num_name_index is always zero.
    uint32_t num_name_index;
    NameIndex name_indexes[num_name_index];
}

struct HashEntry {
    uint32_t key;                        // byte offset into `keys` of NUL-terminated string
    uint32_t value;                      // stream index
}
```

`NamedStreams` is a map from a stream name to a stream index. In memory, it is
implemented as a hash table, and the on-disk representation preserves some
information about the hash table. Understanding the on-disk representation,
especially when generating or modifying `NamedStreams`, requires understanding
the in-memory representation.

The in-memory representation consists of these elements:
* An array of `keys` (strings, in this case)
* A corresponding array of `values` (stream indexes, in this case)
* A bitmask of `present` items
* A bitmask of `deleted` items

 The hash table uses
 [open addressing](https://en.wikipedia.org/wiki/Open_addressing), also known as
 linear probing, to resolve collisions. To add an item `(k, v)` to the table:

   1. Let `h = hash(k)`
   2. Let `p = h`, where `p` is the current probing location.
   3. Loop:
      1. If `present[p] == false`, then set `keys[p] = k`, set
         `values[p] = value`, set `present[p] = true`, **and** set
         `deleted[p] = false`. The algorithm halts.
      2. Set `p = (p + 1) % hash_size`.
      3. If `p == h`, then the hash table is full, and it must be resized.
         (Resizing is out of the scope of this description.)
      4. Continue looking for available entries.

To delete an item `k` from the table:
  1. Let `h = hash(k)`
  2. Let `p = h`, where `p` is the current probing location.
  3. Loop:
     1. If `present[p] == true`, use `keys[p]` to read the key string for `p`
        and compare it to `k`. If they are equal, then set `present[p] = false`
        and set `deleted[p] = true`. The algorithm halts.
     2. If `deleted[p] == false`, then `k` is not present in the table. The algorithm halts.
     3. Set `p = (p + 1) % hash_size`.
     4. If `p == h`, then the hash table is full and `k` was never found. The algorithm halts.
     4. Continue looking for available entries.

Thus, the `deleted` bitmask enables the linear probing algorithm to continue
searching for items, even when an item has been deleted. This is necessary
because the algorithm that deletes an item does not move existing items, to
repair damage from hash collisions.

This clarifies the purpose of the `hash_size` field in the on-disk
`NamedStreams` record. This is the size (number of entries) in the `keys` and
`values` tables, in memory. The `present_mask` and `deleted_mask` arrays are the
serialized forms of the bitmasks.

`present_mask` and `deleted_mask` are both LSB-ordered bitmasks (bit vectors).
Both of these bitmasks define a property of the hash table.

> Invariant: `present_mask_size >= num_hashes * 4`

> Invariant: `present_mask_size == deleted_mask_size`

> Invariant: The `present_mask` and `deleted_mask` bitmasks are disjoint. A hash
> table entry cannot be both "deleted" and "present".

The `keys_size` field specifies the size in bytes of `keys_data`, which contains
NUL-terminated UTF-8 strings.

Each `HashEntry` is an entry in the Named Streams Table. The `key` field within
`HashEntry` points into the `keys_data` array, and points to the start of a
NUL-terminated UTF-8 string. The `value` entry is the stream index (the named
stream). All of the entries in `HashEntry` are valid.

The order of items in `HashEntry` is determined by two things:
  1. The hash of the stream name.
  2. The order of items as they were added to the Named Streams Table, because
     the order determines how hash collisions are resolved.

### Simple decoding

Decoders may choose to ignore the hashing scheme entirely. The number of named
streams is usually fairly small and changes infrequently. To load the
`NamedStreams` table, decoders will need to find `hash_entries` data structure,
which requires using the `present_mask_size` and `deleted_mask_size` to find the
offset of `hash_entries`. Decoders can then iterate the entries in
`hash_entries` and ignore the contents of `present_mask` and `deleted_mask`.

### Encoding

Correctly encoding `NamedStreams` requires understanding the linear probing hash
algorithm and generating correct values for `present_mask`. Because the number
of named streams is generally small, encoders can simply regenerate
`NamedStreams` whenever the table is modified.

To do so, start with the following inputs:

1. Let `names` be a vector of strings. Let `num_names` be the length of this vector.
2. Let `values` be a vector of string indexes. The length of this vector is
   equal to the length of `names`.

Then:
1. Choose a value for `num_hashes`. The value must be greater than or equal to
   `num_names`. A larger value will reduce hash collisions, at the cost of
   larger sizes for the `present_mask` and `deleted_mask`. A simple choice would
   be `num_hashes * 1.5`.
2. Create a `present_mask` bit vector whose size is `num_hashes`.
3. Let `hash_entries` be a new empty vector, whose item type is `HashEntry`.
4. Let `keys_data` be a new empty vector of bytes. This will contain key data (strings).
5. For each string `keys[i]` in `keys`:
   1. Let `offset` = the current length of `keys_data`. Append `keys[i]`
      (including the NUL terminator) to `keys_data`.
   1. Let `h = hash(keys[i])`.
   2. Let `p` = `h`, the current probing location.
   3. Loop:
      1. If `present_mask[p] == false`, then set `present_mask[p] == true`,
         append a new record to `keys_data` and break this loop, continuing to
         the `keys[i]` loop.
      2. Set `p = (p + 1) % num_hashes`.
      3. Continue the loop.

The contents of `deleted_mask` is a vector with the same size as `present_mask`,
but whose contents is all zeroes (no deleted items).

### Name index table (obsolete)

The `num_name_index` and `name_indexes` fields are obsolete. `num_name_index`
should always be zero.

## Feature Codes

After the Named Streams Table is an optional list of feature codes. Each feature
code is a `uint32_t` value that specifies whether a specific feature is enabled.
If the Feature Codes table is present, then it covers all data within the PDB
Information Stream that follows the Named Streams Table. (It appears to be a
backward-compatible addition to the PDB file format.)

These are the known feature codes. Feature codes in the table below may be in
decimal or hexadecimal.

Code                 | Name                | Description
---------------------|---------------------|------------
20091201             | `VC110`             |
20140508             | `VC140`             |
0x4D544F4E           | `NoTypeMerge`       |
0x494E494D (`MINI`)  | `MinimalDebugInfo`  | See [Mini PDBs](mini_pdb.md)

## Example

The following is an example of the entire PDB Information Stream from an example PDB.

```
00000000 : 94 2e 31 01 3d 27 f1 8e 02 00 00 00 63 b7 fc 1c : ..1.='......c...
00000010 : 72 76 f1 91 c2 b1 f0 28 b6 29 60 bb 63 00 00 00 : rv.....(.)`.c...
00000020 : 2f 4c 69 6e 6b 49 6e 66 6f 00 2f 54 4d 43 61 63 : /LinkInfo./TMCac
00000030 : 68 65 00 2f 6e 61 6d 65 73 00 2f 55 44 54 53 52 : he./names./UDTSR
00000040 : 43 4c 49 4e 45 55 4e 44 4f 4e 45 00 73 6f 75 72 : CLINEUNDONE.sour
00000050 : 63 65 6c 69 6e 6b 24 31 00 73 6f 75 72 63 65 6c : celink$1.sourcel
00000060 : 69 6e 6b 24 32 00 73 6f 75 72 63 65 6c 69 6e 6b : ink$2.sourcelink
00000070 : 24 32 00 65 6d 62 65 64 73 70 64 00 73 72 63 73 : $2.embedspd.srcs
00000080 : 72 76 00 07 00 00 00 0e 00 00 00 01 00 00 00 f4 : rv..............
00000090 : 05 00 00 01 00 00 00 01 00 00 00 2c 00 00 00 28 : ...........,...(
000000a0 : 09 00 00 1a 00 00 00 26 09 00 00 13 00 00 00 07 : .......&........
000000b0 : 00 00 00 46 00 00 00 2a 09 00 00 00 00 00 00 05 : ...F...*........
000000c0 : 00 00 00 0a 00 00 00 06 00 00 00 5c 00 00 00 29 : ...........\...)
000000d0 : 09 00 00 00 00 00 00 dc 51 33 01                : ........Q3.
```

* At stream offset 0 is the `PdbStreamHeader`.
  + `version` is `94 2e 31 01`, or 0x01312e94, or 20,000,404 in decimal, which is `PDBImpvVC70`.
  + `signature` is `3d 27 f1 8e`, or 0x8ef1273d.
  + `age` is `02 00 00 00`, or 2.
  + `unique_id` is `63 b7 fc 1c 72 76 f1 91 c2 b1 f0 28 b6 29 60 bb`.

The `NamedStreams` table begins at stream offset 0x1c. The `keys_size` is
`63 00 00 00`, or 0x63. Note that this value is not aligned (not a multiple of
4). This means that many of the `uint32_t` values in the table after it are not
aligned.

The `keys_data` ranges starts at stream offset 0x20 and ends at 0x20 + 0x63 =
0x83:

```
00000020 : 2f 4c 69 6e 6b 49 6e 66 6f 00 2f 54 4d 43 61 63 : /LinkInfo./TMCac
00000030 : 68 65 00 2f 6e 61 6d 65 73 00 2f 55 44 54 53 52 : he./names./UDTSR
00000040 : 43 4c 49 4e 45 55 4e 44 4f 4e 45 00 73 6f 75 72 : CLINEUNDONE.sour
00000050 : 63 65 6c 69 6e 6b 24 31 00 73 6f 75 72 63 65 6c : celink$1.sourcel
00000060 : 69 6e 6b 24 32 00 73 6f 75 72 63 65 6c 69 6e 6b : ink$2.sourcelink
00000070 : 24 32 00 65 6d 62 65 64 73 70 64 00 73 72 63 73 : $2.embedspd.srcs
00000080 : 72 76 00 07 00 00 00 0e 00 00 00 01 00 00 00 f4 : rv..............
```

* At stream offset 0x83 is `num_names`, whose value is 7.
* At stream offset 0x87 is `hash_size`, whose value is 0xe = 14.
* At stream offset 0x8b is `present_mask_size`, whose value is 1.
* The `present_mask` starts at stream offset 0x8f and ends at 0x93 (exclusive).
  It contains the byte values `f4 05 00 00`, implying a bit vector of
  `0 0 0 1 1 1 1 1, 1 0 1 0 0 0 0 0, 0 0 0 0 0 0 0 0, 0 0 0 0 0 0 0 0`
* At stream offset 0x93 is `deleted_mask_size`, whose value is 1.
* At stream offset 0x97 is `deleted_mask`. It contains the byte values
  `01 00 00 00`, implying a bit vector of `1 0 0 ... 0`.
* At stream offset 0x9b is `hash_entries`. By using the bitmasks in
  `present_mask` and `deleted_mask`, and by looking up the key strings in
  `keys_data`, we can build the full hash entry:

Stream offset |Index| Key      | Key string         | Value (Stream)<br>(hex)
--------------|-----|----------|--------------------|---------------
0000009b      | 0   | 0000002c | `sourcelink$1`     | 00000928
000000a3      | 1   | 0000001a | `/UDTSRCLINEUNDONE`| 00000926
000000ab      | 2   | 00000013 | `/names`           | 00000007
000000b3      | 3   | 00000046 | `sourcelink$2`     | 0000092a
000000bb      | 4   | 00000000 | `/LinkInfo`        | 00000005
000000c3      | 5   | 0000000a | `/TMCache`         | 00000006
000000cb      | 6   | 0000005c | `srcsrv`           | 00000929

# References

* [LLVM: The PDB Info Stream](https://llvm.org/docs/PDB/PdbStream.html)
* [`class Map`](https://github.com/microsoft/microsoft-pdb/blob/master/PDB/include/map.h) - equivalent to `NamedStreams`
* [`PDBCommon::featNoTypeMerge`](https://github.com/microsoft/microsoft-pdb/blob/master/PDB/include/pdbcommon.h) - describes the feature codes
* [`GSI1::GSI1`](https://github.com/microsoft/microsoft-pdb/blob/master/PDB/dbi/gsi.cpp)
  - The `GSI1::GSI1` constructor chooses the value for `num_buckets` (called
  `iphrHash`) based on the presence of the `MinimalDebugInfo` feature.
