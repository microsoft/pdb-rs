- [CodeView Symbols](#codeview-symbols)
- [Symbol record framing](#symbol-record-framing)
- [Symbol Streams: Global vs. Module](#symbol-streams-global-vs-module)
- [Symbols Summary](#symbols-summary)
- [Nested Symbol Scopes](#nested-symbol-scopes)
- [TypeIndex: Pointers into the Type Database (TPI)](#typeindex-pointers-into-the-type-database-tpi)
- [`S_COMPILE2` (0x1116) and `S_COMPILE3` (0x113c) - Compile](#s_compile2-0x1116-and-s_compile3-0x113c---compile)
- [`S_OBJNAME` (0x1101) - Object Name](#s_objname-0x1101---object-name)
- [`S_UNAMESPACE` (0x1124) - Using Namespace](#s_unamespace-0x1124---using-namespace)
- [`S_ANNOTATION` (0x1019) - Annotation](#s_annotation-0x1019---annotation)
- [`S_END` (0x0006) - End of Scope](#s_end-0x0006---end-of-scope)
- [`S_BUILDINFO` (0x114c) - Build Info](#s_buildinfo-0x114c---build-info)
- [Defining data and data types](#defining-data-and-data-types)
  - [`S_UDT` (0x1108) - User-Defined Type](#s_udt-0x1108---user-defined-type)
  - [`S_LTHREAD32` (0x1112) and `S_GTHREAD32` (0x1113) - Thread Storage](#s_lthread32-0x1112-and-s_gthread32-0x1113---thread-storage)
  - [`S_CONSTANT` (0x1107) - Constant](#s_constant-0x1107---constant)
  - [`S_MANCONSTANT` (0x112d) - Managed Constant](#s_manconstant-0x112d---managed-constant)
  - [`S_LDATA32` (0x1007) and `S_GDATA32` (0x1008)](#s_ldata32-0x1007-and-s_gdata32-0x1008)
  - [`S_VFTABLE32` (0x100c) - Virtual Function Table Path](#s_vftable32-0x100c---virtual-function-table-path)
- [Procedure definitions](#procedure-definitions)
  - [`S_LPROC32` (0x110f) and `S_GPROC32` (0x1110) - Procedure Start](#s_lproc32-0x110f-and-s_gproc32-0x1110---procedure-start)
  - [`S_LMANPROC` (0x112b) and `S_GMANPROC` (0x112a) - Managed Procedure Start](#s_lmanproc-0x112b-and-s_gmanproc-0x112a---managed-procedure-start)
  - [`S_THUNK32` (0x1102) - Thunk Start](#s_thunk32-0x1102---thunk-start)
  - [`S_FRAMEPROC` (0x1012) - Frame Procedure Information](#s_frameproc-0x1012---frame-procedure-information)
  - [`S_TRAMPOLINE` (0x112c) - Trampoline](#s_trampoline-0x112c---trampoline)
  - [`S_REGREL32` (0x1111) - Register-Relative Local Variable](#s_regrel32-0x1111---register-relative-local-variable)
  - [`S_LOCAL` (0x113e) - Local Variable](#s_local-0x113e---local-variable)
  - [Definition-Range Symbols](#definition-range-symbols)
    - [`S_DEFRANGE_REGISTER` (0x1141) - Definition Range: Register](#s_defrange_register-0x1141---definition-range-register)
    - [`S_DEFRANGE_FRAMEPOINTER_REL` (0x1142) - Definition Range: Frame-Pointer Relative](#s_defrange_framepointer_rel-0x1142---definition-range-frame-pointer-relative)
    - [`S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE` (0x1144) - Definition Range: Frame-Pointer Relative, Full Scope](#s_defrange_framepointer_rel_full_scope-0x1144---definition-range-frame-pointer-relative-full-scope)
  - [`S_BLOCK32` (0x1103) - Block Start](#s_block32-0x1103---block-start)
  - [`S_LABEL32` (0x1105) - Code Label](#s_label32-0x1105---code-label)
- [Global symbols](#global-symbols)
  - [`S_PUB32` (0x110e) - Public Symbol](#s_pub32-0x110e---public-symbol)
  - [`RefSym2` structure](#refsym2-structure)
  - [`S_PROCREF` (0x1125) - Procedure Reference](#s_procref-0x1125---procedure-reference)
  - [`S_DATAREF` (0x1126) - Data Reference](#s_dataref-0x1126---data-reference)
  - [`S_LPROCREF` (0x1127) - Local Procedure Reference](#s_lprocref-0x1127---local-procedure-reference)
  - [`S_TOKENREF` (0x1129) - MSIL Token Reference](#s_tokenref-0x1129---msil-token-reference)
  - [`S_ANNOTATIONREF` (0x1128) - Annotation Reference](#s_annotationref-0x1128---annotation-reference)

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

The `size` field specifies the size in bytes of this record, excluding the `size` field itself, but including the size of the `kind` and payload fields. Because the `size` field includes the size of the `kind` field, the smallest legal value for `size` is 2.

The starting offset of each symbol record is required to be aligned to a multiple of 4. The size of each symbol record, including the size, kind, and payload fields, is also required to be a multiple of 4. Because the size field does not count the size of the size field itself, this means that the size field is always 2 less than a multiple of 4.

The size of each symbol record is required to be a multiple of 4, which implies that the payload size is also a multiple of 4. However, many symbol records contain variable-length data within their payload, and the size of this payload is not necessarily a multiple of 4. In that case, padding bytes are inserted into the payload. By convention, the framing bytes use the values 0xF1, 0xF2, and 0xF3 (in hex). That is, if a single byte needs to be padded, the value is 0xF1. If two bytes are needed, then the values 0xF1 and 0xF2 are inserted, etc.

The `kind` field specifies how to interpret a symbol. It is a named constant, such as `S_CONSTANT`, `S_COFFGROUP`, etc. The symbol kinds are summarized and described in detail in this section.

# Symbol Streams: Global vs. Module

Within a PDB there are two kinds of symbol streams.  Each PDB contains one Global Symbol Stream (GSS). Some PDBs do not contain a GSS; it is not possible for a PDB to contain more than one GSS.  Each module within a PDB may contain its own "module symbol stream".

The Global Symbol Stream and the module symbol streams use the same symbol stream format, but there are many differences in how symbols are stored and interpreted. Many symbol kinds can appear only in the GSS, or appear only in a module symbol stream.  Nested symbols may appear only in module streams.  For example, symbols that describe the local variables of a function may occur only in module symbol streams, never in global symbol streams.
 
# Symbols Summary

This table summarizes the known symbol record kinds. It also lists whether each kind of symbol may appear in the Global Symbol Stream and/or the Module Symbol Stream, and whether the symbol can be used as a "root" scope or can only be used within a nested symbol scope. These terms will be explained in later sections.

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

Some symbol records are related to other symbol records, by arranging them within nested scopes. These typically indicate a parent/child relationship of some kind.

Nested symbols can only occur within module symbol streams. They cannot appear in the global symbol stream.
The following symbols start a new nested scope when they appear within a symbol stream:

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

Nesting relationships imply a tree. The "top-level" column indicates whether a specific symbol can appear as the root of a tree of nesting relationships. The "nested" column indicates whether a specific symbol can be nested within another symbol record.

The `S_END` symbol ends a symbol scope. The `S_END` symbol itself does not contain any fields (its payload is zero-length).

When processing a symbol stream, when a decoder encounters a symbol that starts a nesting scope, it should push a record onto a scope stack (or increment a stack depth counter). When the processor encounters an S_END record, it should pop a record from the scope stack (or decrement a counter). This allows the processor to see the full symbol scope during traversal. The full symbol scope is often relevant when interpreting a symbol.

# TypeIndex: Pointers into the Type Database (TPI)

Many symbol records contain `TypeIndex` values. `TypeIndex` is an alias for `uint32_t`. `TypeIndex` values point into the Type Database (TPI Stream), or identify primitive (intrinsic) types.

# `S_COMPILE2` (0x1116) and `S_COMPILE3` (0x113c) - Compile

This symbol provides a rich set of information regarding the compiler that was used to produce the object file that it is embedded in.  It is expected that there is exactly one of these records in each object file and that it resides in the first, non-optional, `.debug$S` section.

The `S_COMPILE2` symbol is the older variant. Encoders should use the `S_COMPILE3` symbol. Decoders should be prepared to process both `S_COMPILE2` and `S_COMPILE3` symbols. These symbols may only appear in module symbol streams, not the global symbol stream. These symbols are mutually-exclusive within a given module symbol stream; only one of them may be used in a given module stream. However, a linked executable may contain modules from different tools, and so it is legal for some modules to use `S_COMPILE2` and others to use `S_COMPILE3`.

```
struct Compile2 {
    uint32_t flags;
    uint16_t machine;
    uint16_t frontend_major_version;
    uint16_t frontend_minor_version;
    uint16_t frontend_build_version;
    uint16_t major_version;
    uint16_t minor_version;
    uint16_t build_version;
    strz name;
}

struct Compile3 {
    uint32_t flags;
    uint16_t machine;
    uint16_t frontend_major_version;
    uint16_t frontend_minor_version;
    uint16_t frontend_build_version;
    uint16_t frontend_qfe_version;
    uint16_t major_version;
    uint16_t minor_version;
    uint16_t build_version;
    uint16_t qfe_version;
    strz name;
}
```

`flags` contains language and per-module attributes, encoded in these bit fields:

Name            | Bits   | Description
----------------|--------|------------
language        | 0-7    | language index; see below
EC              | 8      | compiled for edit and continue
NoDbgInfo       | 9      | not compiled with debug information
LTCG            | 10     | compiled with link-time code generation
NoDataAlign     | 11     | no data alignment on globals
ManagedPresent  | 12     | managed code/data present
SecurityChecks  | 13     | compiled with security checks (/GS)
HotPatch        | 14     | compiled with hotpatch support  (/hotpatch)
CVTCIL          | 15     | converted with CVTCIL
MSILModule      | 16     | MSIL netmodule
pad             | 16-31  | reserved, must be zero

Language enumeration:

Value  | Language
-------|---------
0      | C
1      | C++
2      | Fortran
3      | MASM
4      | Pascal
5      | Basic
6      | Cobol
7      | Linker
8      | CvtRes
9      | CvtPgd
10     | C#
11     | Visual Basic .NET
12     | CIL (e.g. `ilasm`)
13     | Java
14     | JScript
15     | MSIL (LTCG of `.NETMODULE`)
16     | HLSL
17     | Objective-C
18     | Objective-C++
19     | Swift
20     | ALIASOBJ
21     | Rust
22     | Go
23-255 | Reserved

# `S_OBJNAME` (0x1101) - Object Name

```
struct ObjectName {
    uint32_t signature;
    strz name;
}
```

`signature` is a robust signature that will change every time that the module will be compiled or different in any way.  It should be at least a CRC32 based upon module name and contents.

`name` is the full path of the object file.

# `S_UNAMESPACE` (0x1124) - Using Namespace

```
struct UsingNamespace {
    strz namespace;
}
```

This symbol is used to indicate that the compiler has added a namespace to the lookup-scope of the lexical scope that contains this symbol.  The use of this symbol is restricted to procedures and blocks, as we felt it unreasonable to burden the consumer side of the debugging information with having to search all of the module's symbols to find them.  Consequently, all functions that fall under a module level `using namespace` directive will each have a `S_UNAMESPACE` symbol record.

# `S_ANNOTATION` (0x1019) - Annotation

```
struct Annotation {
    uint32_t offset;
    uint16_t section;
    uint16_t count;
    strz strings[count];
}
```

This symbol stores annotations that point to a specific location in code streams. This allows for analysis tools, such as debuggers, instrumentation systems (ETW), etc. to process annotations.

`strings` contains a sequence of strings, whose count is given by `count`. PDB does not specify how to interpret these strings.

The MSVC compiler provides an extension which allows it to insert `S_ANNOTATION` records into module streams. This extension is invoked using the `__annotation("format", ... args ...)` syntax.

For example, when MSVC compiles this program:

```c
int main(int argc, char** argv) {
    __annotation(L"Hello!", L"World!");
    return 0;
}
```

it produces this `S_ANNOTATION` symbol:

```
00000784 :   S_ANNOTATION: [0001:00006202]
    Hello!
    World!

00000784 :  1a 00 19 10 02 62 00 00 01 00 02 00 48 65 6c 6c : .....b......Hell
00000794 :  6f 21 00 57 6f 72 6c 64 21 00 00 00             : o!.World!...
```

Note the `segment:offset` value of `[0001:00006202]`. This shows that the `S_ANNOTATION` points to a specific location in the code stream, even though the annotation has no effect on code generation.

# `S_END` (0x0006) - End of Scope

The `S_END` record terminates a nested scope. Nested scopes are created by `S_LPROC32`, `S_GPROC32`, `S_THUNK32`, `S_INLINESITE`, etc. For a complete list of symbols that start a nested scope, see Nested Scopes.

The `S_END` symbol has no payload.

This symbol can appear only within module symbol streams.

# `S_BUILDINFO` (0x114c) - Build Info

```
struct BuildInfoSym {
    ItemId id;
}
```

This record associates the current module with an [`LF_BUILDINFO`](type_records.md#lf_buildinfo-0x1603) record in the IPI Stream. The `BuildInfoSym` record does not directly contain the build information; use `id` to look up the corresponding record in the IPI Stream.

# Defining data and data types

## `S_UDT` (0x1108) - User-Defined Type

```
struct Udt {
  TypeIndex type;
  strz name;
};
```

A user-defined type (UDT). This symbol is usually found in the global symbol stream, but in rare cases can also be found in a module symbol stream.

`type` points into the TPI Stream. The pointed-to type should be `LF_ENUM`, `LF_STRUCTURE`, `LF_CLASS`, or `LF_INTERFACE`.

> TODO: Can `S_UDT` point to primitives?  What about typedefs?

The `name` field of the `S_UDT` symbol record should be equal to the name field of the pointed-to type.
The `S_UDT` symbol can appear in the global symbol stream and in module symbol streams.

## `S_LTHREAD32` (0x1112) and `S_GTHREAD32` (0x1113) - Thread Storage

```
struct ThreadStorage {
    TypeIndex type;
    uint32_t offset;
    uint16_t segment;
    strz name;
}
```

These symbols are used for data declared with the `__declspec(thread)` or `thread_static` storage attribute. The `S_LTHREAD32` symbol is used for variables that are local to a module (no external linkage) and `S_GTHREAD32` is used for variables that have external linkage.

These symbols can appear in both module symbol streams and global symbol streams. They are never nested within another symbol scope.

## `S_CONSTANT` (0x1107) - Constant

```
struct Constant {
  TypeIndex type;
  Number value;
  strz name;
};
```

Defines a named constant. This symbol can appear in both the global symbol stream and in module symbol streams.


## `S_MANCONSTANT` (0x112d) - Managed Constant

```
struct ManagedConstant {
    uint32_t metadata_token;
    Number value;
    strz name;
}
```

Defines a named constant whose type is defined by MSIL metadata. This symbol has been observed only in module streams.

## `S_LDATA32` (0x1007) and `S_GDATA32` (0x1008)

```
struct Data {
    TypeIndex type;
    uint32_t offset;
    uint16_t segment;
    strz name;
}
```

Describes a global variable. `S_LDATA32` is used for global variables that are not visible outside of a specific module, such as variables declared with `static` and those defined within an anonymous namespace in C++. `S_GDATA32` is used for global variables that are visible outside of a specific module.

## `S_VFTABLE32` (0x100c) - Virtual Function Table Path

> TODO: Is this obsolete? It is not found in any Windows PDB.

# Procedure definitions

Procedures (functions and methods) are defined using a variety of symbols. Procedure definitions start with one of the following symbols: `S_LPROC32`, `S_GPROC32`, `S_LMANPROC`, `S_GMANPROC`, `S_THUNK32`.  Then follows a series of symbols that describe the procedure, its local variables, nested call sites, etc. The procedure definition is terminated with `S_END`.

## `S_LPROC32` (0x110f) and `S_GPROC32` (0x1110) - Procedure Start

```
struct Procedure {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t p_next;
    uint32_t proc_length;
    uint32_t debug_start;
    uint32_t debug_end;
    TypeIndex proc_type;
    uint32_t offset;
    uint16_t segment;
    uint8 flags;
    strz name;
}
```

The `S_LPROC32` and `S_GPROC32` symbols define "free" functions (functions at global scope or defined within a namespace); they are not used for static or instance methods of classes. In this section of the specification, `S_LPROC32` will be assumed to apply to both `S_LPROC32` and `S_GPROC32`, unless clarified.

This symbol can only appear in a module symbol stream. It cannot appear in the global symbol stream. However, the global symbol stream can point to a procedure symbol by using `S_PROCREF`.

Each `S_LPROC32` symbol start a _symbol scope_, and the records that follow it are associated with that procedure. For example, `S_REGREL32` defines a register-relative local variable, and it is implicitly associated with the containing `S_LPROC32` scope.  The procedure definition scope is terminated by an `S_END` record.

The `p_parent`, `p_end`, and `p_next` fields are byte offsets within the symbol stream, relative to the `S_LPROC32` record. Because `S_LPROC32` cannot be nested within another symbol scope, `s_parent` is always zero (meaning "no parent"). The `p_end` field is the byte offset (relative to the `S_LPROC32` symbol) of the start of the `S_END` symbol. The `p_next` field is deprecated; decoders should ignore it and encoders should always set it to zero.

`proc_length` is the size in bytes of the procedure (the machine code instructions, not the symbol record). This only applies to procedures whose instructions form a single contiguous block.

`debug_start` is the offset in bytes from the start of the procedure to the point where the stack frame has been set up. Parameter and frame variables can be viewed at this point.

`debug_end` is the offset in bytes from the start of the procedure to the point where the procedure is ready to return and has calculated its return value, if any. Frame and register variables can still be viewed.

`proc_type` is the type of the function signature.

`offset` and `segment` give the address of the procedure.

`flags` has these bits:

Name           | Bit | Description
---------------|-----|------------
`fpo`          | 0   | true if function has frame pointer omitted
`interrupt`    | 1   | true if function is interrupt routine
`unused`       | 2   | must be zero
`never`        | 3   | true if function does not return (eg: exit())
`unused`       | 4   | must be zero
`custcall`     | 5   | true if custom calling convention used
`noinline`     | 6   | true if function marked as noinline
`optdbginfo`   | 7   | true if function has optimized code debugging info

## `S_LMANPROC` (0x112b) and `S_GMANPROC` (0x112a) - Managed Procedure Start

Defines the start of a managed (MSIL) procedure. This symbol definition is similar to `S_GDATA32` but uses MSIL tokens instead of `TypeIndex`.

```
struct Procedure {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t p_next;
    uint32_t proc_length;
    uint32_t debug_start;
    uint32_t debug_end;
    uint32_t proc_type;         // MSIL token
    uint32_t offset;
    uint16_t segment;
    uint8 flags;
    strz name;
}
```

## `S_THUNK32` (0x1102) - Thunk Start

```
struct Thunk {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t p_next;
    uint32_t offset;
    uint16_t segment;
    uint16_t thunk_length;
    uint8_t ordinal;
    strz name;
    // variant data follows
}
```

This record is used to specify any piece of code that exists outside a procedure.  It is followed by an `S_END` record.  The thunk record is intended for small code fragments. and a two byte length field is sufficient for its intended purpose. 

The `p_parent`, `p_end`, `p_next`, `offset`, and `segment` fields have the same meaning as the fields with the same name within `S_LPROC32`.

## `S_FRAMEPROC` (0x1012) - Frame Procedure Information

```
struct FrameProc {
    uint32_t frame_size;
    uint32_t pad_size;
    uint32_t pad_offset;
    uint32_t save_regs_size;
    uint32_t exception_handler_offset;
    uint16_t exception_handler_segment;
    uint32_t flags;
}
```

This symbol is used for indicating a variety of extra information regarding a procedure and its stack frame.  If any of the flags are non-zero, this record should be added to the symbols for that procedure.

`flags` describes various attributes of the function:

Name               | Bits | Description
-------------------|------|------------
`has_alloca`       | 0    | function uses `_alloca()`
`has_set_jmp`      | 1    | function uses `setjmp()`
`has_long_jmp`     | 2    | function uses `longjmp()`
`has_inl_asm`      | 3    | function uses inline asm
`has_eh`           | 4    | function has EH states
`inl_spec`         | 5    | function was specified as inline
`has_seh`          | 6    | function has SEH
`naked`            | 7    | function is `__declspec(naked)`
`security_checks`  | 8    | function has buffer security check
`pad`              | 9-31 | must be zero

## `S_TRAMPOLINE` (0x112c) - Trampoline

```
struct Trampoline {
    uint16_t trampoline_kind;
    uint16_t thunk_size;
    uint32_t thunk_offset;
    uint32_t target_offset;
    uint16_t thunk_section;
    uint16_t target_section;
}
```

This symbol is emitted only by a linker to indicate a fairly simple and short, light-weight thunk to the debugger.  It was introduced due to the more complex code requirements of the RISC platforms whereas on x86, a thunk typically doesn't need any more code that a single instruction where it is simple to decode the destination.  These are typically used when the debugger is expected to step through the thunk to the other side.  Hence, there is a need for the target information in the debug symbols to locate the target in a machine independent manner.

`trampoline_kind` is 0 for linker incremental thunks, and 1 for linker branch-island thunks.

`thunk_size` is the size of the thunk's code.

## `S_REGREL32` (0x1111) - Register-Relative Local Variable

```
struct RegRel {
    uint32_t offset;
    TypeIndex type;
    uint16_t register;
    strz name;
}
```

This symbol specifies symbols that are allocated relative to a register.  This should be used on all platforms besides x86 and on x86 when the register is not a form of `EBP`.

This symbol can only occur within procedure symbol scopes (`S_LPROC32` and `S_GPROC32`). It may be nested within inlined call sites with procedure symbol scopes.

## `S_LOCAL` (0x113e) - Local Variable

```
struct Local {
    TypeIndex type;
    uint32_t flags;
    strz name;
}
```

This symbol defines a local and it must follow by more range descriptions.

## Definition-Range Symbols

These symbols specify the location of a local variable within a range of instruction addresses. These symbols must immediately follow an `S_LOCAL` symbol.

> TODO: Document these. None of them appear to contain `TypeIndex` values, so there is no urgency.

### `S_DEFRANGE_REGISTER` (0x1141) - Definition Range: Register

A live range of an en-registered variable.

```
struct DefRangeRegister {
    uint16_t register;
    RangeAttr attr;
    uint8_t gaps[];
}

// See cvinfo.h for CV_RANGEATTR
struct RangeAttr {
    uint16_t bits;
}
```

### `S_DEFRANGE_FRAMEPOINTER_REL` (0x1142) - Definition Range: Frame-Pointer Relative

### `S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE` (0x1144) - Definition Range: Frame-Pointer Relative, Full Scope

## `S_BLOCK32` (0x1103) - Block Start

```
struct Block {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t length;
    uint32_t offset;
    uint16_t segment;
    strz name;
}
```

This symbol specifies the start of an inner block of lexically scoped symbols. The lexical scope is terminated by a matching `S_END` symbol.

This symbol must be nested within a procedure definition (`S_LPROC32`, etc.). It may be nested within another `S_BLOCK32` or inline call site. This also implies that `S_BLOCK32` can only occur within module symbol streams.

## `S_LABEL32` (0x1105) - Code Label

```
struct Label {
    uint32_t offset;
    uint16_t segment;
    uint8_t flags;
    strz name;
}
```

# Global symbols

Global symbols are stored in the Global Symbol Stream (GSS).  See also [Global Symbols](globals.md).

## `S_PUB32` (0x110e) - Public Symbol

```
struct PubSym {
    uint32_t flags;
    uint32_t offset;
    uint16_t segment;
    strz name;
}
```

`S_PUB32` should only appear in the GSS.

## `RefSym2` structure

Several symbols (e.g. `S_PROCREF`) in the Global Symbol Stream use this definition. These symbols point from the GSS into the symbol stream of a specific module.

```
struct RefSym2 {
    uint32_t name_checksum;
    uint32_t symbol_offset;
    uint16_t module_index;
    strz name;
}
```

`name_checksum` appears to be set to 0 in all records found.

Important!  `module_index` is the 1-based index of the module, e.g. 1 is the first module. This is unlike most other PDB data structures, where module indexes are numbered starting at 0.

`symbol_offset` is the offset in bytes in the module symbol stream.

* `name_checksum` is a checksum computed over the `name` field.

  > TODO: Which hash algorithm computes this checksum?

* `symbol_offset` is the byte offset within the symbol stream of a module, identified by `module_index`. This byte offset starts from the beginning of the symbol stream, and the count includes the 4-byte header at the start of the module stream.  That is, if this record points to the first symbol record in the module's symbol stream, then the value of `symbol_offset` will be 4, not 0.

* `module_index` is the 1-based index of the module.

> Invariant: `module_index` is in the range 1 to `num_modules` (inclusive), where `num_modules` is the number of modules as determined by counting `ModuleInfo` records in the DBI Modules Substream.

> Invariant: `symbol_offset` is the byte offset of a valid symbol record stored in the symbol stream of the module identified by `module_index`. It points to the beginning of a symbol record, not the interior of a record.

## `S_PROCREF` (0x1125) - Procedure Reference

Describes a reference to an `S_GPROC32` record, which is stored in a module symbol stream. It uses the `RefSym2` definition.

`S_PROCREF` should only appear in the GSS.

## `S_DATAREF` (0x1126) - Data Reference

> Important: This record is _not present_ in linker PDBs. The record may be obsolete.

Describes a reference to an `S_GDATA32` or `S_LDATA32` record, which is stored in a module symbol stream. It uses the `RefSym2` definition.

## `S_LPROCREF` (0x1127) - Local Procedure Reference

Describes a reference to an `S_LPROC32` record, which is stored in a module symbol stream. It uses the `RefSym2` definition.

`S_LPROCREF` should only appear in the GSS.

## `S_TOKENREF` (0x1129) - MSIL Token Reference

Describes a reference to a symbol related to an MSIL metadata token. It uses the `RefSym2` definition.

> TODO: Clarify how MSIL integration works, and what set of symbol records `S_TOKENREF` can point to.

`S_TOKENREF` should only appear in the GSS.

## `S_ANNOTATIONREF` (0x1128) - Annotation Reference

Describes a reference to an `S_ANNOTATION` symbol, which is stored in a module symbol stream. It uses the `RefSym2` definition.

`S_ANNOTATIONREF` should only appear in the GSS.

The `name` field in `S_ANNOTATIONREF` is not used and should be empty.
