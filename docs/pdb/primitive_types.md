# Primitive Types

CodeView defines a set of _primitive types_. Primitive types have specified
meanings and do not require type records within a PDB to define the primitive
type. For example, the C/C++ types `void`, `unsigned int`, and `char` are
primitive types.

Primitive types are identified by `TypeIndex` values that are less than the
`type_index_begin` value defined in the TPI Stream Header. In practice,
`type_index_begin` is always equal to 4096, and many values below 4096 are
already assigned meanings by CodeView, so it is unlikely that `type_index_begin`
will have a lower value in the future.

The `TypeIndex` value for primitive types is composed of the following bit
fields. Because primitive types are in the reserved range of 0-4095, the
`TypeIndex` value has 12 significant bits.

Field        | Bits     | Description
-------------|----------|------------
`size`       | 0-2      | Specifies the size of the primitive; see below.
(reserved)   | 3        | Reserved; always zero.
`type`       | 4-7      | Specifies the type of the primitive; see below.
`mode`       | 8-10     | Specifies the _mode_ of the primitive; see below.
(reserved)   | 11       | Reserved; always zero.

The `type` field can take on these values:

`type` value  | `type` name    | Usage
--------------|----------------|------
0x00          | `TY_SPECIAL`   | Special
0x01          | `TY_SINT`      | Signed integer
0x02          | `TY_UINT`      | Unsigned integer
0x03          | `TY_BOOLEAN`   | Boolean
0x04          | `TY_REAL`      | Real (floating-point)
0x05          | `TY_COMPLEX`   | Complex
0x06          | `TY_SPECIAL2`  | Special2
0x07          | `TY_REALLYINT` | "Really" int value
0x08 - 0x0f   |                | Reserved

The interpretation of the `size` field depends on the `type` field. It can taken
on these values:

`size` value | `TY_SPECIAL` usage | `TY_SINT`, `TY_UINT`,<br>`TY_BOOLEAN` usage | `TY_REALLYINT` | `TY_REAL` and <br>`TY_COMPLEX` usage | `TY_SPECIAL2` usage
---------|--------------------|-----------|--------------|-----------|--------------
0        | No type            | 1 byte    | `char`       | 32-bit    | bit
1        | absolute symbol    | 2 bytes   | `wchar_t`    | 64-bit    | Pascal `CHAR`
2        | segment            | 4 bytes   | `int16_t`    | 80-bit    | reserved
3        | C/C++ `void` type  | 8 bytes   | `uint16_t`   | 128-bit   | reserved
4        | obsolete           | reserved  | `int32_t`    | 48-bit    | reserved
5        | obsolete           | reserved  | `uint32_t`   | reserved  | reserved
6        | obsolete           | reserved  | `int64_t`    | reserved  | reserved
7        | obsolete           | reserved  | `uint64_t`   | reserved  | reserved

The `mode` field specifies the pointer mode for pointer types. It can take on
thse values:

`mode` value | `mode` name    | Usage
-------------|----------------|------
0            | `MODE_DIRECT`  | Direct value; not a pointer
1            | `MODE_NEAR`    | Near pointer
2            | `MODE_FAR`     | Far pointer
3            | `MODE_HUGE`    | Huge pointer
4            | `MODE_NEAR32`  | 32-bit near pointer
5            | `MODE_FAR32`   | 32-bit far pointer
6            | `MODE_NEAR64`  | 64-bit near pointer

The distinctions between "near", "far", and "huge" pointers date back to
segmented 80x86 CPU architectures. They are no longer relevant, since modern
systems always use flat memory models. For 32-bit architectures (x86, ARM32),
`mode` is `MODE_NEAR32`. For 64-bit architectures (AMD64, ARM64), `mode` is
`MODE_NEAR64`.

CodeView defines types for segmented architectures (such as 8086), which are
obsolete. This specification omits these obsolete variants.

> TODO: actually delete obsolete variants from the lists below

