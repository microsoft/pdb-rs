- [CodeView Type Records](#codeview-type-records)
- [Summary of record kinds](#summary-of-record-kinds)
- [Leaf indices referenced from symbols](#leaf-indices-referenced-from-symbols)
  - [`LF_MODIFIER` (0x1001)](#lf_modifier-0x1001)
  - [`LF_POINTER` (0x1002)](#lf_pointer-0x1002)
    - [Variant data for pointer to type](#variant-data-for-pointer-to-type)
    - [Variant data for pointer to data member](#variant-data-for-pointer-to-data-member)
  - [`LF_ARRAY` (0x1503)](#lf_array-0x1503)
  - [`LF_CLASS` (0x1504) and related](#lf_class-0x1504-and-related)
  - [`LF_UNION` (0x1506)](#lf_union-0x1506)
  - [`LF_ENUM` (0x1507)](#lf_enum-0x1507)
  - [`LF_PROCEDURE` (0x1008)](#lf_procedure-0x1008)
  - [`LF_MFUNCTION` (0x1009)](#lf_mfunction-0x1009)
  - [`LF_VTSHAPE` (0x000a)](#lf_vtshape-0x000a)
  - [`LF_VFTPATH` (0x100d)](#lf_vftpath-0x100d)
  - [`LF_PRECOMP` (0x1509)](#lf_precomp-0x1509)
  - [`LF_ENDPRECOMP` (0x0014) - End of Precompiled Types](#lf_endprecomp-0x0014---end-of-precompiled-types)
- [Records referenced from other type records](#records-referenced-from-other-type-records)
  - [`LF_SKIP` (0x1200)](#lf_skip-0x1200)
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

```
struct TypeRecordHeader {
  uint16_t size;
  uint16_t kind;
  // followed by kind - 2 bytes of kind-specific data
}
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

## `LF_MODIFIER` (0x1001)

```
struct Modifier {
    TypeIndex index;
    uint16_t attribute;
}
```

This record modifies another type record by qualifying it with `const`, `volatile`, or `unaligned` modifiers.

`index` identifies the type that this type is based on.

`attribute` is a set of bit fields. If a bit is set to 1, then that qualifier applies:

Field       | Bits      | Description
------------|-----------|------------
`const`     | 0         | `const` qualifier
`volatile`  | 1         | `volatile` qualifier
`unaligned` | 2         | `unaligned` qualifier
(reserved)  | 3-15      | reserved; must be zero

> **Determinism**: `LF_MODIFIER` records should not point to another `LF_MODIFIER` record. Instead, each `LF_MODIFIER` record should point directly to the unqualified type, and should contain the union of all qualifiers that are needed.

## `LF_POINTER` (0x1002)

Defines a type that is a pointer to another type. This is used for C-style pointers, such as `FOO *`, as well as C++ references such as `FOO &`.

```
struct Pointer {
    TypeIndex type;        // The type that is being pointed-to
    uint32_t attributes;   // Flags; see below
    // more data follows; structure depends on attributes
}
```

The `attribute` field contains several bit fields:

`attribute`<br>bit field | Bits  | Description
------------|---------|--
`ptrtype`   | 0-4     | Specifies the mode of the pointer; see below
`ptrmode`   | 5-7     | Specifies mode; see below
`is_flat32` | 8       | True if is a flat 32-bit pointer
`volatile`  | 9       | True if pointer (not the pointed-to) is volatile, e.g. `int * volatile x;`
`const`     | 10      | True if pointer (not the pointed-to) is const, e.g. `int * const x;`
`unaligned` | 11      | True if pointer (not the pointed-to) is unaligned, e.g. `int * unaligned x;`
`restrict`  | 12      | True if pointer is restricted (is not aliased)
`size`      | 13-18   | Size of pointer in bytes
`ismocom`   | 19      | True if it is a MoCOM pointer (`^` or `%`)
`islref`    | 20      | Ttrue if it is this pointer of member function with `&` ref-qualifier
`lsrref`    | 21      | True if it is this pointer of member function with `&&` ref-qualifier
(reserved)  | 22-31   | reserved

The `ptrtype` bit field can take these values:

```
enum CV_ptrtype {
    CV_PTR_NEAR         = 0x00, // 16 bit pointer
    CV_PTR_FAR          = 0x01, // 16:16 far pointer
    CV_PTR_HUGE         = 0x02, // 16:16 huge pointer
    CV_PTR_BASE_SEG     = 0x03, // based on segment
    CV_PTR_BASE_VAL     = 0x04, // based on value of base
    CV_PTR_BASE_SEGVAL  = 0x05, // based on segment value of base
    CV_PTR_BASE_ADDR    = 0x06, // based on address of base
    CV_PTR_BASE_SEGADDR = 0x07, // based on segment address of base
    CV_PTR_BASE_TYPE    = 0x08, // based on type
    CV_PTR_BASE_SELF    = 0x09, // based on self
    CV_PTR_NEAR32       = 0x0a, // 32 bit pointer
    CV_PTR_FAR32        = 0x0b, // 16:32 pointer
    CV_PTR_64           = 0x0c, // 64 bit pointer
    CV_PTR_UNUSEDPTR    = 0x0d, // first unused pointer type
}
```

The `ptrmode` bit field can take these values:

```
enum CV_ptrmode {
    CV_PTR_MODE_PTR      = 0x00, // "normal" pointer, e.g. `FOO *`
    CV_PTR_MODE_REF      = 0x01, // "old" reference, e.g. `FOO &`
    CV_PTR_MODE_LVREF    = 0x01, // l-value reference, e.g. `FOO &`
    CV_PTR_MODE_PMEM     = 0x02, // pointer to data member
    CV_PTR_MODE_PMFUNC   = 0x03, // pointer to member function
    CV_PTR_MODE_RVREF    = 0x04, // r-value reference, e.g. `FOO &&`
}
```

The data after the `Pointer` structure depends on `ptrtype`, and is called the "variant data".

### Variant data for pointer to type

If the pointer is based on a type (`ptrtype == CV_PTR_BASE_TYPE`), then the variant data consists of a single `TypeIndex`.

### Variant data for pointer to data member

If the pointer is a pointer to a data member, then the variant data has this structure:

```
struct PointerToDataMemberVariant {
    TypeIndex class;        // The pointed-to class
    uint16_t format;
}
```

where `format` has one of these values:

`format` | Description
---------|------------
0        | 16:16 data for class with no virtual functions or virtual bases. 
1        | 16:16 data for class with virtual functions.  
2        | 16:16 data for class with virtual bases.  
3        | 16:32 data for classes w/wo virtual functions and no virtual bases
4        | 16:32 data for class with virtual bases.  
5        | 16:16 near method nonvirtual bases with single address point
6        | 16:16 near method nonvirtual bases with multiple address points
7        | 16:16 near method with virtual bases
8        | 16:16 far method nonvirtual bases with single address point
9        | 16:16 far method nonvirtual bases with multiple address points
10       | 16:16 far method with virtual bases
11       | 16:32 method nonvirtual bases with single address point
12       | 16:32 method nonvirtual bases with multiple address points
13       | 16:32 method with virtual bases

The pointer to data member and pointer to method have the following formats in memory.  In the following descriptions of the format and value of the NULL pointer, `*` means any value.

TODO: convert these; they are quite complicated

## `LF_ARRAY` (0x1503)

```
struct Array {
    TypeIndex element_type;
    TypeIndex index_type;
    Number length;
    strz name;
}
```

Specifies an array type. The array type has a fixed size.

`element_type` is the type of the element, e.g. `int` for `int FOO[100]`.

`index_type` is the type of index used. TODO: what is this used for?

`length` is the size of the array.

`name` is the name of the array type. This is often an empty string.

## `LF_CLASS` (0x1504) and related

This record is used for `LF_CLASS`, `LF_STRUCTURE`, and `LF_INTERFACE`.

```
struct Class {
    uint16_t count;
    uint16_t property;
    TypeIndex fields;
    TypeIndex derivation_list;
    TypeIndex vshape;
    Number length;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
}
```

`count` specifies the number of members defined on the class (or structure). This includes base classes (if any), data members (static and non-static), non-static data fields, member functions (static and non-static), friend declarations, etc. The `count` should not be considered authoritative; it can be used as an allocation hint, but it is possible for a class to have more than 65,535 fields.

`property` specifies a set of bit fields:

Name           | Bits  | Description
---------------|-------|------------
`packed`       | 0     | Structure is packed (has no alignment padding)
`ctor`         | 1     | Class has constructors and/or destructors
`overops`      | 2     | Class has overloaded operators
`nested`       | 3     | Class is a nested class (is nested within another class)
`cnested`      | 4     | Class contains other nested classes
`opassign`     | 5     | Class has overloaded assignment
`opcast`       | 6     | Class has casting methods
`fwdref`       | 7     | Class is a forward declaration (is incomplete; has no field list)
`scoped`       | 8     | This is a scoped definition
`hasuniquename`| 9     | true if there is a decorated name following the regular name
`sealed`       | 10    | true if class cannot be used as a base class
`hfa`          | 11-12 | See `CV_HFA`, below.
`intrinsic`    | 13    | true if class is an intrinsic type (e.g. `__m128d`)
`mocom`        | 14-15 | See `CV_MOCOM_UDT`, below.

The `hfa` bitfield is defined by this enum:

```
enum CV_HFA {
    none = 0,
    float = 1,
    double = 2,
    other = 3,
}
```

HFA (Homogeneous Floating-point Aggregate) is a concept defined by ARM architectures. See [Overview of ARM64 ABI conventions](https://learn.microsoft.com/en-us/cpp/build/arm64-windows-abi-conventions?view=msvc-170).

The `CV_MOCOM_UDT` bitfield is defined by this enum:

```
enum CV_MOCOM_UDT {
    none = 0,
    ref = 1,
    value = 2,
    interface = 3,
}
```


The `fields` field points to an `LF_FIELDLIST` or is 0 if there is no field list. Each `LF_FIELDLIST` record contains a list of fields and optionally a pointer to another `LF_FIELDLIST` if the fields cannot be stored within a single `LF_FIELDLIST` record. This allows fields to be stored in a list-of-lists.

`derivation_list` is obsolete and unused and should always be set to zero.

`vshape` points to a virtual function table shape descriptor (`LF_VFTSHAPE`), or 0 if there is none.

`length` specifies the size of instances of the class in memory, in bytes.

`name` is the name of this type.

If `hasuniquename` is set then the `unique_name` field is present and immediately follows the `name` field. If `hasuniquename` is not set then the `unique_name` field is not present _at all_; there is not even a NUL terminator for `unique_name`. This is because `unique_name` was a later extension of the specification.

## `LF_UNION` (0x1506)

```
struct Union {
    uint16_t count;
    uint16_t property;
    TypeIndex fields;
    Number length;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
}
```

Defines a `union` type. All fields of this record have the same meaning as the corresponding fields defined on the `LF_CLASS` record.

The `LF_UNION` record is similar to `LF_CLASS`, but does not support inheritance (derivation) or virtual functions. The `property` bit fields are the same as those used for `LF_CLASS`.

## `LF_ENUM` (0x1507)

```
struct Enum {
    uint16_t count;
    uint16_t property;
    TypeIndex underlying_type;
    TypeIndex fields;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
}
```

Defines an `enum` type. The `count`, `property`, `fields`, `name`, and `unique_name` fields have the same meaning as the corresponding fields defined on the `LF_CLASS` record.

The `underlying_type` field specifies the type the stores the values of the enum. This should always be a primitive type, such as `T_ULONG`; it should not point to another type record.

When C/C++ compilers emit this record, the field list will contain only `LF_ENUMERATE` fields. However, the Rust compiler may generate `LF_FIELDLIST` field lists that contain methods, even when generating field lists for `LF_ENUM` types. This is because Rust supports methods associated with all types, including enums, while C/C++ do not.

## `LF_PROCEDURE` (0x1008)

```
struct Procedure {
    TypeIndex return_value_type;
    uint8_t calling_convention;
    uint8_t reserved;
    uint16_t num_params;
    TypeIndex arg_list;
}
```

Defines the type of a function or of a static method. Instance methods use the `LF_MFUNCTION` record.

`return_value_type` specifies the return value type, e.g. `T_VOID` for `void`.

`calling_convention` specifies the calling convention of the procedure. See [`CV_call`](codeview_consts.md#cv_call---function-calling-convention).

## `LF_MFUNCTION` (0x1009)

```
struct MFunction {
    TypeIndex return_value_type;
    TypeIndex class;
    TypeIndex this;
    uint8_t calling_convention;
    uint8_t reserved;
    uint16_t num_params;
    TypeIndex arg_list;
    uint32_t this_adjust;
}
```

Defines the type of a non-static instance method. The `return_value_type`, `calling_convention`, `num_params`, and `arg_list` values have the same meaning as the fields with same name defined on `LF_PROCEDURE`.

The `this` field specifies the type of the implicit `this` argument. The `arg_list` _does not_ include an argument for the implicit `this` argument. The `num_params` field _does not_ count the implicit `this` field.

The `class` field specifies the type of the class that defines the method.

> TODO: clarify `this_adjust`

## `LF_VTSHAPE` (0x000a)

```
struct VTShape {
    uint16_t count;
    // An array of 4-bit descriptors follow, whose size is given by 'count'
}
```

Defines the format of a virtual function table. This record is accessed by the `vfunctabptr` in the member list of the class which introduces the virtual function. The `vfunctabptr` is defined either by the `LF_VFUNCTAB` or `LF_VFUNCOFF` member record.  If `LF_VFUNCTAB` record is used, then `vfunctabptr` is at the address point of the class.  If `LF_VFUNCOFF` record is used, then `vfunctabptr` is at the specified offset from the class address point.  The underlying type of the pointer is a `VTShape` type record.  This record describes how to interpret the memory at the location pointed to by the virtual function table pointer.

`count` specifies the number of descriptors.  Each value in `descriptor` describes an entry in the virtual function table. Each descriptor is 4 bits and can take one of the following values:

Value       | Description
------------|------------
0           | Near
1           | Far
2           | Thin
3           | address point displacement to outermost class. This is at `entry[-1]` from table address.
4           | far pointer to metaclass descriptor. This is at `entry[-2]` from table address.
5           | Near32
6           | Far32
7-15        | reserved

## `LF_VFTPATH` (0x100d)

```
struct VFTPath {
    uint32_t count;
    TypeIndex bases[count];
}
```

Describes the path to the virtual function table.

> TODO: Is this record actually used?  We do not see any instances of this in Windows PDBs.

## `LF_PRECOMP` (0x1509)

This type is only present in compiler PDBs and debug information embedded in object files. It is not present in linker PDBs.

```
struct Precomp {
    uint32_t start;
    uint32_t count;
    uint32_t signature;
    strz name;
}
```

This record specifies that the type records are included from the precompiled types contained in another module in the executable.  A module that contains this type record is considered to be a user of the precompiled types.  When emitting to a COFF object the section name must be `.debug$P` rather than `.debug$T`.  All other attributes should be the same.

`start` is the starting type index that is included.  This number must correspond to the current type index in the current module.

`count` is the count of the number of type indices included. After including the precompiled types, the type index must be `start + count`.

`signature` is the signature for the precompiled types being referenced by this module.  The signature will be checked against the signature in the `S_OBJNAME` symbol record and the `LF_ENDPRECOMP` type record contained in the `.debug$T` table of the creator of the precompiled types.  The signature check is used to detect recompilation of the supplier of the precompiled types without recompilation of all of the users of the precompiled types.  The method for computing the signature is unspecified.  It should be sufficiently robust to detect failures to recompile.

`name` is the full path name of the module containing the precompiled types.  This name must match the module name in the `S_OBJNAME` symbol emitted by the compiler for the object file containing the precompiled types.

## `LF_ENDPRECOMP` (0x0014) - End of Precompiled Types 

This record specifies that the preceding type records in this module can be referenced by another module in the executable.  A module that contains this type record is considered to be the creator of the precompiled types.  The subsection index for the `.debug$T` segment for a precompiled types creator is emitted as `.debug$P` instead of `.debug$T` so that the packing processing can pack the precompiled types creators before the users.

Precompiled types must be emitted as the first type records within the `.debug$T` segment and must be self-contained.  That is, they cannot reference a type record whose index is greater than or equal to the type index of the `LF_ENDPRECOMP` type record.

```
struct EndPrecompiledTypes {
    uint32_t signature;
}
```

`signature` is the signature of the precompiled types.  The signatures in the `S_OBJNAME` symbol record, the `LF_PRECOMP` type record and this signature must match.

# Records referenced from other type records

These records define parts of types, but they do not define types. For example, `LF_ARGLIST` specifies the argument list for a procedure, but the argument list alone does not specify a procedure type.

None of the records described in this section should be pointed-to by records outside of the type stream (TPI Stream).

## `LF_SKIP` (0x1200)

This record reserve space within a type stream but does not contain any information. The payload for this record can be non-empty, but its contents are ignored.

This record should never be referenced by any other record.

## `LF_ARGLIST` (0x1201) - Argument List

```
struct ArgList {
    uint32_t arg_count;
    TypeIndex args[arg_count];
}
```

Specifies the arguments for `LF_PROCEDURE` or `LF_MFUNCTION`.

This record should only be pointed-to by `LF_PROCEDURE` and `LF_MFUNCTION`, with the TPI stream.

## `LF_FIELDLIST` (0x1203) - Field List

The `LF_FIELDLIST` record contains a list of fields defined on a type. The type can be `LF_CLASS`, `LF_STRUCTURE`, `LF_INTERFACE`, `LF_UNION`, or `LF_ENUM`.

Each field within an `LF_FIELDLIST` record uses a leaf value to identify the kind of field. However, the leaf values are disjoint from those used for type records. Also, the field records do not use the same header as that used for type records. The length of each field is not stored; it is implied by the leaf value.  For this reason, decoders must know how to decode all possible fields, because if the decoder does not recognize a field then it cannot know the size of the field.

This table summarizes the different field kinds. They use the same `LF_XXX` naming convention, but these values are disjoint from the `LF_XXX` values used for type records and should not be confused with them.

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

## `LF_BITFIELD` (0x1205) - Bit-field

```
struct BitField {
    TypeIndex type;
    uint8_t length;
    uint8_t position;
}
```

`type` is the type of the field that contains the bitfield. For example, `T_ULONG`.

`length` is the length of the bitfield, in bits.

`position` is the index of the lowest bit occupied by this bitfield.

## `LF_METHODLIST` (0x1206) - Method List

```
struct MethodList {
    MethodEntry methods[];
}

struct MethodEntry {
    uint16_t attribute;
    uint16_t padding;
    TypeIndex type;
    uint32_t vtab_offset;     // This field is only present if 'attribute' introduces a new vtable slot
}
```

## `LF_REFSYM` (0x020c) - Referenced Symbol

# ID records

These records can only be placed in the IPI Stream. They cannot be placed in the TPI Stream.

Value  | Name               | Description
-------|--------------------|------------
0x1601 | `LF_FUNC_ID`       | ID of a function
0x1602 | `LF_MFUNC_ID`      | ID of a member function
0x1603 | `LF_BUILDINFO`     | Describes the environment of the compiler when the module was compiled.
0x1604 | `LF_SUBSTR_LIST`   | A list of strings
0x1605 | `LF_STRING_ID`     | A string

## `LF_FUNC_ID` (0x1601)

```
struct FuncId {
    ItemId scope;
    TypeIndex func_type;
    strz name;
    uint64_t decorated_name_hash;       // optional; may not be present
}
```

Identifies a function. This is used for global functions, regardless of linkage visibility. It is not used for member functions.

> TODO: What actually uses this record?

`scope` specifies the scope that contains this function definition. It is 0 for the global scope. If it is non-zero, then it points to an `LF_STRING` record that gives the scope. For C++, the scope is a C++ namespace. In C++, if the scope contains nested namespaces, e.g. `namespace foo { namespace bar { ... } }`, then the `LF_STRING` record will contain the namespaces, separated by `::`, e.g. `foo::bar`.

`func_type` specifies the function signature type.

`name` is the undecorated name of the function, e.g. `CreateWindowExW`.

`decorated_name_hash` is a hash of the full decorated name of a function. This field is optional; it was added as a later extension to the `LF_FUNC_ID` record. Because symbol records are required to have a size that is a multiple of 4, and because `LF_FUNC_ID` records contain a NUL-terminated string, it may be necessary to insert padding bytes at the end of the record.  However, we need to be able to distinguish between padding bytes and the presence of `decorated_name_hash`.

To do so, decode the record up to and including the `name` field. If the size of the remaining data is at least 8 bytes, then `decorated_name_hash` is present and should be decoded. The remainder of the record should be padded (as all symbol records are padded) to a multiple of 4 bytes.

> TODO: clarify what hash function is used for `decorated_name_hash`.

### Examples

```
00000000 : 00 00 00 00 80 10 00 00 52 74 6c 43 61 70 74 75 : ........RtlCaptu
00000010 : 72 65 43 6f 6e 74 65 78 74 00 32 a1 6d 2c 95 ab : reContext.2.m,..
00000020 : 82 0d f2 f1                                     : ....
```

* `scope` is 0 (global)
* `type` is 0x1046
* `name` is `RtlCaptureContext`
* `decorated_name_hash` is 0x0d82ab95_2c6da132
* Note the presence of the `f2 f1` padding bytes at the end of the record. They are not part of `decorated_name_hash`.

```
00000000 : 06 10 00 00 78 13 00 00 43 72 65 61 74 65 43 61 : ....x...CreateCa
00000010 : 63 68 65 43 6f 6e 74 65 78 74 00 dd 56 10 27 2b : cheContext..V.'+
00000020 : 85 cd 21 f1                                     : ..!.
```

* `scope` is 0x0610 and points to an `LF_STRING_ID` record whose value is `DWriteCore::ApiImpl`.
* `type` is 0x1378
* `name` is `CreateCacheContext`.
* `decorated_name_hash` is 0x21cd852b_271056dd.
* Note the presence of the `f1` padding byte at the end of the record. It is not part of `decorated_name_hash`.

## `LF_MFUNC_ID` (0x1602)

```
struct MFuncId {
    TypeIndex parent_type;
    TypeIndex func_type;
    strz name;
    uint64_t decorated_name_hash;       // optional; may not be present
}
```

Identifies a member function. This includes both static and non-static member functions.

`parent_type` specifies the type (`LF_CLASS`, `LF_STRUCTURE`, etc.) that defines the member function.

`func_type` specifies the function signature type.

`name` is the undecorated name of the function, e.g. `AddRef`. For special functions, such as constructors and conversion operations, each language has its own conventions for how to encode those names. The following is a non-exhaustive list of the special function names that have been observed in `LF_MFUNC_ID` records:

* `{ctor}` - constructors
* `{dtor}` - destructors
* `operator unsigned __int64` - conversion method
* `operator new`
* `operator=`
* `operator==`
* `operator!=`
* `operator++`

`decorated_name_hash` has the same meaning as in the `LF_FUNC_ID` record, including its interaction with padding bytes.

### Example

```
00000000 : 7f 10 00 00 a3 10 00 00 47 65 74 4e 65 78 74 45 : ........GetNextE
00000010 : 76 65 6e 74 53 6f 75 72 63 65 4f 62 6a 65 63 74 : ventSourceObject
00000020 : 49 64 00 46 48 d4 bc ac 43 c4 43 f1             : Id.FH...C.C.
```

* `parent_type` is 0x107f
* `func_type` is 0x10a3
* `name` is `GetNextEventSourceObjectId`
* `decorated_name_hash` is 0x43c443ac_bcd44846
* Note the presence of the `f1` padding byte at the end of the record. It is not part of `decorated_name_hash`.

## `LF_BUILDINFO` (0x1603)

```
struct BuildInfo {
    ItemId cwd;
    ItemId build_tool;
    ItemId source_file;
    ItemId pdb_file;
    ItemId args;
}
```

Contains information about the environment and arguments to an invocation of a tool or compiler.

Unlike most records, this record can be truncated after any field. The record can also be seen as a single array of type `ItemId`, with meanings assigned to each fixed array index.

Each field in `BuildInfo` is an `ItemId` that refers to an `LF_STRING_ID` or `LF_SUBSTR_LIST` record. See `LF_SUBSTR_LIST` for details on how string records are concatenated to form whole strings.

Each module stream may contain at most one [`S_BUILDINFO`](symbols.md#s_buildinfo-0x114c---build-info) record. If present, the `S_BUILDINFO` contains the `ItemId` that points to the `LF_BUILDINFO` record in the IPI Stream. This is the only way to associate a module with an `LF_BUILDINFO` record.

* `cwd` - The current directory when the tool ran.
* `build_tool` - The path to the tool executable, e.g. `d:\...\cl.exe`.
* `source_file` - The primary source file that was passed to the tool. For C/C++, this is usually the source file that was passed on the command-line to the compiler. For Rust, this is the path to the root module source file.
* `pdb_file` - The path to the compiler PDB (not linker PDB), if applicable. For MSVC, this will only be non-empty if the compiler was invoked with `/Zi` or `/ZI`. See: [Debug Information Format](https://learn.microsoft.com/en-us/cpp/build/reference/z7-zi-zi-debug-information-format)
* `args` - Command-line arguments that were passed to the tool.

> TODO: It appears that MSVC replaces response file arguments (e.g. `@d:\foo\args.rsp`) with their contents, when generating the string records that `LF_BUILDINFO` points to. However, we should confirm (or disprove) this.

Fields in `LF_BUILDINFO` may be absent entirely (because the structure is too small to contain the field), may have a value of 0, or may point to an empty `LF_STRING_ID` record. Decoders should make very few assumptions about the information in this record.

### Example

```
00000000 : 05 00 4c 41 00 00 ed 10 00 00 4d 41 00 00 4e 41 : ..LA......MA..NA
00000010 : 00 00 54 41 00 00 f2 f1                         : ..TA....
```

* `cwd` = `LF_STRING_ID` : `D:\\dw.main\\.build\\Windows\\x64\\src\\Binding\\FontBindingShared`
* `tool` = `LF_STRING_ID` : `C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\bin\\HostX64\\x64\\CL.exe`
* `source_file` = `LF_STRING_ID` : `D:\\dw.main\\src\\Binding\\FontBindingShared\\FontBindingShared.cpp`
* `pdb` = `LF_STRING_ID` : `D:\\dw.main\\.build\\Windows\\x64\\src\\Binding\\FontBindingShared\\Debug\\FontBinding.pdb`
* `args` = `LF_STRING_ID` : ` -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\cppwinrt\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\NETFXSDK\\4.8\\Include\\um\" -X`

## `LF_SUBSTR_LIST` (0x1604)

```
struct SubStrList {
    ItemId substrs[];
}
```

Contains a list of `ItemId` values that point to `LF_STRING_ID` records. The items in the `substr` list should be dereferenced and concatenated into one large string, in the order implied by `substr`. This is similar to a [Rope](https://en.wikipedia.org/wiki/Rope_(data_structure)).

The boundaries between the items in `substr` do not have any meaning. The divisions are simply necessary in order to keep large strings from overflowing the size limitations of the symbol record format.

### Example

```
00000000 : 09 00 00 00 f1 10 00 00 f2 10 00 00 f3 10 00 00 : ................
00000010 : f4 10 00 00 f6 10 00 00 f7 10 00 00 f8 10 00 00 : ................
00000020 : f9 10 00 00 fa 10 00 00                         : ........
```

This record contains 9 `ItemId` values, all of which point to `LF_STRING_ID` records. They are listed below:

* `-c -ID:\\dw.main\\Inc -ID:\\dw.main\\Inc\\public -ID:\\dw.main\\Inc\\internal -ID:\\dw.main\\src -Zi -nologo -W3 -WX- -diagnostics:column -Od -Ob0 -D_MBCS -DWIN32 -D_WINDOWS -DDWRITE_SUBSET_MIN=0 -DDWRITE_SUBSET_CORE=1 -DDWRITE_SUBSET=1 -DDWRITE_TARGET_WINDOWS=1`
* ` -DCMAKE_INTDIR=\\\"Debug\\\" -Gm- -EHs -EHc -RTC1 -MTd -GS -guard:cf -fp:precise -Qspectre -Zc:wchar_t- -Zc:forScope -Zc:inline -GR- -std:c++17 -permissive- -YuD:/dw.main/.build/Windows/x64/src/Common/CMakeFiles/Common.dir/Debug/cmake_pch.hxx`
* ` -FpD:\\dw.main\\.build\\Windows\\x64\\src\\Common\\Common.dir\\Debug\\cmake_pch.pch -external:W3 -Gz -TP -FID:/dw.main/.build/Windows/x64/src/Common/CMakeFiles/Common.dir/Debug/cmake_pch.hxx -errorreport:queue -validate-charset -I\"C:\\Program`
* ` Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\include\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\atlmfc\\include\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS`
* `\\include\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\ucrt\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS\\UnitTest\\include\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\um\"`
* ` -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\shared\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\winrt\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\cppwinrt\" -I\"C:\\Program`
* ` Files (x86)\\Windows Kits\\NETFXSDK\\4.8\\Include\\um\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\include\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\atlmfc\\in`
* `clude\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS\\include\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\ucrt\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliar`
* `y\\VS\\UnitTest\\include\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\um\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\shared\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\winrt\"`

As you can see, command-line arguments are split among different `LF_STRING_ID` records.

> TODO: It is not known whether `LF_SUBSTR_ID` can point to yet more `LF_SUBSTR_ID` records, forming a tree. This has not been observed in Windows PDBs.

## `LF_STRING_ID` (0x1605)

```
struct StringId {
    ItemId id;
    strz text;
}
```

Contains one string. `LF_STRING_ID` records are pointed-to by these kinds of records:

* `LF_SUBSTR_ID`
* `LF_FUNC_ID`

Each `LF_STRING_ID` directly contains a string. Some records (such as `LF_FUNC_ID`) simply always use `LF_STRING_ID`, while others appear to use `LF_STRING_ID` or `LF_SUBSTR_LIST`, in the same field.

> TODO: What is the `id` field used for?

> TODO: Clarify exactly when `LF_STRING_ID` vs. `LF_SUBSTR_LIST` is used.

## `LF_UDT_SRC_LINE` (0x1606) - UDT Source Line

```
struct UdtSrcLine {
    TypeIndex type;
    NameIndex file_name;
    uint32_t line;
}
```

`LF_UDT_SRC_LINE` specifies the source location for the definition of a user-defined type (UDT).

Source code comments in `cvinfo.h` imply that this record is generated by the compiler, not the linker.

`type` is the type being described.

`file_name` points into the [Names Stream](names_stream.md) and specifies the file name.

`line` is the 1-based line number of the definition of the UDT.

## `LF_UDT_MOD_SRC_LINE` (0x1607) - UDT Source Line in Module

```
struct UdtModSrcLine {
    TypeIndex type;
    NameIndex file_name;
    uint32_t line;
    uint16_t module_index;
}
```

`LF_UDT_MOD_SRC_LINE` specifies the source location for the definition of a user-defined type (UDT). This record is the same as `LF_MOD_SRC_LINE` except that it adds a `module_index` field.

Source code comments in `cvinfo.h` imply that this record is generated by the linker, not the compiler.

`type` is the type being described.

`file_name` points into the [Names Stream](names_stream.md) and specifies the file name.

`line` is the 1-based line number of the definition of the UDT.

`module_index` is the module index of a module which defined this type.

> TODO: There's a big problem here!  What is the proper value for `module_index` if a type was defined in more than one module? It is expected that _most_ types are defined in more than one module. If the linker collects `LF_UDT_SRC_LINE` records from different modules that point to the same `TypeIndex`, which one does it pick?  The choice is likely non-deterministic.
>
> For determinism, should we squash `module_index` and set it to some fixed value?  What actually reads `module_index`?
