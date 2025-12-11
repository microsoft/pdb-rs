# `LF_MODIFIER` (0x1001)

Modifies another type record by qualifying it with `const`, `volatile`, or
`unaligned` modifiers.

For example, `const char *` would be described with an `LF_MODIFIER` type
(applied to the primitive type `T_CHAR`), and a `LF_POINTER` type that points to
the `LD_MODIFIER`.

```c
struct Modifier {
    TypeIndex index;
    uint16_t attribute;
};
```

`index` identifies the type that this type is based on.

`attribute` is a set of bit fields. If a bit is set to 1, then that qualifier
applies:

Field       | Bits      | Description
------------|-----------|------------
`const`     | 0         | `const` qualifier
`volatile`  | 1         | `volatile` qualifier
`unaligned` | 2         | `unaligned` qualifier
(reserved)  | 3-15      | reserved; must be zero

> **Determinism**: `LF_MODIFIER` records should not point to another
> `LF_MODIFIER` record. Instead, each `LF_MODIFIER` record should point directly
> to the unqualified type, and should contain the union of all qualifiers that
> are needed.
