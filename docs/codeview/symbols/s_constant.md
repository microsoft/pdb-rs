
## `S_CONSTANT` (0x1107) - Constant

```c
struct Constant {
  TypeIndex type;
  Number value;
  strz name;
};
```

Defines a named constant. This symbol can appear in both the global symbol
stream and in module symbol streams.

See [Number](../codeview_number.md) for the encoding of `value`.

## `S_MANCONSTANT` (0x112d) - Managed Constant

```c
struct ManagedConstant {
    uint32_t metadata_token;
    Number value;
    strz name;
};
```

Defines a named constant whose type is defined by MSIL metadata. This symbol has
been observed only in module streams.
