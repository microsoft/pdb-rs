- [Debug Information Stream (DBI) (Fixed Stream 3)](#debug-information-stream-dbi-fixed-stream-3)
  - [Signature and version](#signature-and-version)
  - [Global symbols](#global-symbols)
  - [Substreams](#substreams)
- [Determinism](#determinism)
- [DBI Type Server Map Substream](#dbi-type-server-map-substream)
- [DBI End-and-Continue Substream](#dbi-end-and-continue-substream)

# Debug Information Stream (DBI) (Fixed Stream 3)
 
The Debug Information Stream (DBI) contains many important fields and substreams. It is a central data structure for PDBs.
The DBI Stream begins with the DBI Stream Header:

```
// sizeof = 64
struct DbiStreamHeader {
    int32_t signature;
    uint32_t version;
    uint32_t age;
    uint16_t global_symbol_index_stream;
    uint16_t build_number;
    uint16_t public_symbol_index_stream;
    uint16_t pdb_dll_version;
    uint16_t global_symbol_stream;
    uint16_t pdb_dll_rbld;
    int32_t module_info_size;                 // size of the Modules Substream
    int32_t section_contributions_size;       // size of the Section Contributions Substream
    int32_t section_map_size;                 // size of the Section Map Substream
    int32_t source_info_size;                 // size of the Sources Substream
    int32_t type_server_map_size;             // size of the Type Server Map Substream
    uint32_t mfc_type_server_index;
    int32_t optional_debug_header_size;       // size of the Optional Debug Header Substream
    int32_t edit_and_continue_size;           // size of the Edit-and-Continue Substream
    uint16_t flags;
    uint16_t machine;
    uint32_t padding;
};
```

## Signature and version

The signature field is always -1 (0xFFFF_FFFF). The version field specifies which version of the DBI stream is being used:

Value (decimal) | Identifier   | Description
----------------|--------------|------------
930803          | `DBIImpvV41` | MSVC 4.1
19960307        | `DBIImpvV50` | MSVC 5.0
19970606        | `DBIImpvV60` | MSVC 6.0
19990903        | `DBIImpvV70` | MSVC 7.0
20091201        | `DBIImpV110` | MSVC 11.0 (current)

This specification covers only version MSVC 11.0+ (`DBIImpV110`).

## Global symbols

The DBI Stream Header contains three fields that point to streams which are
relevant to the Global Symbols Stream.

* The `global_symbol_stream` field is the stream index of the Global Symbol
  Stream (GSS). It contains a sequence of symbol records.

* The `public_symbol_index_stream` field is the stream index of the Public
  Symbol Index (PSI), which contains lookup tables that accelerate finding
  S_PUB32 symbols. It does not directly contain symbol records; instead, it
  contains pointers (byte offsets) that point into the Global Symbol Stream
  (GSS).

* The `global_symbol_index_stream` field is the stream index of the Global
  Symbol Index (GSI), which contains lookup tables that accelerate finding
  global symbols, including `S_CONSTANT`, `S_PROCREF`, and many more. This
  stream does not directly contain symbol records; instead, it contains pointers
  (byte offsets) that point into the Global Symbol Stream (GSS).

For a description of the GSS, GSI, and PSI, see [Global Symbols](globals.md).

## Substreams

The DBI Stream contains several substreams. The size of each substream is
specified in the `DbiStreamHeader`. The byte offset within the DBI Stream of
each substream is given by adding the size of the previous substream to the
start of the previous substream. The byte offset of the first substream (the DBI
Modules Substream) is fixed at 64; it immediately follows the `DbiStreamHeader`.

> Invariant: `module_info_size` is non-negative and is a multiple of 4.

> Invariant: `section_contributions_size` is non-negative and is a multiple of 4.

> Invariant: `section_map_size` is non-negative and is a multiple of 4.

> Invariant: `source_info_size` is non-negative and is a multiple of 4.

> Invariant: The sum of the size of `DbiStreamHeader` and the values of
> substream size fields (`module_info_size`, `section_contributions_size`,
> `section_map_size`, `source_info_size`, `type_server_map_size`,
> `optional_debug_header_size`, and `edit_and_continue_size`) is less than or
> equal to the size of the DBI Stream.

The above invariants imply that the starting byte offset of each these
substreams is a multiple of 4:

* Modules
* Section Contributions
* Section Map
* Sources
* Type Server Map

```
struct DbiStream {
    DbiStreamHeader header;
    uint8_t modules               [module_info_size];
    uint8_t section_contributions [section_contributions_size];
    uint8_t section_map           [section_map_size];
    uint8_t source_info           [source_info_size];
    uint8_t type_server_map       [type_server_map_size];
    uint8_t edit_and_continue     [edit_and_continue_size];
    uint8_t optional_debug_header [optional_debug_header_size];
}
```

The DBI Substreams immediately follow the DBI stream header. Note that the order
of the substreams is similar but not identical to the order of the corresponding
"size" fields. The last two substreams, `edit_and_continue` and
`optional_debug_header_size`, have different orders for the "size" fields and
the corresponding substream.

The byte offset of each substream is found by adding the size and offset of the
preceding substream. The byte offset of the first substream (the Module Info
Substream) is fixed at 64.

The size of the DBI Stream should be equal to the sum of the size of the DBI
Stream Header and the sizes of the substreams. However, it is legal for the sum
of the sizes of the substreams to be less than the size of the DBI Stream.
Decoders should ignore any extra data beyond the end of the last substream.
Encoders should not write any information beyond the last substream.

There are no gaps between the substreams. Some of the DBI substreams have
alignment requirements. These are specified in each of the following sections.

Each substream is described in its own section in this file, or in separate
files.

* [DBI Modules](dbi_modules.md) - Describes the DBI Modules Substream.
* [DBI Sources](dbi_sources.md) - Describes the DBI Sources Substream.
* [DBI Sections](dbi_sections.md) - Describes the DBI Section Map Substream and
  DBI Section Contributions Substream.
* [DBI Optional Debug Headers](dbi_opt_debug.md) - Describes the DBI Optional
  Debug Headers Substream.

# Determinism

Most of the issues concerning determinism are described in each of the DBI
Substreams sections. The DBI Header itself has only a few requirements:

> Determinism: Assign stream indexes in a deterministic order.

This applies to all stream indexes that are listed in the DBI and its
substreams. The exact stream index does not matter; all that matters is that the
stream index is chosen deterministically.

> Determinism: Set the `padding` field in the DBI Stream Header to zero.

# DBI Type Server Map Substream

The DBI Type Server Map Substream is not specified and is likely obsolete.
Decoders should ignore it. Encoders should set its size to zero.

# DBI Edit-and-Continue Substream

The DBI Edit-and-Continue Substream is not specified. Decoders should ignore it.
Encoders should set its size to zero.
