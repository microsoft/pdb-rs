## `S_ARMSWITCHTABLE` (0x1159) - ARM Switch Table

```c
struct ArmSwitchTable {
    uint32_t offset_base;
    uint16_t sect_base;
    uint16_t switch_type;
    uint32_t offset_branch;
    uint32_t offset_table;
    uint16_t sect_branch;
    uint16_t sect_table;
    uint32_t num_entries;
};
```

This symbol describes a switch table (jump table), which is used to implement
`switch` statements or similar control flow constructs.

MSVC generates this symbol only when targeting ARM64. LLVM generates this symbol
for all target architectures.

## Fields

`offset_base` is the section-relative offset to the base address for switch
offsets. This is the base address of the target of the jump. The value stored
within the jump table entry is added to this base.

`sect_base` is the section index of the base for switch offsets.

`switch_type` specifies the type of each entry in the jump table. See [Switch
Types](#switch-types) below.

`offset_branch` is the section-relative offset to the table branch instruction.

`offset_table` is the section-relative offset to the start of the table.

`sect_branch` is the section index of the table branch instruction.

`sect_table` is the section index of the table.

`num_entries` is the number of switch table entries.

## Switch Types

The `switch_type` field can have the following values:

Value | Name          | Description
------|---------------|------------
`0`   | `INT1`        | Signed 1-byte offset
`1`   | `UINT1`       | Unsigned 1-byte offset
`2`   | `INT2`        | Signed 2-byte offset
`3`   | `UINT2`       | Unsigned 2-byte offset
`4`   | `INT4`        | Signed 4-byte offset
`5`   | `UINT4`       | Unsigned 4-byte offset
`6`   | `POINTER`     | Absolute pointer (no base)
`7`   | `UINT1SHL1`   | Unsigned 1-byte offset, shift left by 1
`8`   | `UINT2SHL1`   | Unsigned 2-byte offset, shift left by 1

## Notes

LLVM often generates tables where the base address and the table address are the
same, but this is not necessarily true for all tables.

This symbol can only appear in module symbol streams, never in the global symbol
stream.
