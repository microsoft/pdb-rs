# CodeView Symbols

Many streams encode sequences of CodeView "symbols". Symbols are variable-length
records which describe certain elements of high-level languages, such as C, C++,
Rust, etc.

CodeView was defined in the 1980s and has been extended and modified in nearly
every release of MSVC. This document does not describe all of the possible
symbol record kinds. Instead, it focuses on symbols used by modern compilers
(including, but not limited to, MSVC and LLVM).

# Symbol record framing

A "symbol stream" is a sequence of variable-length symbol records. Each symbol
record starts with a 4-byte header, which is followed by the payload for that
symbol. The 4-byte header specifies the size of the symbol payload and a 16-bit
"symbol kind," which specifies how to interpret this symbol record.

Each symbol record has this structure:

```c
struct SymbolRecord {
  uint16_t size;              // size of the entire record (including header)
  uint16_t kind;              // S_GPROC32, S_ANNOTATION, etc.
  uint8_t payload[size - 2];
};
```

The `size` field specifies the size in bytes of this record, excluding the
`size` field itself, but including the size of the `kind` and payload fields.
Because the `size` field includes the size of the `kind` field, the smallest
legal value for `size` is 2.

We use the term `kind` to identify the meaning and structure of symbols instead
of `type` to avoid confusion with `TypeIndex` and CodeView type descriptions.

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

Code (Hex) | Name                                         | Location      | Description
-----------|----------------------------------------------|---------------|------------
`0006`     | [`S_END`](s_end.md)                          | module        | Ends a nested scope
`1007`     | [`S_GDATA32`](s_data.md)                     | global        | Global data with external linkage
`1008`     | [`S_LDATA32`](s_data.md)                     | global/module | Global data with private module visibility
`1012`     | [`S_FRAMEPROC`](s_frameproc.md)              | module        | Frame procedure information
`1019`     | [`S_ANNOTATION`](s_annotation.md)            | module        | Annotation
`1101`     | [`S_OBJNAME`](s_objname.md)                  | module        | Gives the name of the object file
`1102`     | [`S_THUNK32`](s_thunk.md)                    | module        | Code outside a procedure (thunk)
`1103`     | [`S_BLOCK32`](s_block.md)                    | module        | Block of code within a procedure
`1105`     | [`S_LABEL32`](s_label.md)                    | module        | Code label within procedures
`1107`     | [`S_CONSTANT`](s_constant.md)                | global/module | Named constant
`1108`     | [`S_UDT`](s_udt.md)                          | global/module | User-defined type (struct, enum, etc.)
`110E`     | [`S_PUB32`](s_pub.md)                        | global        | Public symbol
`110F`     | [`S_LPROC32`](s_procs.md)                    | module        | Procedure private to a module
`1110`     | [`S_GPROC32`](s_procs.md)                    | module        | Procedure with global linkage
`1111`     | [`S_REGREL32`](s_local.md)                   | module        | Register-relative local variable
`1112`     | [`S_LTHREAD32`](s_thread.md)                 | global/module | Thread-local storage (module-local)
`1113`     | [`S_GTHREAD32`](s_thread.md)                 | global/module | Thread-local storage (global)
`1116`     | [`S_COMPILE2`](s_compile.md)                 | module        | Compiler version and flags
`1124`     | [`S_UNAMESPACE`](s_namespace.md)             | module        | Using namespace
`1125`     | [`S_PROCREF`](s_refsyms.md)                  | global        | Procedure reference
`1126`     | [`S_DATAREF`](s_refsyms.md)                  | global        | Data reference
`1127`     | [`S_LPROCREF`](s_refsyms.md)                 | global        | Local procedure reference
`1128`     | [`S_ANNOTATIONREF`](s_refsyms.md)            | global        | Annotation reference
`1129`     | [`S_TOKENREF`](s_refsyms.md)                 | global        | MSIL token reference
`112A`     | [`S_GMANPROC`](s_procs.md)                   | module        | Managed procedure with global linkage
`112B`     | [`S_LMANPROC`](s_procs.md)                   | module        | Managed procedure with module-local linkage
`112C`     | [`S_TRAMPOLINE`](s_trampoline.md)            | module        | Trampoline
`112D`     | [`S_MANCONSTANT`](s_constant.md)             | module        | Managed constant
`1137`     | [`S_COFFGROUP`](s_coffgroup.md)              | module        | COFF group (subsection)
`1136`     | [`S_SECTION`](s_section.md)                  | module        | COFF section in a PE executable
`113C`     | [`S_COMPILE3`](s_compile.md)                 | module        | Compiler version and flags (extended)
`113E`     | [`S_LOCAL`](s_local.md)                      | module        | Local variable
`113F`     | [`S_DEFRANGE`](s_local.md)                   | module        | Define range for local variable
`1140`     | [`S_DEFRANGE_SUBFIELD`](s_local.md)          | module        | Define range with offset in parent variable
`1141`     | [`S_DEFRANGE_REGISTER`](s_local.md)          | module        | Define range for enregistered variable
`1142`     | [`S_DEFRANGE_FRAMEPOINTER_REL`](s_local.md)  | module        | Define range for frame-pointer relative variable
`1143`     | [`S_DEFRANGE_SUBFIELD_REGISTER`](s_local.md) | module        | Define range for sub-field register
`1144`     | [`S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE`](s_local.md) | module | Define range for frame-pointer relative variable (full scope)
`1145`     | [`S_DEFRANGE_REGISTER_REL`](s_local.md)      | module        | Define range for register-relative variable
`114C`     | [`S_BUILDINFO`](s_buildinfo.md)              | module        | Build info
`114D`     | [`S_INLINESITE`](s_inlinesite.md)            | module        | Inline site
`1159`     | [`S_ARMSWITCHTABLE`](s_armswitchtable.md)    | module        | ARM switch table (jump table)
`115C`     | [`S_INLINESITE2`](s_inlinesite.md)           | module        | Inline site (version 2) 

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

See [Types](../types/types.md).

# Global symbols

Global symbols are stored in the [Global Symbol Stream (GSS)](../../pdb/globals.md).

# Module symbols

Module symbols (symbols that describe code and data within a particular
module) are stored in [Module Streams](../../pdb/module_stream.md).
