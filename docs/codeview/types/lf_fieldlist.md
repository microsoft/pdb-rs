# `LF_FIELDLIST` (0x1203) - Field List

The `LF_FIELDLIST` record contains a list of fields defined on a type. The type
can be `LF_CLASS`, `LF_STRUCTURE`, `LF_INTERFACE`, `LF_UNION`, or `LF_ENUM`.

Each field within an `LF_FIELDLIST` record uses a leaf value to identify the
kind of field. However, the leaf values are disjoint from those used for type
records. Also, the field records do not use the same header as that used for
type records. The length of each field is not stored; it is implied by the leaf
value. For this reason, decoders must know how to decode all possible fields,
because if the decoder does not recognize a field then it cannot know the size
of the field.

This table summarizes the different field kinds. They use the same `LF_XXX`
naming convention, but these values are disjoint from the `LF_XXX` values used
for type records and should not be confused with them.

After the summary, each field is described in detail.

Value  | Name               | Description
-------|--------------------|------------
0x040b | `LF_FRIENDCLS`     | A friend class
0x1400 | `LF_BCLASS`        | A non-virtual (real) base class of a class.
0x1401 | `LF_VBCLASS`       | A directly-inherited virtual base class.
0x1402 | `LF_IVBCLASS`      | An indirectly-inherited virtual base class.
0x1404 | `LF_INDEX`         | An index to another `LF_FIELDLIST` which contains more fields
0x1409 | `LF_VFUNCTAB`      | A virtual function table pointer
0x140c | `LF_VFUNCOFF`      | A virtual function table pointer at a non-zero offset
0x1502 | `LF_ENUMERATE`     | An enumerator (named constant) defined on an `LF_ENUM` type
0x150c | `LF_FRIENDFCN`     | A friend function
0x150d | `LF_MEMBER`        | A non-static data member (field)
0x150e | `LF_STMEMBER`      | A static data member (static field)
0x150f | `LF_METHOD`        | A single method of a class, or a pointer to a `LF_METHODLIST`
0x1510 | `LF_NESTEDTYPE`    | A nested type
0x1511 | `LF_ONEMETHOD`     | A single non-overloaded method
0x1512 | `LF_NESTEDTYPEX`   | Nested type extended definition
0x1513 | `LF_MEMBERMODIFY`  | Modifies the protection of a member of method in a subclass
