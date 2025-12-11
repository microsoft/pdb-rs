# CodeView Type Records

This file describes the type records used by CodeView.

Type records are variable-length records. Each record begins with a 4-byte
header which specifies the length and the "kind" of the record.

```c
struct TypeRecord {
   uint16_t size;
   uint16_t kind;
   uint8_t payload[size - 2];
};
```

The `size` field specifies the size in bytes of the record. The `size` field
_does not_ count the `size` field itself, but it _does_ count the `kind` field
and the payload bytes.

> Invariant: The `size` field is a multiple of 2 and is greater than or equal to
> 2.

Type records are aligned at 2-byte boundaries. Unfortunately, many type records
contain fields that have 4-byte alignment, such as `uint32_t`. Encoders and
decoders must handle misaligned access to those fields, either using unaligned
memory accesses or must copy the entire record to a buffer that has a guaranteed
alignment.

The `kind` field specifies how to interpret a type record. In the PDB
documentation, this `kind` field uses the "Leaf Type" enumeration.

# Summary of type records

There are several disjoint categories of record kinds:

* `type` category: Records that are pointed-to by other parts of the PDB file,
  such as symbol records. These are the "top-level" type records. These records
  define a complete type, which is what allows them to be used by symbol
  records.

* `internal` category: Records that are pointed-to by other type records, but
  which are _not_ pointed-to by any symbol record outside of the TPI stream.
  This is why these records are called "internal"; these records (along with the
  records that point to them) describe complex hierarchical structures. Only the
  "external" records, such as `LF_CLASS` or `LF_ENUM`, are directly pointed-to
  by symbol records.

  These internal records are not, by themselves, complete records. They only
  have meaning in the context of the hierarchy of records that they occur
  within. For example, `LF_METHODLIST` gives a list of methods, but must be
  associated with (pointed-to by) a record such as `LF_CLASS`.

* `special` records, such as `LF_PRECOMP`, which are produced by the compiler
  and are read (and consumed) by the linker.

There are also data structures which use `LF_*` naming prefixes and the same
numbering system as type records, but which are embedded within `LF_FIELDLIST`
records. These "field records" **are not** type records. They **do not** use the
`TypeRecord` layout and cannot be found by simply enumerating the type records
in the TPI. See [`LF_FIELDLIST`](./lf_fieldlist.md).

Kind   | Name                                     | Category | Description
-------|------------------------------------------|----------|------------
0x1001 | [`LF_MODIFIER`](lf_modifier.md)          | type     | Modifies a type by applying `volatile`, `const`, or `unaligned` to it
0x1002 | [`LF_POINTER`](lf_pointer.md)            | type     | Defines a pointer to another type, e.g. `FOO*`
0x1502 | [`LF_ARRAY`](lf_array.md)                | type     | A fixed-size array, e.g. `char FOO[100]`
0x1504 | [`LF_CLASS`](lf_class.md)                | type     | A `class` definition
0x1505 | [`LF_STRUCTURE`](lf_class.md)            | type     | A `struct` definition
0x1506 | [`LF_UNION`](lf_union.md)                | type     | A `union` definition
0x1507 | [`LF_ENUM`](lf_enum.md)                  | type     | An `enum` definition
0x1008 | [`LF_PROCEDURE`](lf_procedure.md)        | type     | A function signature type
0x1009 | [`LF_MFUNCTION`](lf_mfunction.md)        | type     | A member function signature type
0x000a | [`LF_VTSHAPE`](lf_vtshape.md)            | ??       | Shape of a virtual function table
0x100d | [`LF_VFTPATH`](lf_vftpath.md)            | ??       | Path to the virtual function table
0x1509 | [`LF_PRECOMP`](lf_precomp.md)            | special  | Specifies types come from precompiled module
0x0014 | [`LF_ENDPRECOMP`](lf_endprecomp.md)      | special  | Specifies end of precompiled types
0x1200 | [`LF_SKIP`](lf_skip.md)                  | internal | Reserves space in the type stream, but contains no information
0x1201 | [`LF_ARGLIST`](lf_arglist.md)            | internal | Specifies arguments for `LF_PROCEDURE` or `LF_MFUNCTION`
0x1203 | [`LF_FIELDLIST`](lf_fieldlist.md)        | internal | Contains field records for `LF_CLASS`, `LF_STRUCTURE`, `LF_ENUM`, etc.
0x1204 | `LF_DERIVED`                             | internal | Specifies classes directly derived from a given class. (obsolete)
0x1205 | [`LF_BITFIELDS`](lf_bitfield.md)         | internal | Specifies a bitfield within another field
0x1206 | [`LF_METHODLIST`](lf_methodlist.md)      | internal | Specifies a list of methods in an overload group (methods that have the same name but differing signatures)

# Summary of item records

# Leaf indices referenced from symbols

These are the top-level type record kinds. These records can be pointed-to by
symbol records, or by other top-level type records. They define types.
