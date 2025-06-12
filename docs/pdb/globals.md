- [Global Symbols: GSS, GSI, and PSI](#global-symbols-gss-gsi-and-psi)
- [Global Symbol Stream (GSS)](#global-symbol-stream-gss)
  - [Reference Symbols](#reference-symbols)
- [Name Table (used in GSI and PSI)](#name-table-used-in-gsi-and-psi)
  - [Hash Records](#hash-records)
  - [Hash Buckets](#hash-buckets)
- [Example Name Table Header](#example-name-table-header)
- [Hash Records](#hash-records-1)
- [Example of hash records](#example-of-hash-records)
- [Hash Buckets](#hash-buckets-1)
- [Building the Name Table](#building-the-name-table)
- [Loading the Name Table](#loading-the-name-table)
- [Querying the Name Table](#querying-the-name-table)
- [Global Symbol Index (GSI) Stream](#global-symbol-index-gsi-stream)
- [Public Symbol Index (PSI) Stream](#public-symbol-index-psi-stream)
- [Address Table](#address-table)
- [Querying the PSI Address Table](#querying-the-psi-address-table)
- [Pointer relationships](#pointer-relationships)

# Global Symbols: GSS, GSI, and PSI

Each PDB contains a set of global symbols. Three streams contain the global
symbols stream and indexed views of the global symbols:

* The Global Symbol Stream (GSS) contains the global symbol records. These are
  encoded using the CodeView symbol stream format, which is described in
  [Symbols](symbols.md).

* The Global Symbol Index (GSI) contains a name-to-symbol lookup table for a
  certain set global symbol kinds; that set will be described below. The GSI
  does not directly contain symbol records; it only contains pointers to them.

* The Public Symbol Index (PSI) contains a name-to-symbol lookup table for
  `S_PUB32` symbols. It also contains an address-to-symbol lookup table. Similar
  to the GSI, the PSI does not directly contain symbol records.

The stream indexes for the GSS, GSI, and PSI are specified in the DBI Stream
Header.

The GSI and PSI use the same format for the name-to-symbol lookup table. It is
specified below.

The GSI and PSI are functions of the GSS; that is, the entire contents of the
GSI and PSI can be reconstructed from the GSS. The GSI and PSI aid in query
performance.

# Global Symbol Stream (GSS)

The Global Symbol Stream contains a sequence of variable-length symbol records.
This stream does not have a header; all of the stream data consists of CodeView
symbol records. See [Symbols](symbols.md) for the framing of the records and the
description of their structure.

> Invariant: The size of the Global Symbol Stream is a multiple of 4.

The Global Symbol Stream may only contain records in the following list. All
other symbol records are not permitted in the GSS.

Record Kind<br> Value (hex) | Record Kind Name | Description
-------|--------------------|---
0x1107 | `S_CONSTANT`       | Global named constants and enum values
0x1108 | `S_UDT`            | User-defined types, such as structs, classes, unions, enums, and type aliases (`using`).
0x110C | `S_LDATA32`        | Local data declarations with global scope.
0x110D | `S_GDATA32`        | Global variables
0x110E | `S_PUB32`          | Public symbols
0x1112 | `S_LTHREAD32`      | Module-private variables with thread-local storage
0x1113 | `S_GTHREAD32`      | Global variables with thread-local storage
0x111d | `S_GMANDATA`       | Global managed variables (MSIL)
0x1125 | `S_PROCREF`        | A reference to a global procedure symbol
0x1127 | `S_LPROCREF`       | A reference to a local procedure symbol
0x1128 | `S_ANNOTATIONREF`  | A reference to an `S_ANNOTATION` symbol
0x1129 | `S_TOKENREF`       | A reference to a managed token (MSIL)

## Reference Symbols

Some symbols in the GSS are references to symbols stored in module streams.
These are the reference symbols:

* `S_PROCREF` - Refers to `S_GPROC32` in a module stream
* `S_LPROCREF` - Refers to `S_LPROC32` in a module stream
* `S_DATAREF` - Refers to `S_LDATA32` or `S_GDATA32` in a module stream.
  However, we have never observed `S_DATAREF` symbols in PDBs, so this symbol
  may be obsolete.
* `S_ANNOTATIONREF` - Refers to `S_ANNOTATION` in a module stream
* `S_TOKENREF` - Refers to one of several symbol records that identify MSIL
  tokens

These symbols use the [`RefSym2`](symbols.md#refsym2-structure) record
definition.

These records allow an application to find a symbol definition using the name of
the symbol, such as a procedure. First, the application scans the records in the
GSS and finds an `S_PROCREF` or `S_LPROCREF` structure, using the `name` field
stored in those records for comparison. If a match is found, then the record
gives the module index and byte offset within the module's symbol stream. This
search does require an unindexed sequential scan through the GSS, but it avoids
scanning through all module symbol streams. The GSI can be used to further
accelerate procedure lookups.

# Symbol Name Table (used in GSI and PSI)

Before we specify the structure of the GSI and PSI, we first define a Symbol
Name Table. The GSI and PSI both use the same Name Table format.

The Name Table allows finding global symbols using their symbol name. The
lookups are case-insensitive. Lookups require the entire symbol name; partial
(substring) lookups are not supported because the lookups use name hashing. The
hash function is case-insensitive, for ASCII.

The Name Table can use one of two encodings: the "large" encoding or the "small"
encoding. The large encoding appears to have been the earlier one, with the
"small" encoding being a later optimization of it. Only the small encoding has
been observed. This document only specifies the small encoding.

The Name Table is a hash table. It has the following structure. Again, this
describes only the "small table" representation; the "large" representation is
not specified.

```
struct SymbolNameTable {
    // Always 0xFFFF_FFFF. This value indicates that the "small" representation is being used.
    uint32_t signature;

    // Always 0xF12F_091A.  This value is 0xEFFE_0000 + 19990810.
    // This suggests that this version was created on August 10, 1999.
    uint32_t version;

    // The size in bytes of hash_records. Since each hash record has a fixed
    // size of 8 bytes, this determines the number of hash records.
    uint32_t hash_records_size;

    // The size in bytes of the hash_buckets region.
    uint32_t hash_buckets_size;

    // Contains one record for each symbol in the Name Table.
    HashRecord hash_records[header.hash_records_size];

    // Contains a bitmap which describes which hash buckets are present
    uint8_t hash_buckets[header.hash_buckets_size];
}
```

> Invariant: `hash_records_size` is a multiple of 8.

Let `num_hash_records = hash_records_size / 8`.

## Hash Records

```
struct HashRecord {
    // This field specifies the byte offset of this symbol within the Global
    // Symbol Stream (GSS), plus 1.
    int32_t offset; 

    // This appears to be a reference count, but is only relevant to in-memory
    // data structures. Decoders should ignore this field. Encoders should set
    // this field 1.
    int32_t c_refs;
}
```

Each entry in `hash_records` describes a symbol record stored in the Global
Symbol Stream. `hash_records` is sorted by hash code, in increasing order. The
hash code is not stored directly in the `HashRecord`, but is computed by
accessing the symbol record stored in the GSS.

> Invariant: `offset` is positive. `offset - 1` points to a valid byte offset of
> the start of a symbol record within the Global Symbol Stream (GSS), and that
> record has a `name` field.

> Limit: The previous invariant implies that the GSS cannot be larger than
> approximately 2^31.

> Determinism: The `c_ref` field should always be set to 1.

## Hash Buckets

A _hash bucket_ is a sequence of contiguous hash records within `hash_records`
that have the same hash value.

The `hash_buckets` array contains a compressed array of `int32_t` values, one
for each hash bucket. It must be decompressed before it can be used. The
compression scheme uses a bitmap which indicates which of the hash buckets are
non-empty. After decompression, each `int32_t` value points into the
`hash_records` array and specifies the start of the hash records for a given
hash bucket. The following `int32_t` value (if there is one) implies the end of
the hash bucket.

Let `DecompressedHashBuckets` be the in-memory representation of the hash
buckets:

```
struct DecompressedHashBuckets {
    int32_t buckets[num_buckets];
}
```

`num_buckets` is a parameter that is chosen when the hash table is constructed.
Unfortunately, this parameter is **not** stored in the Name Table. The value is
a function of bit flags stored in the [PDB Information Stream](pdb_stream.md).

# Example Name Table Header

In this example, the PSI Stream Header is at offset 0. The Name Table
immediately follows the PSI Stream Header and is at offset 0x1A (28). The
`ver_signature` field is -1; the `ver_header` field is 0xF12F091A. The
`hash_records_size` field is 0x88500; the `hash_buckets_size` is 0x4204.

# Hash Records

There is one `HashRecord` structure for each name in the Name Table.
`HashRecord` has this structure:

> Invariant: The `offset` field is positive.

> Invariant: The offset field of `HashRecord` (after subtracting 1) points to a
> valid byte offset of the start of a symbol record within the Global Symbol
> Stream (GSS).

Each hash record points to the Global Symbol Stream (GSS). The hash record does
not store the hash value of a symbol; applications which read or write PDBs are
expected to generate those hash values during table construction or table
querying.

# Example of hash records

The first record has an offset value of 0x66_2121. Removing the +1 bias gives a
value of 0x66_2120. In the GSS of this PDB at offset 0x66_2120 we see:

The 2A 00 encodes the symbol length (42 bytes). The 0E 11 encodes the symbol
kind (S_PUB32). The symbol name is `__imp_RtlInitUnicodeStringEx`.

The hash records are sorted in order of increasing hash code. Each contiguous
run of hash records that have the same hash code form a single hash bucket.

Conceptually, the hash records look like this:

Hash record index | Hash code (bucket) | Symbol name
------------------|--------------------|------------
0                 | 0                  | `DllMain`
1                 | 0                  | `FooBar`
2                 | 0                  | `ZapfDingbats`
3                 | 1                  | `MessageBoxW`
4                 | 1                  | `GetProcAddress`
5                 | 2                  | `CreateWindowW`
6                 | 3                  | `CloseHandle`
7                 | 3                  | `HeapAlloc`
8                 | 4                  | `CreateThread`
9                 | 4                  | `GetCurrentThread`
...               | ...                | ...
10,571            | 4095               | `ExitProcess`
10,572            | 4095               | `__chkstk`

The encoded form of the hash records does not contain these fields directly. The
hash record index is implicit (it is simply the location of the hash record
itself). The hash code was computed when the table was generated but is not
stored. The only field in `HashRecord` that is needed is the offset field, which
points to the symbol record in the Global Symbol Stream (GSS).

The hash buckets array looks like this:

Bucket index | Hash record offset | Covers these symbols
-------------|--------------------|---------------------
0            | 0                  | `DllMain`, `Foobar`, `ZapfDingbats`
1            | 3                  | `MessageBoxW`, `GetProcAddress`
2            | 5                  | `CreateWindowW`
3            | 6                  | `CloseHandle`, `HeapAlloc`
4            | 8                  | `CreateThread`, `GetCurrentThread`
...          | ...
4095         | 10,571             | `ExitProcess`, `_chkstk`

# Hash Buckets

The `hash_buckets` region contains the hash buckets for the Name Table. As
mentioned above, each hash bucket is a sequence of contiguous hash records. This
is an implication of another invariant: The hash records are sorted by their
hash.

To describe a hash bucket, we need the index of the first hash record in this
hash bucket and the number of entries in that hash bucket. This is what the
`hash_buckets` region tells us. Conceptually, the `hash_buckets` region contains
one `int32_t` value for each hash bucket, which is the offset within the
`hash_records` region where this hash bucket starts. The offset in
`hash_buckets` for the next bucket tells us where the next hash bucket starts,
and implicitly where the current hash bucket ends.

> Invariant: The hash offsets are sorted in increasing order. Duplicates are
> permitted.

For historical reasons, the values stored in `hash_buckets` are multiplied by
12, not by 8. To find the index of the hash record, you must first divide
`hash_buckets[i]` by 12. To find the byte offset of a hash record, you then
multiply the value by 8. (Apparently the values were multiplied by 12 because
that was the size of the in-memory data structure used for hash buckets, many
years ago.)

> Invariant: The first hash offset is always zero.

> Invariant: Each hash offset is a non-negative multiple of 12.

> Invariant: Each hash offset (divided by 12) must be less than
> `num_hash_records`.

The `hash_buckets` region uses an encoding that minimizes the encoding size of
the hash buckets. It only encodes the hash record offset for hash buckets that
are not empty, because many hash tables will have a large number of empty hash
buckets. The encoding uses a bitmap that indicates which hash buckets are
non-empty (1 in the bitmap) vs. empty (0 in the bitmap).

To encode the `hash_buckets` region, you must know the number of hash buckets
(`num_buckets`). It is not stored directly in the Name Table, or even in the GSI
Header. Instead, it is determined by the presence of the `MinimalDebugInfo`
feature code in the PDB Information Stream. See
[Feature Codes](pdb_stream.md#feature-codes). If the `MinimalDebugInfo` feature
is _absent_, then the value of `num_buckets` is 0x1000. If the
`MinimalDebugInfo` feature is _present_, then the value of `num_buckets` is
0x3ffff.

The `hash_buckets` region consists of a bitmap which indicates which hash
buckets are non-empty, followed by an array of `int32_t` values for the
non-empty hash buckets.

The following is an example of a `hash_buckets` region. The `FF` values (and the
`00 00 00 00` immediately after them) are the "non-empty buckets" bitmap.
Following that are the bucket-index values.

```
00088520 : 01 00 00 00 19 63 5b 00 01 00 00 00 ff ff ff ff
00088530 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088540 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088550 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088560 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088570 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088580 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088590 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885a0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885b0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885c0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885d0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885e0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000885f0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088600 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088610 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088620 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088630 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088640 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088650 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088660 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088670 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088680 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088690 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886a0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886b0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886c0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886d0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886e0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
000886f0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088700 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088710 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088720 : ff ff ff ff ff ff ff ff ff ff ff ff 00 00 00 00
00088730 : 00 00 00 00 d8 00 00 00 d4 01 00 00 88 02 00 00
00088740 : 30 03 00 00 a8 03 00 00 38 04 00 00 40 05 00 00
00088750 : 00 06 00 00 c0 06 00 00 50 07 00 00 d4 07 00 00
```

The first part is the `nonempty_buckets` bitmap. In this example, `num_buckets`
is 4096 (0x1000), which is the default value. The length in bits of
`nonempty_buckets` is equal to `num_buckets`. It is easy to see that all 4096
bits in this table have been set.

However, the size in bytes of `nonempty_buckets` is a bit strange. It relies on
an implementation detail of the PDB reader/writer library. Let `num_buckets` be
the number of hash buckets. Then let
`non_empty_bitmask_size_in_bytes = (num_buckets + 32) / 8`. The C++ PDB library
adds 1 to `num_buckets` and then computes the 32-bit aligned size of the
bitmask, because it stores it in uint32 values. Then it serializes the mask as a
byte array, but it uses the full `uint32_t` array.

In our example, since `num_buckets` is 4096 (0x1000), we compute the size in
bytes of `nonempty_buckets` as 516 (0x204), not 512. You can see the unused 32
bits at the end of the nonempty_buckets bitmap:

```
000886f0 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088700 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088710 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088720 : ff ff ff ff ff ff ff ff ff ff ff ff 00 00 00 00
00088730 : 00 00 00 00 d8 00 00 00 d4 01 00 00 88 02 00 00
00088740 : 30 03 00 00 a8 03 00 00 38 04 00 00 40 05 00 00
00088750 : 00 06 00 00 c0 06 00 00 50 07 00 00 d4 07 00 00
```

Immediately following this bitmap is the array of hash record offsets that start
each bucket. This highlights the first 16 hash bucket values:

```
00088710 : ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff ff
00088720 : ff ff ff ff ff ff ff ff ff ff ff ff 00 00 00 00
00088730 : 00 00 00 00 d8 00 00 00 d4 01 00 00 88 02 00 00
00088740 : 30 03 00 00 a8 03 00 00 38 04 00 00 40 05 00 00
00088750 : 00 06 00 00 c0 06 00 00 50 07 00 00 d4 07 00 00
00088760 : b8 08 00 00 9c 09 00 00 80 0a 00 00 4c 0b 00 00
00088770 : 54 0c 00 00 38 0d 00 00 58 0e 00 00 dc 0e 00 00
00088780 : 78 0f 00 00 68 10 00 00 1c 11 00 00 c4 11 00 00
00088790 : d8 12 00 00 10 14 00 00 d0 14 00 00 18 15 00 00
```

In table format:

Bytes | Hex | Decimal,<br>divided by 12 | Difference from next bucket offset<br>(number of hash records in this bucket)
--------------|------------|---|----
`00 00 00 00` | `00000000` |   0 | 18
`D8 00 00 00` | `000000D8` |  18 | 21
`D4 01 00 00` | `000001D4` |  39 | 15
`88 02 00 00` | `00000288` |  54 | 14
`30 03 00 00` | `00000330` |  68 | 10
`A8 03 00 00` | `000003A8` |  78 | 12
`38 04 00 00` | `00000438` |  90 | 22
`40 05 00 00` | `00000540` | 112 | 16
`00 06 00 00` | `00000600` | 128 | 16
`C0 06 00 00` | `000006C0` | 144 | 12
`50 07 00 00` | `00000750` | 156 | 11
`D4 07 00 00` | `000007D4` | 167 | 19
`B8 08 00 00` | `000008B8` | 186 | ...
...           | ...        | ... | ...

# Building the Name Table

To build a GSI Name Table, you need to have the encoded byte stream of the
Global Symbol Stream (GSS) and know the number of hash buckets to use
(num_buckets).

1. Inputs:
   1. Let `gss` be the Global Symbol Stream.
   1. Let `num_buckets` be the number of hash buckets (usually 4096).
   1. Let `output_bytes` be the byte stream that is the output of this function.
2. Let `hash_names_temp` be a vector (array) and initialize it to empty. Each
   record in `hash_names_temp` holds the byte offset within the GSS and the hash
   code of a symbol.
3. Scan through the GSS and locate all relevant public symbols. This is
   `S_PUB32` for the PSI, and `S_GPROC32` etc. for the GSI. Compute the hash
   code for the symbol’s name. Add an entry to `hash_names_temp` for this
   symbol, containing the hash code and the byte offset of this record.
4. Sort `hash_names_temp` in order of increasing hash code, and then increasing
   byte offset.
5. Let `hash_records_size = 8 * length of hash_names_temp`.
6. Let `hash_buckets` be a vector of `int32_t`, whose length is `num_buckets`.
   Scan through `hash_names_temp` and find the pairs of consecutive records
   where the hash code (bucket index) is different. Use this to initialize
   hash_buckets to the starting offset of each bucket.
7. Let `nonempty_buckets` be a bitmap, whose length is `num_buckets`. Initialize
   it so that each entry `nonempty_bucket[j]` is 1 if the corresponding value in
   `hash_buckets[j]` is less than `hash_buckets[j – 1]` (indicating that it is
   not empty). For the last `hash_buckets[j]`, compare it to `num_hash_records`.
8. Let `num_nonempty_buckets` = count the number of bits in nonempty_buckets
   that are 1.
9. Let `nonempty_buckets_size = (num_buckets + 32) / 32 * 4`.
10. Let `hash_buckets_size = nonempty_buckets_size + num_nonempty_buckets * 4`.
11. Write the Name Table Header to `output_bytes`, using `hash_records_size` and
    `hash_buckets_size` as computed above.
12. Write the hash records to output_bytes. For each hash record, write a
    `HashRecord`, containing the symbol’s GSS stream offset and setting
    `c_ref = 1`.
13. Write the `nonempty_buckets` bitmap to `output_bytes`. Ensure that exactly
    `nonempty_buckets_size` bytes are written.
14. Scan through `hash_buckets` and write the offset for those hash buckets that
    are not empty. This should write exactly `num_nonempty_buckets * 4` bytes.

# Loading the Name Table

The description above should be sufficient for decoding the GSI Name Table. The
only interesting bit is decoding the hash buckets, using the nonempty_buckets
bitmap. The goal is to produce a hash_buckets array that contains non-decreasing
values. If the nonempty_buckets bitmap indicates that a bucket is empty, then
its offset is equal to the offset of the next bucket.

# Querying the Name Table

Each query begins with a symbol name to locate.

1. Compute the hash of the symbol name, using the PDB Hash Function (V1). This
   gives the bucket index. Let `h` be the hash value.
2. Use the hash_buckets table to find the hash records that are in this hash
   bucket. The starting index is given by hash_buckets[h] and the ending
   (exclusive bound) index is given by `hash_buckets[h + 1]`. In Rust syntax,
   the records are: `&hash_records[hash_buckets[h] .. hash_buckets[h + 1]]`.
3. For each hash record in the hash bucket:
   1. Use the symbol offset value in the hash record to look up the symbol in
      the GSS. It is a byte offset (biased by 1).
   1. Read / decode the symbol record.
   1. Compare the string to the name being searched for. If it matches, the
      query terminates (succeeds).
4. If no matches are found, then there is no global symbol with that name.

# Global Symbol Index (GSI) Stream

The Global Symbol Index (GSI) Stream provides a name-to-symbol lookup table for
global symbols that have a name. The GSI contains entries only for the following
symbol kinds:

Record Kind<br>(hex) | Record Kind
---------------------|------------
0x1107               | `S_CONSTANT`
0x1108               | `S_UDT`
0x110C               | `S_LDATA32`
0x110D               | `S_GDATA32`
0x1113               | `S_GTHREAD32`
0x1127               | `S_LPROCREF`
0x1128               | `S_ANNOTATIONREF`
0x1129               | `S_TOKENREF`

Note that `S_PUB32` is **not** included in this list. `S_PUB32` symbols are
indexed in the PSI, not the GSI.

The GSI Stream simply contains an instance of the Name Table described above.
The GSI Stream does not have a header of its own; instead, it starts immediately
with the Name Table. The GSI Stream does not have an address lookup table.

# Public Symbol Index (PSI) Stream

The Public Symbol Index (PSI) provides several look-up tables that accelerate
finding information in the Global Symbol Stream. The PSI indexes only `S_PUB32`
symbols in the GSS; all other symbol kinds are indexed in the GSI.

* Name-to-symbol table: This table allows finding global symbols using a symbol
  name.
* Address-to-symbol table: This table allows finding global symbols using a
  symbol address. The address is represented as a segment and an offset within a
  segment.
* Thunk table: TODO

The PSI stream has this structure:

```
struct PsiStream {
  PsiStreamHeader header;
  uint8_t name_table[];
  uint8_t address_table[];
};
```

`name_table` contains the name-to-symbol lookup table and is specified by the
`SymbolNameTable` structure, above. This section immediately follows the header
and is variable-length. Its size is specified in the `PsiStreamHeader`.

`address_table` contains the address-to-symbol lookup table. This section
immediately follows `name_table` and is variable-length. Its size is specified
in the `PsiStreamHeader`.

`PsiStreamHeader` has this structure:

```
struct PsiStreamHeader {
    uint32_t name_table_size;  // Size in bytes of name_table
    uint32_t addr_table_size;  // Size in bytes of address_table
    uint32_t num_thunks;       // The number of thunk records
    uint32_t thunk_size;       // Size in bytes of each thunk record.
    uint16_t thunk_table_section;
    uint16_t padding;
    uint32_t thunk_table_offset;
    uint32_t num_sections;
};
```

> TODO: Specify the thunk table.

Example PSI Stream Header:

```
00000000 : 14 c7 08 00 80 42 04 00 00 00 00 00 00 00 00 00
00000010 : 00 00 00 00 00 00 00 00 00 00 00 00 ff ff ff ff
00000020 : 1a 09 2f f1 00 85 08 00 04 42 00 00 21 21 66 00
00000030 : 01 00 00 00 65 2a 92 00 01 00 00 00 49 07 70 00
00000040 : 01 00 00 00 29 c9 75 00 01 00 00 00 95 60 62 00
00000050 : 01 00 00 00 5d d8 74 00 01 00 00 00 ad e4 89 00
```

## Address Table

The Address Table allows finding a global symbol based on its address. The size
in bytes of the Address Table is specified in the PSI Stream Header as
`addr_table_size`.

The PSI Address Table is an array of `int32` values. Let
`num_addr_records = addr_table_size / 4`. Each value is the byte offset of a
symbol record in the GSS.

The values in the Address Table are in a sorted order. The order is determined
by the `(segment, offset)` values of the `S_PUB32` symbols that are referenced
by the GSI Address Table. The address records are not sorted by their own value;
the sorting algorithm must use the address record to dereference the GSS in
order to compare entries.

The entries in the Address Map are not sorted by their value, but by the `name`
field of the global symbol that those values point to. That is, sorting must
dereference the pointer (stream offset) into the GSS.

It is meaningless for a value (GSS stream offset) to appear more than once in
the Address Map. This is not stated as an invariant; it would not be harmful for
a value to be repeated, but it would have no benefit.

> Determinism: The values in the Address Map are unique.

This is an example of the GSI Address Table, starting at 0x8_C730:

```
0008c710 : 8c c1 0c 00 f8 c1 0c 00 d0 c2 0c 00 b4 c3 0c 00 : ................
0008c720 : 44 c4 0c 00 64 c5 0c 00 b8 c5 0c 00 9c c6 0c 00 : D...d...........
0008c730 : a8 d5 58 00 9c 29 8c 00 fc 74 8e 00 80 d8 68 00 : ..X..)...t....h.
0008c740 : c0 59 50 00 38 1a 66 00 8c 4b 8b 00 e4 26 6c 00 : .YP.8.f..K...&l.
0008c750 : 5c 5c 63 00 34 69 6c 00 c4 31 8c 00 14 0f 7b 00 : \\c.4il..1....{.
0008c760 : 88 69 70 00 a4 fd 7b 00 5c 9b 7a 00 c0 c2 5c 00 : .ip...{.\.z...\.
0008c770 : 6c 56 79 00 18 24 55 00 a0 16 59 00 5c e8 5a 00 : lVy..$U...Y.\.Z.
0008c780 : 48 19 65 00 f0 5f 70 00 40 6c 5a 00 50 33 76 00 : H.e.._p.@lZ.P3v.
0008c790 : d0 ed 7a 00 10 01 51 00 38 ea 61 00 48 6f 59 00 : ..z...Q.8.a.HoY.
```

The first few offsets in this table:

Bytes         | Hex value  | Symbol name
--------------|------------|------------
`A8 D5 58 00` | `0058D5A8` | `EnumSystemGeoID`
`9C 29 8C 00` | `008C299C` | `DeleteStateAtomValue`
`FC 74 8E 00` | `008E74FC` | `?DeleteSetting@StateAtom@@QEAAJPEBG@Z`

We can manually find these records in the GSS table:

```
0058d5a0 : 61 63 6b 61 67 65 40 00 1e 00 0e 11 02 00 00 00 : ackage@.........
0058d5b0 : 10 00 00 00 01 00 45 6e 75 6d 53 79 73 74 65 6d : ......EnumSystem
0058d5c0 : 47 65 6f 49 44 00 00 00 42 00 0e 11 02 00 00 00 : GeoID...B.......
...
008c2990 : 65 61 64 42 61 74 63 68 00 00 00 00 22 00 0e 11 : eadBatch...."...
008c29a0 : 02 00 00 00 f0 00 00 00 01 00 44 65 6c 65 74 65 : ..........Delete
008c29b0 : 53 74 61 74 65 41 74 6f 6d 56 61 6c 75 65 00 00 : StateAtomValue..
...
008e74f0 : 6d 61 70 68 6f 72 65 45 78 41 00 00 32 00 0e 11 : maphoreExA..2...
008e7500 : 02 00 00 00 70 01 00 00 01 00 3f 44 65 6c 65 74 : ....p.....?Delet
008e7510 : 65 53 65 74 74 69 6e 67 40 53 74 61 74 65 41 74 : eSetting@StateAt
008e7520 : 6f 6d 40 40 51 45 41 41 4a 50 45 42 47 40 5a 00 : om@@QEAAJPEBG@Z.
008e7530 : 3a 00 0e 11 00 00 00 00 b8 c3 15 00 02 00 3f 3f : :.............??
```

## Querying the PSI Address Table

Querying the Address Table is simply a binary-search process. Use a standard
binary-search algorithm to search the Address Table, but keep in mind that the
Address Table does not directly contain the field that is being tested for the
binary search (the search key). The search key is the `segment:offset` value
stored in the `S_PUB32` record. Each entry in the Address Table contains the
offset within the GSS that contains the `S_PUB32`, so the binary search needs to
use that offset to dereference the GSS.

## Pointer relationships

The GSI and PSI both contain pointers (stream offsets) into the Global Symbol
Stream (GSS). Therefore, if symbol records within the GSS are moved, added, or
deleted, the GSI and PSI will both need to be reconstructed.

The GSS contains `TypeIndex` values that point into the TPI Type Stream. If type
records in the TPI are moved (added, deleted, etc.), then the GSS will need to
updated. If this update also reorders symbol records in the GSS, then the GSI
and PSI will also need to be rebuilt.

* GSS contains `TypeIndex` values &rarr; TPI Type Stream
* GSS contains Module index values &rarr; DBI Modules Substream
* GSS contains byte offsets that point into Module symbol streams &rarr; Module
  Streams
 