# C/C++ types and explicitly-sized ("really") types

C/C++ defines a family of integer types, such as `int`, `short int`, `long int`,
`long long int`, and their unsigned variants. The C/C++ language specification
does not mandate a specific bit size for each type. Instead, the C/C++
specification defines that the bit size (and therefore range) of `int` is equal
to or greater than that of `short int`, and that `long int` is equal to or
greater than that of `int`, and similar for `long long int`.

MSVC uses specific bit sizes for these types, but MSVC also defines a separate
set of integer types which have explicit sizes. These are separate types, even
if they are trivially convertible between the C/C++ types of the same bit size.
These explicitly-sized types are the "really" types. In C/C++ source code, they
are identified by `__int8`, `__int32`, `unsigned __int64`, etc.

# Types summary

Most types also have a 32-bit pointer type and a 64-bit pointer type. There are many obsolete types that are not shown in this table.

Most compilers, such as the MSVC C/C++ compiler, MASM, and Rust, can generate only a subset of the types in this table.

Name          |Value | 32-bit<br>Pointer Name | Value  | 64-bit<br>Pointer Name | Value | Size<br>(bytes) | C/C++ Type | Description
--------------|------|--------------|------|--------------|------|--|-------------------|------------------------
`T_NOTYPE`    |0x0000|              |      |              |      |  |                   | Uncharacterized type
`T_ABS`       |0x0001|              |      |              |      |  |                   | Absolute symbol
`T_SEGMENT`   |0x0002|              |      |              |      |  |                   | Segment type (obsolete)
`T_VOID`      |0x0003|`T_32PVOID`   |0x0403|`T_64PVOID`   |0x0603|  |`void`             | The C/C++ `void` type
`T_CHAR`      |0x0010|`T_32PCHAR`   |0x0410|`T_64PCHAR`   |0x0610| 1|`char`             | 8-bit signed 
`T_QUAD`      |0x0013|`T_32PQUAD`   |0x0413|`T_64PQUAD`   |0x0613| 8|`long long`        | 64-bit signed 
`T_SHORT`     |0x0011|`T_32PSHORT`  |0x0411|`T_64PSHORT`  |0x0611| 2|`short`            | 16-bit signed
`T_LONG`      |0x0012|`T_32PLONG`   |0x0412|`T_64PLONG`   |0x0612| 4|`long`             | 32-bit signed 
`T_UCHAR`     |0x0020|`T_32PUCHAR`  |0x0420|`T_64PUCHAR`  |0x0620| 1|`unsigned char`    | 8-bit unsigned
`T_USHORT`    |0x0021|`T_32PUSHORT` |0x0421|`T_64PUSHORT` |0x0621| 2|`unsigned short`   | 16-bit unsigned
`T_ULONG`     |0x0022|`T_32PULONG`  |0x0422|`T_64PULONG`  |0x0622| 4|`unsigned long`    | 32-bit unsigned
`T_UQUAD`     |0x0023|`T_32PUQUAD`  |0x0423|`T_64PUQUAD`  |0x0623| 8|`unsigned long long` | 64-bit unsigned
`T_BOOL08`    |0x0030|`T_32PBOOL08` |0x0430|`T_64PBOOL08` |0x0630| 1|`bool`             | 8-bit Boolean value
`T_BOOL16`    |0x0031|`T_32PBOOL16` |0x0431|`T_64PBOOL16` |0x0631| 2|                   | 16-bit Boolean value
`T_BOOL32`    |0x0032|`T_32PBOOL32` |0x0432|`T_64PBOOL32` |0x0632| 4|                   | 32-bit Boolean value
`T_BOOL64`    |0x0033|`T_32PBOOL64` |0x0433|`T_64PBOOL64` |0x0633| 8|                   | 64-bit Boolean value
`T_RCHAR`     |0x0070|`T_32PRCHAR`  |0x0470|`T_64PRCHAR`  |0x0670|  |                   | real char: is this `__int8`?
`T_WCHAR`     |0x0071|`T_32PWCHAR`  |0x0471|`T_64PWCHAR`  |0x0671| 2|`wchar_t`          | wide char
`T_INT2`      |0x0072|`T_32PINT2`   |0x0472|`T_64PINT2`   |0x0672| 2|`__int16`          | 16 bit signed int
`T_UINT2`     |0x0073|`T_32PUINT2`  |0x0473|`T_64PUINT2`  |0x0673| 2|`unsigned __int16` | 16 bit unsigned int
`T_INT4`      |0x0074|`T_32PINT4`   |0x0474|`T_64PINT4`   |0x0674| 4|`__int32`          | really 32 bit signed int
`T_UINT4`     |0x0075|`T_32PUINT4`  |0x0475|`T_64PUINT4`  |0x0675| 4|`unsigned __int32` | really 32 bit unsigned int
`T_INT8`      |0x0076|`T_32PINT8`   |0x0476|`T_64PINT8`   |0x0676| 8|`__int64`          | 64-bit signed  int
`T_UINT8`     |0x0077|`T_32PUINT8`  |0x0477|`T_64PUINT8`  |0x0677| 8|`unsigned __int64` | 64-bit unsigned int
`T_REAL32`    |0x0040|`T_32PREAL32` |0x0440|`T_64PREAL32` |0x0640| 4|`float`            | 32-bit real 
`T_REAL48`    |0x0044|`T_32PREAL48` |0x0444|`T_64PREAL48` |0x0644| 6|                   | 48-bit real
`T_REAL64`    |0x0041|`T_32PREAL64` |0x0441|`T_64PREAL64` |0x0641| 8|`double`           | 64-bit real 
`T_REAL80`    |0x0042|`T_32PREAL80` |0x0442|`T_64PREAL80` |0x0642|10|                   | 80-bit real 
`T_REAL128`   |0x0043|`T_32PREAL128`|0x0443|`T_64PREAL128`|0x0643|16|                   | 128-bit real 
`T_CPLX32`    |0x0050|`T_32PCPLX32` |0x0450|`T_64PCPLX32` |0x0650| 8|                   | 32-bit complex 
`T_CPLX64`    |0x0051|`T_32PCPLX64` |0x0451|`T_64PCPLX64` |0x0651|16|                   | 64-bit complex 
`T_CPLX80`    |0x0052|`T_32PCPLX80` |0x0452|`T_64PCPLX80` |0x0652|20|                   | 80-bit complex 
`T_CPLX128`   |0x0053|`T_32PCPLX128`|0x0453|`T_64PCPLX128`|0x0653|32|                   | 128-bit complex

