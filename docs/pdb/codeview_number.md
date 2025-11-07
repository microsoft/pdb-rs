# Numbers

Type records and symbol records may contain instances of the `Number` type. In
the PDB documentation, these are called "numeric leaves".

`Number` contains a numeric value of a given value and type. The type of the
number is represented in its encoding, so there is no need for an external
specification of the type of the number.

`Number` is a variable-length data type. Its size can be determined from its
encoded form.

The representation of `Number` always requires at least two bytes. These first
two bytes either specify the literal value of the number (with an implied type),
or they specify the type and size of the `Number`. The first two bytes are
encoded as a little-endian `uint16_t` value, and we refer to this `uint16_t`
value as the _leaf_.

If _leaf_ is less than 0x8000, then the type is implicitly `LF_USHORT` (an
unsigned 16-bit value), the value is simply the leaf value itself, and there are
no additional bytes after the first two bytes. This is the simplest case and is
very common.

If _leaf_ is greater or equal to 0x8000, then the leaf specifies the type and
size of the `Number`:

Leaf   | Leaf Name       | Size<br>(bytes) | Description
-------|-----------------|----|------------
0x8000 | `LF_CHAR`       | 1  | signed 8-bit integer
0x8001 | `LF_SHORT`      | 2  | signed 16-bit integer
0x8002 | `LF_USHORT`     | 2  | unsigned 16-bit integer
0x8003 | `LF_LONG`       | 4  | signed 32-bit integer
0x8004 | `LF_ULONG`      | 4  | unsigned 32-bit integer
0x8005 | `LF_REAL32`     | 4  | 32-bit floating-point
0x8006 | `LF_REAL64`     | 8  | 64-bit floating-point
0x8007 | `LF_REAL80`     | 10 | 80-bit floating-point
0x8008 | `LF_REAL128`    | 16 | 128-bit floating-point
0x8009 | `LF_QUADWORD`   | 8  | signed 64-bit integer
0x800a | `LF_UQUADWORD`  | 8  | unsigned 64-bit integer
0x800b | `LF_REAL48`     | 6  | 48-bit floating-point
0x800c | `LF_COMPLEX32`  | 8  | 32-bit floating-point complex (real, imaginary)
0x800d | `LF_COMPLEX64`  | 16 | 64-bit floating-point complex (real, imaginary)
0x800e | `LF_COMPLEX80`  | 20 | 80-bit floating-point complex (real, imaginary)
0x800f | `LF_COMPLEX128` | 32 | 128-bit floating-point complex (real, imaginary)
0x8010 | `LF_VARSTRING`  | \* | Variable-length string; see below
0x8017 | `LF_OCTWORD`    | 16 | signed 128-bit integer
0x8018 | `LF_UOCTWORD`   | 16 | unsigned 128-bit integer
0x8019 | `LF_DECIMAL`    | 16 | OLE variant decimal value
0x801a | `LF_DATE`       | 8  | OLE variant date value
0x801b | `LF_UTF8STRING` | \* | UTF-8 NUL-terminated string

In this table, the "Size (Bytes)" column specifies the number of bytes that
follow the leaf value; the size does not count the number of bytes in the leaf
value itself.

There is no leaf value for an unsigned 8-bit value because all unsigned 8-bit
values can be stored directly in _leaf_.

## `LF_VARSTRING` (0x8010)

For `LF_VARSTRING`, the leaf value is followed by a `uint16_t` value that
specifies the length in bytes of the string data, followed by the length data.
There is no alignment padding after the string data.
