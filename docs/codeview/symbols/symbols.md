- [CodeView Symbols](#codeview-symbols)
- [Symbol record framing](#symbol-record-framing)
- [Symbol Streams: Global vs. Module](#symbol-streams-global-vs-module)
- [Symbols Summary](#symbols-summary)
- [Nested Symbol Scopes](#nested-symbol-scopes)
- [TypeIndex: Pointers into the Type Database (TPI)](#typeindex-pointers-into-the-type-database-tpi)
- [`S_END` (0x0006) - End of Scope](#s_end-0x0006---end-of-scope)
- [Defining data and data types](#defining-data-and-data-types)
  - [`S_VFTABLE32` (0x100c) - Virtual Function Table Path](#s_vftable32-0x100c---virtual-function-table-path)
- [Global symbols](#global-symbols)

# CodeView Symbols

Many streams encode sequences of CodeView "symbols". Symbols are variable-length records which describe certain elements of high-level languages, such as C, C++, Rust, etc.

The full description of symbols is outside of the scope of this document. See `Debug110.docx` in the MSVC repo.

# Symbol record framing

A "symbol stream" is a sequence of variable-length symbol records. Each symbol record starts with a 4-byte header, which is followed by the payload for that symbol. The 4-byte header specifies the size of the symbol payload and a 16-bit "symbol kind," which specifies how to interpret this symbol record.

Each symbol record has this structure:

```
struct SymbolRecord {
  uint16_t size;
  uint16_t kind;
  uint8_t payload[size - 2];
};
```

The `size` field specifies the size in bytes of this record, excluding the
`size` field itself, but including the size of the `kind` and payload fields.
Because the `size` field includes the size of the `kind` field, the smallest
legal value for `size` is 2.

The starting offset of each symbol record is required to be aligned to a
multiple of 4. The size of each symbol record, including the size, kind, and
payload fields, is also required to be a multiple of 4. Because the size field
does not count the size of the size field itself, this means that the size field
is always 2 less than a multiple of 4.

The size of each symbol record is required to be a multiple of 4, which implies
that the payload size is also a multiple of 4. However, many symbol records
contain variable-length data within their payload, and the size of this payload
is not necessarily a multiple of 4. In that case, padding bytes are inserted
into the payload. By convention, the framing bytes use the values 0xF1, 0xF2,
and 0xF3 (in hex). That is, if a single byte needs to be padded, the value is
0xF1. If two bytes are needed, then the values 0xF1 and 0xF2 are inserted, etc.

The `kind` field specifies how to interpret a symbol. It is a named constant,
such as `S_CONSTANT`, `S_COFFGROUP`, etc. The symbol kinds are summarized and
described in detail in this section.

# Symbol Streams: Global vs. Module

Within a PDB there are two kinds of symbol streams. Each PDB contains one Global
Symbol Stream (GSS). Some PDBs do not contain a GSS; it is not possible for a
PDB to contain more than one GSS. Each module within a PDB may contain its own
"module symbol stream".

The Global Symbol Stream and the module symbol streams use the same symbol
stream format, but there are many differences in how symbols are stored and
interpreted. Many symbol kinds can appear only in the GSS, or appear only in a
module symbol stream. Nested symbols may appear only in module streams. For
example, symbols that describe the local variables of a function may occur only
in module symbol streams, never in global symbol streams.
 
# Symbols Summary

This table summarizes the known symbol record kinds. It also lists whether each
kind of symbol may appear in the Global Symbol Stream and/or the Module Symbol
Stream, and whether the symbol can be used as a "root" scope or can only be used
within a nested symbol scope. These terms will be explained in later sections.

Code (Hex) | Name        | Global? | Module (Root)? | Module (Nested)? | Description
-----------|----------------|------|------|--------|--------
`0006`     | `S_END`        | No   | No   |   Yes  | Ends a nested scope
`1101`     | `S_OBJNAME`    | No   | Yes  |   No   | Gives the name of the object file
`1102`     | `S_FRAMEPROC`  | No   | No   |   Yes  | Always nested within `S_GPROC` or `S_LPROC`, describes some attributes of a procedure
`1103`     | `S_BLOCK32`    | No   | No   |   Yes  | Describes a block of code within a procedure
`1105`     | `S_LABEL32`    | No   | No   |   Yes  | Nested within procedures, defines a code label
`1107`     | `S_CONSTANT`   | Yes  | Yes  |   No   | Defines a constant
`1108`     | `S_UDT`        | Yes  | Yes  |   No   | Defines a user-defined type (struct, enum, etc.)
`110C`     | `S_LDATA32`    | No   | Yes  |   No   | Describes global data with private module visibility
`110F`     | `S_LPROC32`    | No   | Yes  |   No   | A procedure that is private to a module
`1110`     | `S_GPROC32`    | No   | Yes  |   No   | A procedure that has global linkage. The procedure is defined by exactly one module.
`1111`     | `S_REGREL32`   | No   | No   |   Yes  | 
`1112`     | `S_LTHREAD32`  | No   | Yes  |   No   | 

# Nested Symbol Scopes

Some symbol records are related to other symbol records, by arranging them
within nested scopes. These typically indicate a parent/child relationship of
some kind.

Nested symbols can only occur within module symbol streams. They cannot appear
in the global symbol stream. The following symbols start a new nested scope when
they appear within a symbol stream:

Symbol         | Top-level? | Nested?
---------------------|------|---------
`S_GPROC32`          | Yes  | No
`S_LPROC32`          | Yes  | No
`S_LPROC32_DPC_ID`   | Yes  | No
`S_GPROC32_ID`       | Yes  | No
`S_BLOCK32`          | No   | Yes
`S_THUNK32`          | Yes  | No
`S_INLINESITE`       | No   | Yes
`S_INLINESITE2`      | No   | Yes
`S_SEPCODE`          | Yes  | No
`S_GMANPROC`         | Yes  | No
`S_LMANPROC`         | Yes  | No

Nesting relationships imply a tree. The "top-level" column indicates whether a
specific symbol can appear as the root of a tree of nesting relationships. The
"nested" column indicates whether a specific symbol can be nested within another
symbol record.

The `S_END` symbol ends a symbol scope. The `S_END` symbol itself does not
contain any fields (its payload is zero-length).

When processing a symbol stream, when a decoder encounters a symbol that starts
a nesting scope, it should push a record onto a scope stack (or increment a
stack depth counter). When the processor encounters an S_END record, it should
pop a record from the scope stack (or decrement a counter). This allows the
processor to see the full symbol scope during traversal. The full symbol scope
is often relevant when interpreting a symbol.

# TypeIndex: Pointers into the Type Database (TPI)

Many symbol records contain `TypeIndex` values. `TypeIndex` is an alias for
`uint32_t`. `TypeIndex` values point into the Type Database (TPI Stream), or
identify primitive (intrinsic) types.

# `S_END` (0x0006) - End of Scope

The `S_END` record terminates a nested scope. Nested scopes are created by
`S_LPROC32`, `S_GPROC32`, `S_THUNK32`, `S_INLINESITE`, etc. For a complete list
of symbols that start a nested scope, see Nested Scopes.

The `S_END` symbol has no payload.

This symbol can appear only within module symbol streams.

# Defining data and data types

## `S_VFTABLE32` (0x100c) - Virtual Function Table Path

> TODO: Is this obsolete? It is not found in any Windows PDB.


# Global symbols

Global symbols are stored in the Global Symbol Stream (GSS). See also
[Global Symbols](../../pdb/globals.md).
