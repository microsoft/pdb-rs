# Procedures: `S_LPROC32` (0x110F), `S_GPROC32` (0x1110), and more

Procedures (executable code such as functions and methods) are defined using a
variety of symbols records. Procedures are described by a _sequence_ of symbol
records. These records are organized into a nested hierarchy (a tree) of
_scopes_.

The scopes within a procedure describe semantic scopes (such as compound
statements) within source code. However, there are no constraints or guarantees
about the exact number of scopes or their nesting relationships, only that the
scopes form a tree.

The procedure symbol (`S_LPROC32`, `S_GPROC32`, etc.) starts the root scope.
The following is the full list of procedure symbols; all of them start a
procedure scope:

* `S_LPROC32`
* `S_GPROC32`
* `S_LPROC32_DPC`
* `S_LPROC32_DPC_ID`
* `S_LPROC32_ST`
* `S_GMANPROC`
* `S_LMANPROC`

The following symbols create a nested scope. These symbols _should not_ be
present in symbol streams unless they are nested (directly or indirectly) within
a procedure scope.

* `S_BLOCK32`: defines a block of code
* `S_INLINESITE`: describes code that was inlined
* `S_INLINESITE2`: describes code that was inlined

All scopes (procedure scopes and nested scopes) are terminated with an `S_END`
record.

Within this document, the term "procedure symbol" or simply "procedure" refers
to the entire tree of symbol records, starting with the procedure record
(e.g. `S_GPROC32`) and ending with the matching `S_END` record. If necessary,
we will use "procedure symbol" or `G_PROC32` to clarify the distinction between
the procedure (the actual executable code), the root symbol record, or the
tree of records associated with the procedure.

Example:

```text
S_GPROC32: paint_house
  S_LOCAL: favorite_color
  S_REGREL32: rax
  S_BLOCK32:                    <-- starts a new scope
    S_LOCAL: x
    S_REGREL32: rcx             <-- location of x
    S_LOCAL: y
    S_REGREL32: rdx             <-- location of y
    S_END                       <-- ends the scope of S_BLOCK32
  S_END                         <-- ends the scope of S_GPROC32
```

This shows the nesting relationship of the scopes.

Procedure symbols are stored in module streams. They are never stored in the
Global Symbol Stream. However, the Global Symbol Stream may contain references
to procedure symbols; see [RefSym](s_refsyms.md).

## Local variables

See [Local variables](s_local.md).

## Procedure Start Records

Each procedure symbol begins with a _procedure start_ record. There are several
symbol record kinds that start a procedure symbol:

* `S_GPROC32`: A global procedure, visible to all modules.
* `S_LPROC32`: A "local" procedure (i.e.g `static`), visible only within a
    single * `S_LPROC32_DPC`: A local procedure with DPC semantics (for Windows
    kernel development)
* `S_LPROC32_DPC_ID`: A local procedure with DPC semantics (for Windows kernel
    development)
* `S_LPROC32_ST`: A local procedure; older (obsolete) record format.
* `S_GMANPROC`: A global procedure defined in an MSIL manifest.
* `S_LMANPROC`: A local procedure defined in an MSIL manifest.

## `S_LPROC32` (0x110f) and `S_GPROC32` (0x1110) - Procedure Start

The `S_LPROC32` and `S_GPROC32` records have the same field layout and (aside
from visibility) have the same semantics.

```c
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
};
```

The `S_LPROC32` and `S_GPROC32` symbols define "free" functions (functions at
global scope or defined within a namespace); they are not used for static or
instance methods of classes. In this section of the specification, `S_LPROC32`
will be assumed to apply to both `S_LPROC32` and `S_GPROC32`, unless clarified.

Procedure symbols can only appear in a module symbol stream. It cannot appear in
the global symbol stream. However, the global symbol stream can point to a
procedure symbol by using `S_PROCREF`.

Each `S_LPROC32` symbol starts a _symbol scope_, and the records that follow it
are associated with that procedure. For example, `S_REGREL32` defines a
register-relative local variable, and it is implicitly associated with the
containing `S_LPROC32` scope. The procedure definition scope is terminated by an
`S_END` record.

The `p_parent`, `p_end`, and `p_next` fields are byte offsets within the symbol
stream, relative to the `S_LPROC32` record. Because `S_LPROC32` cannot be nested
within another symbol scope, `s_parent` is always zero (meaning "no parent").
The `p_end` field is the byte offset (relative to the `S_LPROC32` symbol) of the
start of the `S_END` symbol. The `p_next` field is deprecated; decoders should
ignore it and encoders should always set it to zero.

`proc_length` is the size in bytes of the procedure (the machine code
instructions, not the symbol record). This only applies to procedures whose
instructions form a single contiguous block.

`debug_start` is the offset in bytes from the start of the procedure to the
point where the stack frame has been set up. Parameter and frame variables can
be viewed at this point.

`debug_end` is the offset in bytes from the start of the procedure to the point
where the procedure is ready to return and has calculated its return value, if
any. Frame and register variables can still be viewed.

`proc_type` is the type of the function signature.

`offset` and `segment` give the address of the procedure.

`flags` has these bits:

| Name         | Bit | Description                                              |
|--------------|-----|----------------------------------------------------------|
| `fpo`        | 0   | true if function has frame pointer omitted               |
| `interrupt`  | 1   | true if function is interrupt routine                    |
| `unused`     | 2   | must be zero                                             |
| `never`      | 3   | true if function does not return (eg: exit())            |
| `unused`     | 4   | must be zero                                             |
| `custcall`   | 5   | true if custom calling convention used                   |
| `noinline`   | 6   | true if function marked as noinline                      |
| `optdbginfo` | 7   | true if function has optimized code debugging info       |

## `S_LMANPROC` (0x112b) and `S_GMANPROC` (0x112a) - Managed Procedure Start

Defines the start of a managed (MSIL) procedure. This symbol definition is
similar to `S_GDATA32` but uses MSIL tokens instead of `TypeIndex`.

```c
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
};
```

## Nested symbols

Procedures may contain the following nested symbols:

| Hex    | Symbol          | Link                                     | Description
|--------|-----------------|------------------------------------------|---------------------------
| 0x1103 | `S_BLOCK32`     | [s_block.md](s_block.md)                 | Block scope
| 0x114d | `S_INLINESITE`  | [s_inlinesite.md](s_inlinesite.md)       | Inlined code
| 0x115c | `S_INLINESITE2` | [s_inlinesite.md](s_inlinesite.md)       | Inlined code (v2)
| 0x113e | `S_LOCAL`       | [s_local.md](s_local.md)                 | Local variable
| 0x1111 | `S_REGREL32`    | [s_local.md](s_local.md)                 | Register-relative variable
| 0x0006 | `S_END`         | [s_end.md](s_end.md)                     | Scope terminator
