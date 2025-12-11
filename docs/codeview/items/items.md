# CodeView Item Records

CodeView defines a set of _item records_, which contain a variety of information
about programs. Item records are stored in the [IPI Stream](../../pdb/ipi.md).

To distinguish between type records and item records, we use `ItemId` (an alias
for `uint32_t`) for identifying item records.

Item records use the same record framing as type records. Briefly repeated:

```c
struct ItemRecordHeader {
  uint16_t size;
  uint16_t kind;
  // followed by kind - 2 bytes of kind-specific data
};
```

Kind   | Name              | Category     | Description
-------|-------------------|--------------|------------
0x1601 | `LF_FUNC_ID`      | item         | Identifies a function.
0x1602 | `LF_MFUNC_ID`     | item         | Identifies a member function. This includes both static and non-static member functions.
0x1603 | `LF_BUILDINFO`    | item         | Describes the environment and arguments to an invocation of a tool or compiler.
0x1604 | `LF_SUBSTR_LIST`  | item         | Contains a list of string IDs, forming a concatenated string.
0x1605 | `LF_STRING_ID`    | item         | A string, identified by an `ItemId`.
