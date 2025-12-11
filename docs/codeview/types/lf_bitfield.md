## `LF_BITFIELD` (0x1205) - Bit-field

Describes a bitfield within a structure.

```c
struct BitField {
    TypeIndex type;
    uint8_t length;
    uint8_t position;
};
```

`type` is the type of the field that contains the bitfield. For example,
`T_ULONG`.

`length` is the length of the bitfield, in bits.

`position` is the index of the lowest bit occupied by this bitfield.
