## `LF_UNION` (0x1506)

```c
struct Union {
    uint16_t count;
    uint16_t property;
    TypeIndex fields;
    Number length;
    strz name;
    strz unique_name;     // present only if `hasuniquename` is set in `property`
};
```

Defines a `union` type. All fields of this record have the same meaning as the corresponding fields defined on the `LF_CLASS` record.

The `LF_UNION` record is similar to `LF_CLASS`, but does not support inheritance (derivation) or virtual functions. The `property` bit fields are the same as those used for `LF_CLASS`.
