
# DBI Sources Substream

The DBI Sources Substream provides a list of the source files that were read when compiling each module. Because many executables contain modules that read from the same source files (such as header files), the DBI Sources substream only lists each unique source file once. It stores a list of file names that were read in each compiland, using offsets into a string table to avoid storing the file names more than once.

The size and location of the DBI Sources Substream is specified in the DBI Stream Header. The size is specified explicitly; its location is computed by summing the sizes of the substreams that precede it.

> Invariant: The starting byte offset of the DBI Sources Substream is a multiple of 4. The starting byte offset is computed from `module_info_size + section_contributions_size + section_map_size`, from the DBI Stream Header. Each of those sections has a corresponding invariant that those sizes are multiplies of 4.

> Invariant: The size of the DBI Sources Substream (`source_info_size`) is a multiple of 4.

The DBI Sources Substream has this structure:

```
struct DbiSourcesSubstream {
   uint16_t num_modules;
   uint16_t num_sources;                        // obsolete; do not read
   uint16_t module_file_starts[num_modules];
   uint16_t module_file_counts[num_modules];
   uint32_t file_offsets[num_file_offsets];
   uint8_t names_buffer[];
   uint8_t alignment_padding[];                 // indistinguishable from `names_buffer`
};
```

`num_modules` specifies the number of modules in this substream. The order of module records in the DBI Modules Substream corresponds to the order of module records in the DBI Sources Substream.

> Invariant: `num_modules` is equal to the number of Module info records in the [DBI Modules Substream](dbi_modules.md).

The size of the `file_offsets` array should be computed by summing the values in the `module_file_counts` array.
Let `num_file_offsets` be the sum of all values in `module_file_counts`.  This is not the number of _unique_ file names; it is the length of the `file_offsets` array. `file_offsets` will usually contain duplicates because different modules will often include the same header files.

The `num_sources` field is obsolete and must be ignored when reading the DBI Sources Substream. In the past, it specified the number of entries in the `file_offsets` array. However, because the field only has 16 bits of precision, and many PDBs exist that link together modules that read from more than 2^16 source files, it can no longer be used as an accurate count of the number of source files in the stream. Encoders should set `num_sources` to the truncated value (the lower 16 bits) of the length of `num_file_offsets`.

The `module_file_starts` and `module_file_counts` arrays, taken together, specify the range of values within `file_offsets` that correspond to each module. For a module `m`, `module_file_starts[m]` is the index within `file_offsets` of the first file in the list of files for `m` and `module_file_counts[m]` specifies the number of files for `m`.  In Rust notation, `file_offsets[module_file_starts[m] .. module_file_starts[m] + module_file_counts[m]]` is the slice of `file_offsets` for `m`.

> Invariant: `module_file_starts[m] + module_file_counts[m] <= num_file_offsets` for all `m` in `0..num_modules`.

> Guideline: Every entry in `file_offsets` is covered by exactly one module. There are no unused gaps and no entries covered by more than one module.

The following determinism requirements state this more directly. They ensure that every entry in `file_offsets` is used by exactly one module, and that the order of the per-module ranges within `file_offsets` follows the same order as the modules themselves.

> Determinism: `module_file_starts[0] == 0`.

> Determinism: `module_file_starts[i] = module_file_starts[i - 1] + module_file_counts[i - 1]` for all `i > 0`.

`file_offsets` contains offsets into the names_buffer array. The values are relative to the start of `names_buffer`; e.g. a value of zero references the first character in `names_buffer`. The values stored in `file_name_offsets` are organized as a set contiguous sequences, where each sequence corresponds to one module and the length of that sequence is specified in module_file_counts.

`names_buffer` contains the character data for the file names. Each string is UTF-8 and is NUL-terminated. Values in the `file_name_offsets` array point to the start of strings in `names_buffer`.

In all observed PDBs the values in `file_name_offsets` always point to the start of a string, never to the middle of a string.  That is, `file_name_offsets[i]` is either 0, or `names_buffer[file_name_offsets[i] – 1] == 0`. PDB writers should adhere to this requirement, but it is not stated as an invariant. PDB readers should be prepared for decoding DBI Sources Substreams where file name offsets point into the middle of strings.

