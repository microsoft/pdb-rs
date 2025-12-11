# `S_COMPILE2` (0x1116) and `S_COMPILE3` (0x113c) - Compile

This symbol provides a rich set of information regarding the compiler that was
used to produce the object file that it is embedded in. It is expected that
there is exactly one of these records in each object file and that it resides in
the first, non-optional, `.debug$S` section.

In a PDB, the module stream for a given module should contain exactly one
`S_COMPILE` symbol (either `S_COMPILE2` or `S_COMPILE3`).

The `S_COMPILE2` symbol is the older variant. Encoders should use the
`S_COMPILE3` symbol. Decoders should be prepared to process both `S_COMPILE2`
and `S_COMPILE3` symbols. These symbols may only appear in module symbol
streams, not the global symbol stream. These symbols are mutually-exclusive
within a given module symbol stream; only one of them may be used in a given
module stream. However, a linked executable may contain modules from different
tools, and so it is legal for some modules to use `S_COMPILE2` and others to use
`S_COMPILE3`.

```c
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
};

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
};
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