# Special types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_NOTYPE`    | 0x0000 |                       | Uncharacterized type (no type)
`T_ABS`       | 0x0001 |                       | Absolute symbol
`T_SEGMENT`   | 0x0002 |                       | Segment type (obsolete)
`T_VOID`      | 0x0003 | `void`                | The C/C++ `void` type
`T_32PVOID`   | 0x0403 | `void *`              | 32 bit near pointer to `void`
`T_64PVOID`   | 0x0603 | `void *`              | 64-bit pointer to `void`

# Character types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_CHAR`      | 0x0010 |                       | 8-bit signed 
`T_UCHAR`     | 0x0020 |                       | 8-bit unsigned
`T_32PCHAR`   | 0x0410 |                       | 32-bit pointer to 8-bit signed
`T_32PUCHAR`  | 0x0420 |                       | 32-bit pointer to 8-bit unsigned
`T_64PCHAR`   | 0x0610 |                       | 64-bit pointer to 8 bit signed
`T_64PUCHAR`  | 0x0620 |                       | 64-bit pointer to 8 bit unsigned

# Really a character types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_RCHAR`     | 0x0070 |                       | real char
`T_32PRCHAR`  | 0x0470 |                       | 32-bit pointer to a real char
`T_64PRCHAR`  | 0x0670 |                       | 64-bit pointer to a real char