Decoders _should not_ assume that strings within `names_buffer` have no gaps between them. An encoder could place strings at any offset within `names_buffer`, as long as `file_offsets` points to valid offsets within `names_buffer`. This is implied by the fact that there is alignment padding at the end of `DbiSourcesSubstream` and that nothing indicates where `names_buffer` ends and the alignment padding begins.  Decoders should only decode strings that correspond to an entry in `file_offsets`.

## Example

This is an example of the start of a DBI Sources Substream. In this example, the DBI Sources Substream begins at stream offset 0x1ee4c0, which is aligned to our 16-byte rows, so all of the data shown is from the DBI Sources Substream.

```
001ee4c0 : 15 09 2f d0 00 00 01 00 06 00 63 00 bf 00 c2 00 : ../.......c.....
001ee4d0 : 14 01 14 01 18 01 71 01 cc 01 27 02 8a 02 3a 03 : ......q...'...:.
001ee4e0 : 9a 03 fa 03 a8 04 56 05 b8 05 18 06 b2 06 73 07 : ......V.......s.
001ee4f0 : 16 08 e6 08 8f 09 8f 09 38 0a d3 0a d3 0a 86 0b : ........8.......
001ee500 : 8a 0b e9 0b 48 0c 9c 0c 42 0d e8 0d 8e 0e 54 0f : ....H...B.....T.
001ee510 : fc 0f a4 10 4c 11 f4 11 9c 12 44 13 ec 13 94 14 : ....L.....D.....
001ee520 : 3c 15 e4 15 8c 16 32 17 d8 17 7e 18 24 19 ca 19 : <.....2...~.$...
001ee530 : 70 1a 16 1b bc 1b 62 1c 08 1d ae 1d 54 1e 1b 1f : p.....b.....T...
001ee540 : dd 1f 9f 20 47 21 ef 21 97 22 3f 23 e7 23 8f 24 : ... G!.!."?#.#.$
001ee550 : 37 25 df 25 87 26 2f 27 d7 27 7f 28 27 29 cf 29 : 7%.%.&/'.'.(').)
001ee560 : 77 2a 1e 2b c5 2b 6c 2c 12 2d c0 2d 6e 2e 1c 2f : w*.+.+l,.-.-n../
001ee570 : cb 2f 7a 30 29 31 d8 31 87 32 36 33 e5 33 90 34 : ./z0)1.1.263.3.4
001ee580 : 3b 35 e6 35 8e 36 50 37 12 38 d4 38 96 39 58 3a : ;5.5.6P7.8.8.9X:
001ee590 : 1a 3b ef 3b c4 3c 99 3d 6e 3e 43 3f e9 3f 90 40 : .;.;.<.=n>C?.?.@
```
 
In this example, `num_modules` is 0x915 (2325) and `num_sources` is 53295 (0xD02F). However, we cannot trust the `num_sources` value, as we will see below when we recompute it from `module_file_counts`.

Using the `num_modules` value (0x915), we can find the offsets of the `module_file_counts`, `module_file_starts`, and `file_name_offsets` fields:

Field                | Stream offset | Size expression                       | Size value
---------------------|---------------|---------------------------------------|-----------
`num_modules`        | 0x001ee4c0    | `sizeof(uint16_t)`                    | 2
`num_sources`        | 0x001ee4c2    | `sizeof(uint16_t)`                    | 2
`module_file_starts` | 0x001ee4c4    | `sizeof(uint16_t) * num_modules`      | 0x122a
`module_file_counts` | 0x001ef6ee    | `sizeof(uint16_t) * num_modules`      | 0x122a
`file_name_offsets`  | 0x001f0918    | `sizeof(uint32_t) * num_file_offsets` | unknown
`names_buffer`       | unknown       | consumes rest of record               | implicit

Because we do not yet know the value of `num_file_offsets` we cannot yet compute the location of `names_buffer`.

Using the stream offsets that we just computed, we can read the first few entries of the `module_file_counts` and `module_file_starts` arrays.

At stream offset 0x1ee4c4 (which is visible in the hex dump above), we see the beginning of the `module_file_starts` array. At stream offset 0x1ef6ee we see the beginning of the `module_file_counts` array:

