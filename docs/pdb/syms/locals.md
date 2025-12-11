# Local variables: `S_LOCAL` (0x113e) and related records

Symbol records describe the names and locations of local variables. The records
that describe local variables can be nested at any level within a procedure
symbol.

## `S_REGREL32` (0x1111) - Register-Relative Local Variable

```c
struct RegRel {
    uint32_t offset;
    TypeIndex type;
    uint16_t register;
    strz name;
};
```

This symbol specifies symbols that are allocated relative to a register. This
should be used on all platforms besides x86 and on x86 when the register is not
a form of `EBP`.

This symbol can only occur within procedure symbol scopes (`S_LPROC32` and
`S_GPROC32`). It may be nested within inlined call sites with procedure symbol
scopes.

**Unlike** the rest of the local variable symbols described in this document,
`S_REGREL32` **is not** paired with an `S_LOCAL` record. This is because
`S_REGREL32` already contains a `name` field.

## Paired symbols

Most local variables are described by a _pair_ of symbol records. The first
record is always `S_LOCAL` and this record specifies the name of the variable,
e.g. `result` or `windowHandle`. The second record describes the location of the
value of the local and there are many different kinds of records for the
different locations for locals.

For example:

```text
S_GPROC32: add_two_numbers
  S_LOCAL: x
  S_REGREL32: rcx                 <-- location of x
  S_LOCAL: y
  S_REGREL32: rdx                 <-- location of y
  ...
  S_END
```

## `S_LOCAL` (0x113e) - Local Variable

The `S_LOCAL` symbol introduces a local variable.

```c
struct Local {
    TypeIndex type;
    uint32_t flags;
    strz name;
};
```

Immediately after the `S_LOCAL` record is a record that describes the location
of the value of the variable. The following record kinds are known to describe
local variables. The list is not exhaustive and

There are known to be records that Microsoft considers to be its proprietary
intellectual property, relating largely to debugging of optimized code. This
document may list the numeric symbol IDs for these records but it will not
describe their structure or their usage.

## Definition-Range Symbols

These symbols specify the location of a local variable within a range of
instruction addresses. These symbols must immediately follow an `S_LOCAL`
symbol.

## `S_DEFRANGE` (0x113F)

Specifies a DIA program for interpreting a variable.

### `S_DEFRANGE_SUBFIELD` (0x1140)

Specifies a DIA program and an offset in a parent variable for interpreting a
variable.

### `S_DEFRANGE_REGISTER` (0x1141) - Definition Range: Register

A live range of an en-registered variable.

```c
struct DefRangeRegister {
    uint16_t register;
    RangeAttr attr;
    uint8_t gaps[];
};

// See cvinfo.h for CV_RANGEATTR
struct RangeAttr {
    uint16_t bits;
};
```

### `S_DEFRANGE_FRAMEPOINTER_REL` (0x1142) - Definition Range: Frame-Pointer Relative

### `S_DEFRANGE_SUBFIELD_REGISTER` (0x1143) - Definition Range: Sub-Field Register

### `S_DEFRANGE_FRAMEPOINTER_REL_FULL_SCOPE` (0x1144) - Definition Range: Frame-Pointer Relative, Full Scope

A frame variable valid in the entire function scope.

```
struct DefRangeFramePointerRelFullScope {
    uint32 offset_to_frame_pointer;
};
```

### `S_DEFRANGE_REGISTER_REL` (0x1145) - Definition Range: Register-Relative
