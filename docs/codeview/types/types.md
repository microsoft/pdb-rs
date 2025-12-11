- [CodeView Type Records](#codeview-type-records)
- [Summary of record kinds](#summary-of-record-kinds)
- [Leaf indices referenced from symbols](#leaf-indices-referenced-from-symbols)
  - [`LF_ARGLIST` (0x1201) - Argument List](#lf_arglist-0x1201---argument-list)
  - [`LF_FIELDLIST` (0x1203) - Field List](#lf_fieldlist-0x1203---field-list)
  - [`LF_BITFIELD` (0x1205) - Bit-field](#lf_bitfield-0x1205---bit-field)
  - [`LF_METHODLIST` (0x1206) - Method List](#lf_methodlist-0x1206---method-list)
  - [`LF_REFSYM` (0x020c) - Referenced Symbol](#lf_refsym-0x020c---referenced-symbol)
- [ID records](#id-records)
  - [`LF_FUNC_ID` (0x1601)](#lf_func_id-0x1601)
    - [Examples](#examples)
  - [`LF_MFUNC_ID` (0x1602)](#lf_mfunc_id-0x1602)
    - [Example](#example)
  - [`LF_BUILDINFO` (0x1603)](#lf_buildinfo-0x1603)
    - [Example](#example-1)
  - [`LF_SUBSTR_LIST` (0x1604)](#lf_substr_list-0x1604)
    - [Example](#example-2)
  - [`LF_STRING_ID` (0x1605)](#lf_string_id-0x1605)
  - [`LF_UDT_SRC_LINE` (0x1606) - UDT Source Line](#lf_udt_src_line-0x1606---udt-source-line)
  - [`LF_UDT_MOD_SRC_LINE` (0x1607) - UDT Source Line in Module](#lf_udt_mod_src_line-0x1607---udt-source-line-in-module)

# CodeView Type Records

This file describes the type records used by CodeView.

Type records are variable-length records. Each record begins with a 4-byte header which specifies the length and the "kind" of the record.

```c
struct TypeRecordHeader {
  uint16_t size;
  uint16_t kind;
  // followed by kind - 2 bytes of kind-specific data
};
```

The `size` field specifies the size in bytes of the record. The `size` field _does not_ count the `size` field itself, but it _does_ count the `kind` field and the payload bytes.

> Invariant: The `size` field is a multiple of 2 and is greater than or equal to 2.

Type records are aligned at 2-byte boundaries. Unfortunately, many type records contain fields that have 4-byte alignment, such as `uint32_t`. Encoders and decoders must handle misaligned access to those fields, either using unaligned memory accesses or must copy the entire record to a buffer that has a guaranteed alignment.

The kind field specifies how to interpret a type record. In the PDB documentation, this kind field uses the "Leaf Type" enumeration. The details of these records are outside of the scope of this document. See these references:

# Summary of record kinds

There are three disjoint categories of record kinds:

1. `type` category: Records that are pointed-to by other parts of the PDB file, such as symbol records. These are the "top-level" type records. These records define a complete type, which is what allows them to be used by symbol records.
2. `internal` category: Records that are pointed-to by other type records. These records allow complex type definitions to be defined, with hierarchical internal structure. These records form part of types, but are not themselves complete types. For example, `LF_METHODLIST` gives a list of methods, but is not attached to any single type.
3. `field` category: Records that are part of complex field lists, with `LF_FIELDLIST` records. These do not use the `TypeRecordHeader` structure.

Value  | Name              | Category     | Description
-------|-------------------|--------------|------------
0x1001 | `LF_MODIFIER`     | type         | Modifies a type by applying `volatile`, `const`, or `unaligned` to it
0x1002 | `LF_POINTER`      | type         | Defines a pointer to another type, e.g. `FOO*`
0x1502 | `LF_ARRAY`        | type         | A fixed-size array, e.g. `char FOO[100]`
0x1504 | `LF_CLASS`        | type         | A `class` definition
0x1505 | `LF_STRUCTURE`    | type         | A `struct` definition
0x1506 | `LF_UNION`        | type         | A `union` definition
0x1507 | `LF_ENUM`         | type         | An `enum` definition
0x1008 | `LF_PROCEDURE`    | type         | A function signature type
0x1009 | `LF_MFUNCTION`    | type         | A member function signature type
0x000a | `LF_VTSHAPE`      | ??           | Shape of a virtual function table
0x100d | `LF_VFTPATH`      | ??           | Path to the virtual function table
0x1509 | `LF_PRECOMP`      | ??           | Specifies types come from precompiled module
0x0014 | `LF_ENDPRECOMP`   | ??           | Specifies end of precompiled types
0x1200 | `LF_SKIP`         | internal     | Reserves space in the type stream, but contains no information
0x1201 | `LF_ARGLIST`      | internal     | Specifies arguments for `LF_PROCEDURE` or `LF_MFUNCTION`
0x1203 | `LF_FIELDLIST`    | internal     | Contains field records for `LF_CLASS`, `LF_STRUCTURE`, `LF_ENUM`, etc.
0x1204 | `LF_DERIVED`      | internal     | Specifies classes directly derived from a given class. (Obsolete??)
0x1205 | `LF_BITFIELDS`    | internal     | Specifies a bitfield within another field
0x1206 | `LF_METHODLIST`   | internal     | Specifies a list of methods in an overload group (methods that have the same name but differing signatures)

# Leaf indices referenced from symbols

These are the top-level type record kinds. These records can be pointed-to by symbol records, or by other top-level type records. They define types.
