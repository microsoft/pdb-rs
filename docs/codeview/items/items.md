# CodeView Item Records

CodeView defines a set of _item records_, which contain a variety of information
about programs. Item records are stored in the
[IPI Stream](../../pdb/ipi_stream.md).

To distinguish between type records and item records, we use `ItemId` (an alias
for `uint32_t`) for identifying item records.

Item records use the same record framing as type records. Briefly repeated:

```c
struct TypeRecord {
  uint16_t size;
  uint16_t kind;
  uint8_t payload[size - 2];
};
```

## `ItemId`

Item records are identified by the `ItemId` type alias:

```c
typedef uint32_t ItemId;
```

The value zero is reserved for a nil `ItemId`, meaning nil points to no record
at all. All other `ItemId` values must be within the range of `type_index_begin`
(inclusive lower bound) to `type_index_end` (exclusive upper bound), which is
specified in the IPI Stream Header.

To find a specific record in the IPI given its `ItemId`, first subtract
`type_index_begin` from the `ItemId`. This gives the 0-based index of the record
within the stream; let this be the value `R`. Then, begin decoding records
within the IPI Stream, counting them as they are decoded. When `R` records have
been decoded, the next record is the desired record.

The value of `type_index_begin` (in the IPI Stream Header) is typically 0x1000.
No other value has been observed.

## Item Record Summary

These are the defined item records. Although the names of these records share
the same `LF_*` prefix with "type" records (and the same numeric identifier
space), they are not type records.

Kind   | Name                                               | Description
-------|----------------------------------------------------|------------
0x1601 | [`LF_FUNC_ID`](./lf_func_id.md.md)                 | Identifies a function.
0x1602 | [`LF_MFUNC_ID`](./lf_mfunc_id.md)                  | Identifies a member function. This includes both static and non-static member functions.
0x1603 | [`LF_BUILDINFO`](./lf_buildinfo.md)                | Describes the environment and arguments to an invocation of a tool or compiler.
0x1604 | [`LF_SUBSTR_LIST`](./lf_substr_list.md)            | Contains a list of string IDs, forming a concatenated string.
0x1605 | [`LF_STRING_ID`](./lf_string_id.md)                | A string, identified by an `ItemId`.
0x1607 | [`LF_UDT_MOD_SRC_LINE`](./lf_udt_mod_src_line.md)  | Specifies the source location for the definition of a user-defined type (UDT), in a module.
0x1606 | [`LF_UDT_SRC_LINE`](./lf_udt_src_line.md)          | Specifies the source location for the definition of a user-defined type (UDT).
