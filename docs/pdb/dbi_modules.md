# DBI Modules Substream

The DBI Modules Substream lists the modules of the executable. A module is a
compiland / translation unit, such as an *.OBJ file or a *.RES file (produced by
the Resource Compiler). The DBI Modules Substream is very important; many PDB
data structures refer to it.

The DBI Modules Substream is a sequence of contiguous bytes within the
[DBI Stream](dbi.md). The DBI Modules Substream begins immediately after the DBI
Stream Header, which has a fixed size of 64 bytes, so the DBI Modules Substream
always begins at stream offset 64. The size of the DBI Modules Substream is
specified by the `DbiStreamHeader.module_info_size` field.

> Invariant: `DbiStreamHeader.module_info_size` is non-negative and is a
> multiple of 4.

The DBI Modules Substream contains a sequence of DBI Module Info records. These
records have a fixed-size header and a variable-length tail. The size of DBI
Module Info record is a multiple of 4, and records always start at a byte offset
(within the DBI Modules Substream) that is a multiple of 4.

```
// variable-length
struct ModuleInfo {
    uint32_t old_module_index;
    SectionContrib section_contrib;
    uint16_t flags;
    uint16_t stream;
    uint32_t sym_byte_size;
    uint32_t c11_byte_size;
    uint32_t c13_byte_size;
    uint16_t source_file_count;
    uint16_t padding;
    uint32_t unused2;
    uint32_t source_file_name_index;
    uint32_t pdb_file_path_name_index;
    // end of fixed-length header (size = 64)

    // variable-length fields:
    strz module_name;
    strz obj_file;
    uint8_t alignment_padding[];        // 0-3 bytes of alignment padding
}
```

> Invariant: Within the DBI Module Substream, each `ModuleInfo` record starts at
> a byte offset that is a multiple of 4.

> Invariant: The size of each `ModuleInfo` record, including the variable-length
> fields and the `alignment_padding`, is a multiple of 4.

## Module Index

The order of the `ModuleInfo` records determines the _module index_. Module
indices are zero-based; the first `ModuleInfo` record is module index 0. Many
PDB data structures contain a module index field, which points to a specific
module within the DBI Modules Substream. In some cases, PDB data structures
contain arrays of records which have the same number and order as the modules in
the DBI Modules Substream. These relationships must be preserved when making
changes to a PDB, such as changing the order of `ModuleInfo` records.

The `old_module_index` field is deprecated. Some encoders (such as the MSVC
linker) set this value to module index, but some encoders set it to
unpredictable values or to zero. For this reason, the `old_module_index`
field should be considered obsolete; encoders should always set it to the
module index (implied by the order of a given `ModuleInfo` structure in the
array of all modules); decoders should ignore the `old_module_index` field.

> Determinism: Set `old_module_index` to the implied value of the module index.

## First section contribution

The `section_contrib` field describes the contributions of this module to the
executable. See [`SectionContrib`](dbi_sections.md).

> Invariant: `section_contrib` contains a `module_index` field, which must be
> equal to the module index, or have the value 0xffff.

> Determinism: `section_contrib.module_index` must be updated if the DBI Modules
> Substream is sorted.

## Flags

`flags` specifies several bit fields:

Name         | Bits  | Description
-------------|-------|------------
`written`    | 0     | True if this module has been written since DBI opened
`ec_enabled` | 1     | True if this module has Edit-and-Continue symbolic information
(unused)     | 2-7   |
`tsm_index`  | 8-15  | Index into TSM list for this module's server

> Determinism: Set the `written` bit to 0.

`source_file_count` specifies the number of source files used when compiling
this module.

> Invariant: The value of `source_file_count` is equal to the value of
> `module_file_counts[m]` for the corresponding entry in the
> [DBI Sources Substream](dbi_sources.md).

## Edit-and-Continue Fields

The `source_file_name_index` and `pdb_file_path_name_index` values are
used by the MSVC Edit-and-Continue feature. This document does not specify
the meaning or usage of these fields.

## Module Stream

The `stream` field is the stream index of this module's Module Stream, if the
module has one. If a Module does not have a Module Stream then this field will
be set to 0xFFFF. See [Module Stream](module_stream.md) for details on the
contents of this stream. The `sym_byte_size`, `c11_byte_size`, and
`c13_byte_size` fields describe the size of substreams within the Module Stream;
these values are necessary for correctly interpreting the module stream.

