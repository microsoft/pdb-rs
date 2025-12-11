# `LF_POINTER` (0x1002)

Defines a type that is a pointer to another type. This is used for C-style
pointers, such as `FOO *`, as well as C++ references such as `FOO &`.

```c
struct Pointer {
    TypeIndex type;        // The type that is being pointed-to
    uint32_t attributes;   // Flags; see below
    // more data follows; structure depends on attributes
};
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

Value | Name           | Description
------|----------------|---------------------------------------
0x00  | `NEAR`         | 16 bit pointer
0x01  | `FAR`          | 16:16 far pointer
0x02  | `HUGE`         | 16:16 huge pointer
0x03  | `BASE_SEG`     | based on segment
0x04  | `BASE_VAL`     | based on value of base
0x05  | `BASE_SEGVAL`  | based on segment value of base
0x06  | `BASE_ADDR`    | based on address of base
0x07  | `BASE_SEGADDR` | based on segment address of base
0x08  | `BASE_TYPE`    | based on type
0x09  | `BASE_SELF`    | based on self
0x0a  | `NEAR32`       | 32 bit pointer
0x0b  | `FAR32`        | 16:32 pointer
0x0c  | `64`           | 64 bit pointer
0x0d  | `UNUSEDPTR`    | first unused pointer type

The `ptrmode` bit field can take these values:

Value | Name      | Description
------|-----------|---------------------------------------
0x00  | `PTR`     | "normal" pointer, e.g. `FOO *`
0x01  | `REF`     | "old" reference, e.g. `FOO &`
0x01  | `LVREF`   | l-value reference, e.g. `FOO &`
0x02  | `PMEM`    | pointer to data member
0x03  | `PMFUNC`  | pointer to member function
0x04  | `RVREF`   | r-value reference, e.g. `FOO &&`

The data after the `Pointer` structure depends on `ptrtype`, and is called the "variant data".

### Variant data for pointer to type

If the pointer is based on a type (`ptrtype == CV_PTR_BASE_TYPE`), then the
variant data consists of a single `TypeIndex`.

### Variant data for pointer to data member

If the pointer is a pointer to a data member, then the variant data has this
structure:

```c
struct PointerToDataMemberVariant {
    TypeIndex class;        // The pointed-to class
    uint16_t format;
};
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

The pointer to data member and pointer to method have the following formats in
memory. In the following descriptions of the format and value of the NULL
pointer, `*` means any value.

> TODO: convert these; they are quite complicated