# Wide character types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_WCHAR`     | 0x0071 | `wchar_t`             | wide char
`T_32PWCHAR`  | 0x0471 | `wchar_t *`           | 32-bit pointer to a wide char
`T_64PWCHAR`  | 0x0671 | `wchar_t *`           | 64-bit pointer to a wide char

# Really 16 bit integer types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_INT2`      | 0x0072 | `__int16`             | 16 bit signed int
`T_UINT2`     | 0x0073 | `unsigned __int16`    | 16 bit unsigned int
`T_32PINT2`   | 0x0472 | `__int16 *`           | near pointer to 16 bit signed int
`T_32PUINT2`  | 0x0473 | `unsigned __int16 *`  | near pointer to 16 bit unsigned int
`T_64PINT2`   | 0x0672 | `__int16 *`           | 64-bit pointer to 16 bit signed int
`T_64PUINT2`  | 0x0673 | `unsigned __int16 *`  | 64-bit pointer to 16 bit unsigned int

# 16-bit short types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_SHORT`     | 0x0011 | `short`               | 16-bit signed
`T_USHORT`    | 0x0021 | `unsigned short`      | 16-bit unsigned
`T_32PSHORT`  | 0x0411 | `short *`             | 32-bit pointer to 16 bit signed
`T_32PUSHORT` | 0x0421 | `unsigned short *`    | 32-bit pointer to 16 bit unsigned
`T_64PSHORT`  | 0x0611 | `short *`             | 64-bit pointer to 16 bit signed
`T_64PUSHORT` | 0x0621 | `unsigned short *`    | 64-bit pointer to 16 bit unsigned

# Really 32 bit integer types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_INT4`      | 0x0074 |                       | really 32 bit signed int
`T_UINT4`     | 0x0075 |                       | really 32 bit unsigned int
`T_32PINT4`   | 0x0474 |                       | 32-bit pointer to 32 bit signed int
`T_32PUINT4`  | 0x0475 |                       | 32-bit pointer to 32 bit unsigned int
`T_64PINT4`   | 0x0674 |                       | 64-bit pointer to 32 bit signed int
`T_64PUINT4`  | 0x0675 |                       | 64-bit pointer to 32 bit unsigned int

# 32-bit long types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_LONG`      | 0x0012 |                       | 32-bit signed 
`T_ULONG`     | 0x0022 |                       | 32-bit unsigned
`T_32PLONG`   | 0x0412 |                       | 32-bit pointer to 32 bit signed 
`T_32PULONG`  | 0x0422 |                       | 32-bit pointer to 32 bit unsigned 
`T_64PLONG`   | 0x0612 |                       | 64-bit pointer to 32 bit signed 
`T_64PULONG`  | 0x0622 |                       | 64-bit pointer to 32 bit unsigned

# Really 64-bit integer types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_INT8`      | 0x0076 |                       | 64-bit signed  int
`T_UINT8`     | 0x0077 |                       | 64-bit unsigned int
`T_32PINT8`   | 0x0476 |                       | 32-bit pointer to 64 bit signed int
`T_32PUINT8`  | 0x0477 |                       | 32-bit pointer to 64 bit unsigned int
`T_64PINT8`   | 0x0676 |                       | 64-bit pointer to 64 bit signed int
`T_64PUINT8`  | 0x0677 |                       | 64-bit pointer to 64 bit unsigned int

# 64-bit integral types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_QUAD`      | 0x0013 |                       | 64-bit signed 
`T_UQUAD`     | 0x0023 |                       | 64-bit unsigned
`T_32PQUAD`   | 0x0413 |                       | 32-bit pointer to 64 bit signed 
`T_32PUQUAD`  | 0x0423 |                       | 32-bit pointer to 64 bit unsigned 
`T_64PQUAD`   | 0x0613 |                       | 64-bit pointer to 64 bit signed 
`T_64PUQUAD`  | 0x0623 |                       | 64-bit pointer to 64 bit unsigned 