> Invariant: No two `ModuleInfo` records should have the same value for `stream`
> (unless the value is 0xffff).

> Invariant: `sym_byte_size` is a multiple of 4.

> Invariant: `c11_byte_size` is a multiple of 4.

> Invariant: `c13_byte_size` is a multiple of 4.

> Invariant: `c11_byte_size` and `c13_byte_size` are not both non-zero.

> Invariant: If `stream` is 0xffff, then `sym_byte_size` is equal to zero,
> `c11_byte_size` is equal to zero, and `c13_byte_size` is equal to zero.

> Invariant: The sum of `sym_byte_size`, `c11_byte_size`, and `c13_byte_size` is
> less than or equal to the size of the module stream.

The `source_file_count` field specifies the number of source files that were
read by the compiler when compiling this module. The DBI Sources Substream also
specifies the number of source files for each module; these values are required
to be equal in any well-formed PDB.

The meaning of the `unused2` field is unclear. We see some PDBs with non-zero
values, but they appear to be meaningless, and may be simply uninitialized data
being written to disk. Encoders should set this field to zero.

> Determinism: Set `unused2` to zero.

## Module Name and Object File Path

The `module_name` and `obj_file` fields are both NUL-terminated UTF-8 strings.
They immediately follow the fixed-length portion of the Module Info record.

The `module_name` and `obj_file` fields, taken together, uniquely identify this
module in a way that is meaningful outside of the PDB (i.e. not simply an index
in an array). Because an executable can be composed of different kinds of
modules, these fields can be used in different ways.

In general, the `obj_file` field specifies the path of the file that was
submitted to the linker. This could be an object file (`*.obj`), a static
library (`*.lib`), a DLL import lib (`*.lib`), a compiled Windows resource file
(`*.res`), etc. The `module_name` field specifies a string that uniquely
identifies that module within the scope of the file it was read from (the
`obj_file`). For example, if a module was linked in from a static library, then
`module_name` is the original object file, such as `printf.obj`, at the time
that the static library was created (not when the linker was run for the current
executable).

## Examples

`module_name`         | `obj_file`     | Description
----------------------|----------------|------------
`printf.obj`          | `msvcrtd.lib`  | Module pulled from static library
`my_app.obj`          | `my_app.obj`   | OBJ file submitted directly to linker
`version.res`         | `version.res`  | Compiled resource file (RC) submitted directly to linker
`Import:KERNEL32.dll` | `kernel32.lib` | DLL import entries pulled from a DLL import library

## Alignment

The `ModuleInfo` record is variable-length, due to the string fields that follow
the fixed-length header. The `ModuleInfo` record must be padded to an alignment
of 4. This must be done not only so that fields fall on their natural alignment,
but also so that the Module Info record decoder can correctly locate the start
of the next record.

The `alignment_padding` pseudo-field exists to pad the size of the entire
`ModuleInfo` to a size that is a multiple of 4. Decoders should compute the
number of alignment padding bytes based on the size of the variable-length
string fields. Similarly, encoders *must* insert padding bytes so that the
complete `ModuleInfo` record has a size that is a multiple of 4. This is
required even for the last `ModuleInfo` record in the DBI Modules Substream
because the DBI Modules Substream is required to have a size that is a multiple
of 4.

> Determinism: If padding bytes are present, use a fixed value, such as zero.

> Invariant: The stream offset of the DBI Modules Substream is a multiple of 4.
> This is trivially met because the DBI Modules Substream is located after the
> DBI Stream Header, whose size is a multiple of 4.

## Determinism

> Determinism: `ModuleInfo` records should be sorted by the tuple of
> (`module_name`, `obj_file`). These are the only identifiers that are
> meaningful outside of the contents of the PDB file itself. The tuple should
> always be unique.
>
> After sorting the list of modules, many PDB data structures will need to be
> updated. The `module_index` field within the `section_contrib` field will need
> to be updated.

> Determinism: Set the alignment padding bytes (if any) to zero.

# References

* [`struct MODI_60_Persist`](https://github.com/microsoft/microsoft-pdb/blob/master/PDB/dbi/dbi.h)
