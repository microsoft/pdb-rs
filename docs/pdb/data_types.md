# Data Structures and Types

This document defines many data structures. This section describes the notation used for those data structures.

Structures are defined using pseudo-C code, using the `struct { ... }` syntax.  Unlike C, structures defined in this document may be variable-length and may contain variable-length fields.  Variable-length fields of structures may be arrays of fixed-size structures, arrays of variable-size structures, NUL-terminated strings, the Number type, or a reference to another variable-length structure defined in this document.

# Range syntax

Numeric ranges are specified using `start-end`, where both `start` and `end` are inclusive bounds.  For example, "bits 3-5" means "bits 3, 4, and 5".

# Fixed-size Integers

This document uses the C/C++ integer types defined in `<stdint.h>`. All integer values are in LSB-first (little-endian) encoding, unless specified otherwise.

These are the integer types used in this document:

Name        | Usage
------------|------
`uint8_t`   | (obvious)
`uint16_t`  |   
`uint32_t`  |   
`uint64_t`  |   
`int8_t`    | 
`int16_t`   |  
`int32_t`   |  
`int64_t`   |  
`char8_t`   | 8-bit value used for encoding code units of UTF-8 strings.
 
# GUID

The well-known `GUID` type is imported from Windows. It is a 16-byte quantity.
 
# Embedded arrays

Variable-length arrays may be directly embedded within a structure. It is not the case that a fixed amount of storage is reserved. Instead, the elements are directly embedded within the structure, followed by the next field (if any).  The number of elements in the array is usually given as an expression within the array’s square brackets.  

Example:

```
struct Garden {
  uint32_t num_apples;
  Apple apples[num_apples];
};
```

If the last field of a structure is an array field, and the number of elements is implied by the length of the entire encoded structure, then the length may be omitted. Example:

```
struct MoreGarden {
  uint32_t num_apples;
  Apple apples[num_apples];
  Tomato tomatoes[];   // the rest of the storage is tomatoes
};
```

It is not legal to have another field follow an array with implicit length, unless the meaning is clarified by context. The following is an example of an illegal structure definition:

```
struct BadDefinition {
  uint32_t numbers[];
  uint32_t x;
};
```

# Strings: strz

The data type `strz` represents a NUL-terminated string, using UTF-8 encoding. For example:

```
struct Foo {
    uint32_t month;   // fixed-length header
    uint32_t day;
    uint32_t year;

    strz flavor;      // variable-length fields
    strz build;
}
```

This syntax means that the string data is directly stored in the structure, and that the structure is variable-length. It is not the case that a fixed number of characters are allocated for the string; instead, character data is written, followed by a NUL (a zero byte), followed by the next field (if any).

# The `Number` type

The `Number` type represents a numeric constant. Numbers may have different sizes and representations. Each Number value is at least 2 bytes in size. These first 2 bytes specify either an immediate value or specify the encoding to use for the entire value.

See: [Numbers](number.md)

## Example of using `Number`:

```
struct NamedConstant {
    strz name;
    Number value;
}
```

Both of the fields of `NamedConstant` are variable-length.

# `TypeIndex`: Pointer into TPI Stream

`TypeIndex` is an alias for `uint32_t`, with a specific interpretation. Each `TypeIndex` either refers to a record within the Type Database (TPI Stream), which is described later in this document, or refers to a primitive (well-known / intrinsic) type. See the Type Database section for more information.

# `ItemId`: Pointer into IPI Stream

`ItemId` is an alias for `uint32_t`, with a specific interpretation. Each `ItemId` refers to a record within the IPI Stream, or to `NIL` (no record). The value of `NIL` is zero. `ItemId` counts records, not bytes.

See [`ItemId`](ipi.md#itemid).

# `NameIndex`

 `NameIndex` is an alias for `uint32_t` which points into the [Names Stream](names_stream.md).
