## `S_UDT` (0x1108) - User-Defined Type

```c
struct Udt {
  TypeIndex type;
  strz name;
};
```

A user-defined type (UDT). This symbol is usually found in the global symbol
stream, but in rare cases can also be found in a module symbol stream.

`type` points into the TPI Stream. The pointed-to type should be `LF_ENUM`,
`LF_STRUCTURE`, `LF_CLASS`, or `LF_INTERFACE`.

> TODO: Can `S_UDT` point to primitives?  What about typedefs?

The `name` field of the `S_UDT` symbol record should be equal to the name field
of the pointed-to type. The `S_UDT` symbol can appear in the global symbol
stream and in module symbol streams.

