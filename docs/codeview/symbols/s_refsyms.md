# Reference Symbols: `S_PROCREF`, `S_DATAREF`, etc.

Several symbols (e.g. `S_PROCREF`) in the Global Symbol Stream use this
definition. These symbols point from the GSS into the symbol stream of a
specific module.

```c
struct RefSym2 {
    uint32_t name_checksum;
    uint32_t symbol_offset;
    uint16_t module_index;
    strz name;
};
```

`name_checksum` appears to be set to 0 in all records found.

Important! `module_index` is the 1-based index of the module, e.g. 1 is the
first module. This is unlike most other PDB data structures, where module
indexes are numbered starting at 0.

`symbol_offset` is the offset in bytes in the module symbol stream.

* `name_checksum` is a checksum computed over the `name` field.

  > TODO: Which hash algorithm computes this checksum?

* `symbol_offset` is the byte offset within the symbol stream of a module,
  identified by `module_index`. This byte offset starts from the beginning of
  the symbol stream, and the count includes the 4-byte header at the start of
  the module stream. That is, if this record points to the first symbol record
  in the module's symbol stream, then the value of `symbol_offset` will be 4,
  not 0.

* `module_index` is the 1-based index of the module.

> Invariant: `module_index` is in the range 1 to `num_modules` (inclusive),
> where `num_modules` is the number of modules as determined by counting
> `ModuleInfo` records in the DBI Modules Substream.

> Invariant: `symbol_offset` is the byte offset of a valid symbol record stored
> in the symbol stream of the module identified by `module_index`. It points to
> the beginning of a symbol record, not the interior of a record.

## `S_PROCREF` (0x1125) - Procedure Reference

Describes a reference to an `S_GPROC32` record, which is stored in a module
symbol stream. It uses the `RefSym2` definition.

`S_PROCREF` should only appear in the GSS.

## `S_DATAREF` (0x1126) - Data Reference

> Important: This record is _not present_ in linker PDBs. The record may be
> obsolete.

Describes a reference to an `S_GDATA32` or `S_LDATA32` record, which is stored
in a module symbol stream. It uses the `RefSym2` definition.

## `S_LPROCREF` (0x1127) - Local Procedure Reference

Describes a reference to an `S_LPROC32` record, which is stored in a module
symbol stream. It uses the `RefSym2` definition.

`S_LPROCREF` should only appear in the GSS.

## `S_TOKENREF` (0x1129) - MSIL Token Reference

Describes a reference to a symbol related to an MSIL metadata token. It uses the
`RefSym2` definition.

> TODO: Clarify how MSIL integration works, and what set of symbol records
> `S_TOKENREF` can point to.

`S_TOKENREF` should only appear in the GSS.

## `S_ANNOTATIONREF` (0x1128) - Annotation Reference

Describes a reference to an `S_ANNOTATION` symbol, which is stored in a module
symbol stream. It uses the `RefSym2` definition.

`S_ANNOTATIONREF` should only appear in the GSS.

The `name` field in `S_ANNOTATIONREF` is not used and should be empty.
