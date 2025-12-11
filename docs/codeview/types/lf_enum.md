# `LF_ENUM` (0x1507)

```c
struct Enum {
    uint16_t count;
    uint16_t property;
    TypeIndex underlying_type;
    TypeIndex fields;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
};
```

Defines an `enum` type. The `count`, `property`, `fields`, `name`, and
`unique_name` fields have the same meaning as the corresponding fields defined
on the `LF_CLASS` record.

The `underlying_type` field specifies the type the stores the values of the
enum. This should always be a primitive type, such as `T_ULONG`; it should not
point to another type record.

When C/C++ compilers emit this record, the field list will contain only
`LF_ENUMERATE` fields. However, the Rust compiler may generate `LF_FIELDLIST`
field lists that contain methods, even when generating field lists for `LF_ENUM`
types. This is because Rust supports methods associated with all types,
including enums, while C/C++ do not.
