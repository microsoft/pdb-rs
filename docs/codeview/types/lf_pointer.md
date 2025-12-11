# `LF_POINTER` (0x1002)

Defines a type that is a pointer to another type. This is used for C-style
pointers, such as `FOO *`, as well as C++ references such as `FOO &`.

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

```c
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
};
```

The `ptrmode` bit field can take these values:

```c
enum CV_ptrmode {
    CV_PTR_MODE_PTR      = 0x00, // "normal" pointer, e.g. `FOO *`
    CV_PTR_MODE_REF      = 0x01, // "old" reference, e.g. `FOO &`
    CV_PTR_MODE_LVREF    = 0x01, // l-value reference, e.g. `FOO &`
    CV_PTR_MODE_PMEM     = 0x02, // pointer to data member
    CV_PTR_MODE_PMFUNC   = 0x03, // pointer to member function
    CV_PTR_MODE_RVREF    = 0x04, // r-value reference, e.g. `FOO &&`
};
```

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