# 32-bit real types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_REAL32`    | 0x0040 | `float`               | 32-bit real 
`T_32PREAL32` | 0x0440 | `float *`             | 32-bit pointer to 32 bit real
`T_64PREAL32` | 0x0640 | `float *`             | 64 pointer to 32 bit real

# 48-bit real types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_REAL48`    | 0x0044 | 48-bit real 
`T_32PREAL48` | 0x0444 | 32-bit pointer to 48 bit real
`T_64PREAL48` | 0x0644 | 64-bit pointer to 48 bit real

# 64-bit real types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_REAL64`    | 0x0041 | 64-bit real 
`T_32PREAL64` | 0x0441 | 32-bit pointer to 64 bit real
`T_64PREAL64` | 0x0641 | 64-bit pointer to 64 bit real

# 80-bit real types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_REAL80`    | 0x0042 | 80-bit real 
`T_32PREAL80` | 0x0442 | 32-bit pointer to 80 bit real
`T_64PREAL80` | 0x0642 | 64-bit pointer to 80 bit real

# 128-bit real types

Name            | Value  | C/C++ Type            | Description
----------------|--------|-----------------------|------------
`T_REAL128`     | 0x0043 | 128-bit real 
`T_32PREAL128`  | 0x0443 | 32-bit pointer to 128 bit real
`T_64PREAL128`  | 0x0643 | 64-bit pointer to 128 bit real

# 32-bit complex types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_CPLX32`    | 0x0050 | 32-bit complex 
`T_32PCPLX32` | 0x0450 | 32-bit pointer to 32 bit complex
`T_64PCPLX32` | 0x0650 | 64-bit pointer to 32 bit complex

# 64-bit complex types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_CPLX64`    | 0x0051 | 64-bit complex 
`T_32PCPLX64` | 0x0451 | 32-bit pointer to 64 bit complex
`T_64PCPLX64` | 0x0651 | 64-bit pointer to 64 bit complex

# 80-bit complex types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_CPLX80`    | 0x0052 | 80-bit complex 
`T_32PCPLX80` | 0x0452 | 32-bit pointer to 80 bit complex
`T_64PCPLX80` | 0x0652 | 64-bit pointer to 80 bit complex

# 128-bit complex types

Name            | Value  | C/C++ Type            | Description
----------------|--------|-----------------------|------------
`T_CPLX128`     | 0x0053 |                       | 128-bit complex 
`T_32PCPLX128`  | 0x0453 |                       | 32-bit pointer to 128 bit complex
`T_64PCPLX128`  | 0x0653 |                       | 64-bit pointer to 128 bit complex

# Boolean types

Name          | Value  | C/C++ Type            | Description
--------------|--------|-----------------------|------------
`T_BOOL08`    | 0x0030 |                       | 8-bit boolean
`T_BOOL16`    | 0x0031 |                       | 16-bit boolean
`T_BOOL32`    | 0x0032 |                       | 32-bit boolean
`T_BOOL64`    | 0x0033 |                       | 64-bit boolean
`T_32PBOOL08` | 0x0430 |                       | 32-bit pointer to 8 bit boolean
`T_32PBOOL16` | 0x0431 |                       | 32-bit pointer to 16 bit boolean
`T_32PBOOL32` | 0x0432 |                       | 32-bit pointer to 32 bit boolean
`T_32PBOOL64` | 0x0433 |                       | 32-bit pointer to 64-bit boolean
`T_64PBOOL08` | 0x0630 |                       | 64-bit pointer to 8 bit boolean
`T_64PBOOL16` | 0x0631 |                       | 64-bit pointer to 16 bit boolean
`T_64PBOOL32` | 0x0632 |                       | 64-bit pointer to 32 bit boolean
`T_64PBOOL64` | 0x0633 |                       | 64-bit pointer to 64-bit boolean