```
001ef6e0 :                                           01 00 : -.-.-.-.-.-.-...
001ef6f0 : 05 00 5d 00 5c 00 03 00 52 00 00 00 04 00 59 00 : ..].\...R.....Y.
001ef700 : 5b 00 5b 00 63 00 b0 00 60 00 60 00 ae 00 ae 00 : [.[.c...`.`.....
001ef710 : 62 00 60 00 9a 00 c1 00 a3 00 d0 00 a9 00 00 00 : b.`.............
001ef720 : a9 00 9b 00 00 00 b3 00 04 00 5f 00 5f 00 54 00 : .........._._.T.
001ef730 : a6 00 a6 00 a6 00 c6 00 a8 00 a8 00 a8 00 a8 00 : ................
001ef740 : a8 00 a8 00 a8 00 a8 00 a8 00 a8 00 a8 00 a6 00 : ................
001ef750 : a6 00 a6 00 a6 00 a6 00 a6 00 a6 00 a6 00 a6 00 : ................
001ef760 : a6 00 a6 00 a6 00 c7 00 c2 00 c2 00 a8 00 a8 00 : ................
001ef770 : a8 00 a8 00 a8 00 a8 00 a8 00 a8 00 a8 00 a8 00 : ................
001ef780 : a8 00 a8 00 a8 00 a8 00 a8 00 a7 00 a7 00 a7 00 : ................
```

`module_file_starts`<br>stream offset | `module_file_starts[i]`<br>value | `module_file_counts`<br>stream offset | `module_file_counts[i]`<br>value
-----------|--------|------------|-------
0x001ee4c4 | 0x0000 | 0x001ef6ee | 0x0001
0x001ee4c6 | 0x0001 | 0x001ef6f0 | 0x0005
0x001ee4c8 | 0x0006 | 0x001ef6f2 | 0x005d
0x001ee4ca | 0x0063 | 0x001ef6f4 | 0x005c
0x001ee4cc | 0x00bf | 0x001ef6f6 | 0x0003
0x001ee4ce | 0x00c2 | 0x001ef6f8 | 0x0052
0x001ee4d0 | 0x0114 | 0x001ef6fa | 0x0000
0x001ee4d2 | 0x0114 | 0x001ef6fc | 0x0004
0x001ee4d4 | 0x0118 | 0x001ef6fe | 0x0059

Note that each `module_file_starts[i + 1]` can be computed from `module_file_starts[i] + module_file_counts[i]`. This is because the `module_file_starts` values point into the `file_name_offsets` array, and these values are "packed". There is no overlap between the files for different modules and the slice of values for each module have been appended in the same order as the modules themselves.

Now that we have found the `module_file_counts` array, we scan it and compute the sum of all values in it. Let `num_file_offsets` be the sum of all values in `module_file_counts`. In our example, that value is 0x4d02f (315,439). This allows us to find the remaining offsets of our fields:

Field                | Stream offset | Size expression                       | Size value
---------------------|---------------|---------------------------------------|-----------
`num_modules`        | 0x001ee4c0    | `sizeof(uint16_t)`                    | 2
`num_sources`        | 0x001ee4c2    | `sizeof(uint16_t)`                    | 2
`module_file_starts` | 0x001ee4c4    | `sizeof(uint16_t) * num_modules`      | 0x122a
`module_file_counts` | 0x001ef6ee    | `sizeof(uint16_t) * num_modules`      | 0x122a
`file_name_offsets`  | 0x001f0918    | `sizeof(uint32_t) * num_file_offsets` | 0x1340bc
`names_buffer`       | 0x003249d4    | consumes rest of record               | implicit

At 0x1f0918 we find the beginning of `file_name_offsets`:

```
001f0910 :                         4a a7 02 00 29 c5 02 00 : ........J...)...
001f0920 : 57 c5 02 00 8f c5 02 00 b7 c5 02 00 fd 92 02 00 : W...............
001f0930 : 15 1b 00 00 3a 1b 00 00 66 5e 00 00 94 1c 00 00 : ....:...f^......
001f0940 : b7 1c 00 00 db 1c 00 00 5e 1d 00 00 d8 1d 00 00 : ........^.......
001f0950 : ea 1e 00 00 1b 1a 00 00 00 00 00 00 72 1f 00 00 : ............r...
001f0960 : 35 1a 00 00 23 00 00 00 53 5c 00 00 8b 5e 00 00 : 5...#...S\...^..
001f0970 : 77 1a 00 00 f8 1f 00 00 ab 29 00 00 82 13 00 00 : w........)......
```

At 0x3249d4 we find the beginning of `names_buffer`:

```
003249d0 :             6f 6e 65 63 6f 72 65 5c 69 6e 74 65 : )...onecore\inte
003249e0 : 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c 77 61 72 : rnal\sdk\inc\war
003249f0 : 6e 69 6e 67 2e 68 00 6f 6e 65 63 6f 72 65 5c 69 : ning.h.onecore\i
00324a00 : 6e 74 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c : nternal\sdk\inc\
00324a10 : 73 75 70 70 72 65 73 73 5f 78 2e 68 00 6d 69 6e : suppress_x.h.min
00324a20 : 6b 65 72 6e 65 6c 5c 6e 74 6f 73 5c 72 74 6c 5c : kernel\ntos\rtl\
00324a30 : 68 6f 74 70 61 74 63 68 68 65 6c 70 65 72 5c 67 : hotpatchhelper\g
00324a40 : 6c 6f 62 61 6c 73 2e 63 00 6f 6e 65 63 6f 72 65 : lobals.c.onecore
00324a50 : 5c 69 6e 74 65 72 6e 61 6c 5c 73 64 6b 5c 69 6e : \internal\sdk\in
00324a60 : 63 5c 4d 69 6e 57 69 6e 5c 77 6f 77 36 34 74 6c : c\MinWin\wow64tl
00324a70 : 73 2e 68 00 6f 6e 65 63 6f 72 65 5c 65 78 74 65 : s.h.onecore\exte
00324a80 : 72 6e 61 6c 5c 73 64 6b 5c 69 6e 63 5c 4d 69 6e : rnal\sdk\inc\Min
00324a90 : 57 69 6e 5c 6c 69 62 6c 6f 61 64 65 72 61 70 69 : Win\libloaderapi
00324aa0 : 2e 68 00 4f 6e 65 43 6f 72 65 5c 49 6e 74 65 72 : .h.OneCore\Inter
```

Decoding the first few values from `file_name_offsets` and using the value to look up a string in `names_buffer` gives us this:

`i`    | `file_name_offsets[i]`  | Stream offset<br>of string | String
-------|-------------------------|----------------------------|-------
0      | 0x0002a74a              | 0x0034f11e                 | `d:\os\obj\amd64fre\mincore\kernelbase\daytona\objfre\amd64\kernelbase.def`
1      | 0x0002c529              | 0x00350efd                 | `minkernel\tools\gs_support\amd64\amdsecgs.asm`
2      | 0x0002c557              | 0x00350f2b                 | `OneCore\Private\MinWin\Priv_Sdk\Inc\gs\crt_amdsecgs.asm`
3      | 0x0002c58f              | 0x00350f63                 | `onecore\external\shared\inc\ksamd64.inc`
4      | 0x0002c5b7              | 0x00350f8b                 | `onecore\external\shared\inc\kxamd64.inc`

## Invariants

> Invariant: In the DbiStreamHeader, the fields which give the size of substreams must never be negative. If a substream is empty, its length should be zero.

> Invariant: The Sources substream must begin on a 4-byte aligned boundary, and its length must be a multiple of 4.

> Invariant: The size of the entire DBI stream should be equal to the sum of the size of the DbiStreamHeader and all of the substreams.

> Limit: The number of modules is limited to 65,534 (0xfffe). The value 0xffff is not available because it is used to mean "no module" in some data structures.

> Limit: The number of unique source files for a given module is limited to 65,535 (0xffff).

## Determinism

> Determinism: The strings in `names_buffer` are sorted and unique, using case-sensitive rules.

> Determinism: Every string in `names_buffer` is referenced by at least one entry in `file_name_offsets`.

> Determinism: There are no gaps (unused bytes) between strings within `names_buffer`.

> Determinism: Every value in `file_name_offsets` points to the start of a string, not the middle of a string.

> Determinism: For a given sequence of values in `file_name_offsets` that correspond to a single module M, the order of the file name offsets should be deterministic. Since the strings are required to be sorted, the simplest deterministic order would be to sort the file name offsets for M (which is the same as sorting them by the strings they refer to).

> Determinism: The modules themselves should be sorted, but this order is provided by the DBI Modules Substream. 

> Determinism: `num_modules` is set to the low 16 bits of `num_file_offsets`.

> Determinism: Alignment padding bytes are set to zero, if present.
