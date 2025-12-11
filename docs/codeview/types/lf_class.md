# `LF_CLASS` (0x1504) and related

This record is used for `LF_CLASS`, `LF_STRUCTURE`, and `LF_INTERFACE`.

```c
struct Class {
    uint16_t count;
    uint16_t property;
    TypeIndex fields;
    TypeIndex derivation_list;
    TypeIndex vshape;
    Number length;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
};
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

```c
enum CV_HFA {
    none = 0,
    float = 1,
    double = 2,
    other = 3,
};
```

HFA (Homogeneous Floating-point Aggregate) is a concept defined by ARM architectures. See [Overview of ARM64 ABI conventions](https://learn.microsoft.com/en-us/cpp/build/arm64-windows-abi-conventions?view=msvc-170).

The `CV_MOCOM_UDT` bitfield is defined by this enum:

```c
enum CV_MOCOM_UDT {
    none = 0,
    ref = 1,
    value = 2,
    interface = 3,
};
```


The `fields` field points to an `LF_FIELDLIST` or is 0 if there is no field list. Each `LF_FIELDLIST` record contains a list of fields and optionally a pointer to another `LF_FIELDLIST` if the fields cannot be stored within a single `LF_FIELDLIST` record. This allows fields to be stored in a list-of-lists.

`derivation_list` is obsolete and unused and should always be set to zero.

`vshape` points to a virtual function table shape descriptor (`LF_VFTSHAPE`), or 0 if there is none.

`length` specifies the size of instances of the class in memory, in bytes.

`name` is the name of this type.

If `hasuniquename` is set then the `unique_name` field is present and immediately follows the `name` field. If `hasuniquename` is not set then the `unique_name` field is not present _at all_; there is not even a NUL terminator for `unique_name`. This is because `unique_name` was a later extension of the specification.
