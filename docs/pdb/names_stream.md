- [Names Stream](#names-stream)
  - [`NameIndex`](#nameindex)
  - [Hash Table](#hash-table)
  - [Querying the hash table](#querying-the-hash-table)
  - [Building](#building)
  - [Determinism](#determinism)
  - [Example](#example)
  - [References](#references)

# Names Stream

The Names Stream stores a set of unique strings (names). This allows other data
structures to refer to strings using an integer index, rather than storing
copies of the same string in many different places.

The stream index for the Names Stream is found in the PDB Information Stream, in
the Named Streams section. The key is `/names`.

The Names Stream has this structure:

```
struct NamesStream {
    uint32_t signature;
    uint32_t version;
    uint32_t strings_size;
    uint8_t string_data[strings_size];

    // Hash table
    uint32_t num_hashes;                // 'cit' in nmt.h
    NameIndex hash_table[num_hashes];   // `mphashni` in nmt.h
    uint32_t num_names;                 // 'cni' in nmt.h 
}
```

The `signature` field should always be 0xEFFE_EFFE. The `version` field can take
on one of these values:

Name | Value | Description
-----|-------|------------
`V1` |     1 | Uses `LHashPbCb` for hashing strings.
`V2` |     2 | Uses `LHashPbCbV2` for hashing strings.

All observed PDBs use version `V1`.

`strings_size` specifies the size of `string_data`, in bytes. There is no
alignment requirement for `strings_size`.

`string_data` contains the strings stored in each table. Each string is a
NUL-terminated UTF-8 string. These strings are identified by absolute stream
offset from other tables. The `strings_size` field indicates the size in bytes
of the string data that follows the header. Because the size of `string_data`
must be a multiple of 4, there may be padding bytes at the end of `string_data`.

The first string in `string_data` is the empty string. This allows a `NameIndex`
value of 0 to mean "the empty string" without consulting the table and without
needing to subtract 1 from `NameIndex` values before indexing into
`string_data`. This empty string is _not_ counted in `num_names`.

## `NameIndex`

A `NameIndex` value is a `uint32_t` that points into the Names Stream, starting
at the `string_data` array. `NameIndex` identifies strings by location. By
convention, the empty string is always stored first, which means that the
`NameIndex` value 0 always means the empty string.

There are no guarantees made about `NameIndex` values pointing into the middle
of a string. We have not observed any PDBs that contain string indexes that
point to the middle of strings. Decoders should be prepared to handle pointers
to the middle of strings. Encoders should conservatively avoid generating
pointers to the middle of strings, even if this requires duplicating some string
data.

These are the known locations in other parts of the PDB that contain `NameIndex`
values. If the `NameIndex` table is modified (existing strings are moved), then
the `NameIndex` values that point to them will need to be updated to point to
the new location.

* [C13 Line Data](line_data.md), File Checksums Subsection. Each File Checksum
  contains a `NameIndex` for the file name.
* `LF_UDT_SRC_LINE` and `LF_MOD_UDT_SRC_LINE` records (in the IPI Stream)
  contain `NameIndex` values that point to file names.

## Hash Table

The Names Stream contains a hash table which accelerates finding strings. The
hash table is stored in `hash_table`; each entry contains a `NameIndex` value or
0 to indicate that the slot is not used. `num_hashes` specifies the number of
elements in the `hash_table` array. The hash table uses
[open addressing](https://en.wikipedia.org/wiki/Open_addressing), also known as
linear probing, to resolve collisions.

`num_names` value specifies the number of names in the table. This value _does
not_ count the empty string at the start of `string_data`.

> TODO: How does this handle hashes that are zero? How do we distinguish an
> empty hash slot from a hash slot that points to a string whose hash code is
> zero?

> Invariant: `num_hashes` must be greater than or equal to `num_names`.

## Querying the hash table

Querying the table begins with a query string `q` and returns either "not found" or the `NameIndex` value that points to `q`.

1. Let `q` be the query string. `q` cannot be the empty string.
2. Let `h = hash(q)`, where `hash` is the hash function implied by `version`.
3. Let `p = h`, where `p` is the current probe location.
4. Loop:
   1. Let `i = hash_table[p]`.
   2. If `i` is zero, then `q` is not in the table, and the search halts with
      "not found".
   3. Read the string at `string_data[i]` and compare it to `q`. If it is equal,
      then we have found the string in the table, and the search halts with
      "found" with a `NameIndex` value of `i`.
   4. Let `i = (i + 1) % num_hashes`.
   5. If `i == h`, then the table is completely full and does not contain `q`;
      the search halts with "not found". Else, continue the loop.

## Building

This section describes how to build the byte representation for a Names Stream.

1. Input: The input is an ordered list of UTF-8 strings. Let `num_strings` be
   the number of strings in the list.
   * Determinism: There are no duplicates in this list.
   * Determinism: The list is sorted. The empty string is the first element in
     the list.

2. Choose a value for `num_hashes`. This value must be greater than or equal to
   `num_strings`. Generally, the larger this number is, the fewer hash
   collisions there will be, but the more space is taken up with unused hash
   slots.

3. Let hashes be a new array for the hash table. Each element is a `uint32_t`
   value and the length of the table is `num_hashes`. Initialize all elements to
   0.

4. Iterate through the set of strings to add to the table. For each string S:
   * Compute `H = hash(S) % num_hashes`.
   * Loop:
     + If `hashes[H] == 0`, then set `hashes[H]` to the `NameIndex` of this
       string. Remember that the `NameIndex` is the byte offset into the strings
       data section for this string.  Exit this loop.
     + If `hashes[H] != 0`, then set `H = (H + 1) % num_hashes` and continue
       this loop.

The requirement that `num_hashes >= num_strings` guarantees that the inner loop
will terminate.

## Determinism

> Determinism: `NameIndex` values should never point into the "middle" of a
> string. If a `NameIndex` value `n` is non-zero, then
> `string_data[n - 1] == 0` (i.e. the previous string has been terminated).

> Determinism: The strings in `string_data` are sorted and are unique.

> Determinism: If `string_data` has padding bytes at the end, then they have the
> value 0.

Strings should be written in a deterministic order, with case-sensitive sorting
being the most obvious option. Take care to ensure that the empty string is
still placed at the start of the stream (at stream offset 8). Duplicate strings
should be eliminated.

## Example

This is the beginning of a Names Stream, showing the header and the first part
of `string_data`. Note the `strings_size` field, which is `c9 13 07 00`, or
0x713c9.

```
00000000 : fe ef fe ef 01 00 00 00 c9 13 07 00 00 6f 6e 65 : .............one
00000010 : 63 6f 72 65 5c 65 78 74 65 72 6e 61 6c 5c 64 64 : core\external\dd
00000020 : 6b 5c 69 6e 63 5c 6e 74 70 6f 61 70 69 2e 68 00 : k\inc\ntpoapi.h.
00000030 : 6f 6e 65 63 6f 72 65 5c 69 6e 74 65 72 6e 61 6c : onecore\internal
00000040 : 5c 73 64 6b 5c 69 6e 63 5c 6e 74 70 73 61 70 69 : \sdk\inc\ntpsapi
00000050 : 5f 78 2e 68 00 6f 6e 65 63 6f 72 65 5c 69 6e 74 : _x.h.onecore\int
00000060 : 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c 6e 74 : ernal\sdk\inc\nt
00000070 : 74 70 61 70 69 2e 68 00 6f 6e 65 63 6f 72 65 5c : tpapi.h.onecore\
00000080 : 69 6e 74 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 : internal\sdk\inc
00000090 : 5c 6e 74 70 73 61 70 69 2e 68 00 6f 6e 65 63 6f : \ntpsapi.h.oneco
000000a0 : 72 65 5c 69 6e 74 65 72 6e 61 6c 5c 73 64 6b 5c : re\internal\sdk\
000000b0 : 69 6e 63 5c 72 74 6c 66 65 61 74 75 72 65 63 6f : inc\rtlfeatureco
000000c0 : 6e 66 69 67 74 79 70 65 73 2e 68 00 6f 6e 65 63 : nfigtypes.h.onec
000000d0 : 6f 72 65 5c 65 78 74 65 72 6e 61 6c 5c 64 64 6b : ore\external\ddk
000000e0 : 5c 69 6e 63 5c 6e 74 69 6d 61 67 65 2e 68 00 6f : \inc\ntimage.h.o
000000f0 : 6e 65 63 6f 72 65 5c 69 6e 74 65 72 6e 61 6c 5c : necore\internal\
00000100 : 73 64 6b 5c 69 6e 63 5c 6e 74 65 78 61 70 69 2e : sdk\inc\ntexapi.
00000110 : 68 00 6f 6e 65 63 6f 72 65 5c 69 6e 74 65 72 6e : h.onecore\intern
00000120 : 61 6c 5c 73 64 6b 5c 69 6e 63 5c 6e 74 78 63 61 : al\sdk\inc\ntxca
00000130 : 70 69 5f 78 2e 68 00 6f 6e 65 63 6f 72 65 5c 69 : pi_x.h.onecore\i
00000140 : 6e 74 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c : nternal\sdk\inc\
```

The end of the `strings_data` is at 0x713c9 + 0xc (where 0xc is the size of the
header), which is 0x713d5, we find the `num_hashes` and `hash_table` fields:

```
00071370 : 74 2d 6d 73 2d 6f 6e 65 63 6f 72 65 2d 61 70 70 : t-ms-onecore-app
00071380 : 6d 6f 64 65 6c 2d 73 74 61 74 65 72 65 70 6f 73 : model-staterepos
00071390 : 69 74 6f 72 79 2d 69 6e 74 65 72 6e 61 6c 2d 6c : itory-internal-l
000713a0 : 31 2d 31 2d 36 5f 71 75 65 72 79 2e 6f 62 6a 00 : 1-1-6_query.obj.
000713b0 : 6d 69 6e 6b 65 72 6e 65 6c 5c 6b 65 72 6e 65 6c : minkernel\kernel
000713c0 : 62 61 73 65 5c 61 6d 64 36 34 5c 66 69 62 65 72 : base\amd64\fiber
000713d0 : 2e 61 73 6d 00 68 2f 00 00 00 00 00 00 00 00 00 : .asm.h/.........   <-- num_hashes is 68 2f 00 00
000713e0 : ... zeroes ...
000713f0 : 00 7a 0f 01 00 00 00 00 00 00 00 00 00 00 00 00 : .z..............
00071400 : ... zeroes ...
00071410 : 00 43 29 05 00 bd a3 04 00 e1 cc 00 00 09 46 03 : .C)...........F.
00071420 : 00 ed f0 04 00 fd d7 03 00 31 ff 04 00 1c 42 05 : .........1....B.
00071430 : 00 cf 60 02 00 a2 78 06 00 af f0 00 00 6b b6 01 : ..`...x......k..
00071440 : 00 98 3e 03 00 22 98 00 00 00 00 00 00 00 00 00 : ..>.."..........
00071450 : 00 00 00 00 00 00 00 00 00 00 00 00 00 d2 01 03 : ................
00071460 : 00 00 00 00 00 b3 b8 01 00 00 00 00 00 24 aa 01 : .............$..
00071470 : 00 e5 e7 01 00 a4 28 04 00 c9 12 05 00 13 de 02 : ......(.........
00071480 : 00 73 8f 03 00 c5 ad 00 00 00 00 00 00 00 00 00 : .s..............
00071490 : 00 00 00 00 00 00 00 00 00 00 00 00 00 5c 88 00 : .............\..
000714a0 : 00 01 e5 00 00 00 00 00 00 c3 69 03 00 cc 43 03 : ..........i...C.
000714b0 : 00 25 f5 06 00 0d dd 03 00 60 80 00 00 9e e7 05 : .%.......`......
000714c0 : 00 d2 35 03 00 00 00 00 00 00 00 00 00 00 00 00 : ..5.............
```

We find that `num_hashes` is 0x2f68. The first few entries in `hash_table` are
given in the table below; entires with zero values are omitted. Remember that
the values are `NameIndex` values, which point into `string_data`.

Stream Offset | `NameIndex`
--------------|------------
000713f1      | 00010f7a
00071411      | 00052943
00071415      | 0004a3bd
00071419      | 0000cce1
0007141d      | 00034609

Remember that the stream offset of the `string_data` field is 12 (the size of
the fields that precede `string_data`), so we add 12 to each of these
`NameIndex` values and look up their strings. For example, at 0x10f7a + 12 =
0x10f86, we find:

00010f80 : 61 70 69 2e 68 00 6f 6e 65 63 6f 72 65 5c 65 78 : api.h.onecore\ex
00010f90 : 74 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c 69 : ternal\sdk\inc\i
00010fa0 : 73 6f 6c 61 74 69 6f 6e 2e 68 00 6f 6e 65 63 6f : solation.h.oneco

So the first string, whose `NameIndex` is 0x10f7a, decodes as
`onecore\external\sdk\inc\isolation.h`.

## References

* [`class NMT`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/PDB/include/nmt.h#L26)
