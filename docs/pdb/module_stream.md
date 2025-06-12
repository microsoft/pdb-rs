- [Module Streams](#module-streams)
- [Line Data (C13)](#line-data-c13)
- [Module Symbols](#module-symbols)
- [Global Refs](#global-refs)
 
# Module Streams

Each PDB may contain multiple Module Streams. In this context, a "module" is a
translation unit -- a single PE/COFF object file. When the MSVC linker produces
a PE/COFF executable, it finds all of the object files (modules) that are needed
to resolve symbol references and merges them into a single executable image. The
linker records the contributions of each module in a Module Stream.

Each module may have its own Module Stream. Modules are not required to have a
Module Stream; a "stripped" (public) PDB is a PDB whose module streams have been
removed. Different modules do not share a Module Stream; it is a 1:1
relationship (or 1:none).

The [DBI Modules Substream](dbi.md#dbi-modules-substream) describes the modules
within an executable. For each module there is a `ModuleInfo` record which
specifies the stream index of the Module Stream and the size of the substreams
within the Module Stream. The `ModuleInfo` record is necessary to locate the
Module Stream and its substreams.

Each Module Stream has this structure:

```
struct ModuleStream {
    uint8_t symbols[sym_byte_size];
    uint8_t c11_lines[c11_byte_size];
    uint8_t c13_lines[c13_byte_size];
    uint8_t global_refs[];
}
```

where `sym_byte_size`, `c11_byte_size`, and `c13_byte_size` are taken from
`ModuleInfo` record.

# Line Data (C13)

Module Streams may contain line number information, which describes the mapping
between source locations and instruction streams.

`c11_lines` contains C11 Line Data. C11 Line Data is obsolete and will not be
specified in this document. This field is expected to be empty in all modern
PDBs.

`c13_lines` contains C13 Line Data. See [C13 Line Data](line_data.md) for the
specification of the contents of this substream.

# Module Symbols

`sym_byte_size` specifies the length of the symbols substream for this module.
The symbols stream has a `uint32_t` (4-byte) prefix, followed by symbols
records.

# Global Refs

`global_refs` contains the locations of global symbols that are relevant to this
module. Global Refs is an extension, and was added in such a way that older
tools could safely ignore it. It is legal for `global_refs` to have a length of
zero, meaning this Module Stream does not contain Global Refs.

If Global Refs is present, then it has this structure:

```
struct GlobalRefs {
    uint32_t size;                     // size in bytes of the global refs table
    uint32_t global_refs[size / 4];
}
```

Each value in `global_refs` is a byte offset into the [Global Symbol Stream](globals.md). Each entry identifies a global symbol that is relevant to this module.
