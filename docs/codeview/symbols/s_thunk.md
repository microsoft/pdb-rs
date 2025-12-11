# `S_THUNK32` (0x1102) - Thunk Start

```c
struct Thunk {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t p_next;
    uint32_t offset;
    uint16_t segment;
    uint16_t thunk_length;
    uint8_t ordinal;
    strz name;
    // variant data follows
};
```

This record is used to specify any piece of code that exists outside a
procedure. It is followed by an `S_END` record. The thunk record is intended for
small code fragments. and a two byte length field is sufficient for its intended
purpose.

The `p_parent`, `p_end`, `p_next`, `offset`, and `segment` fields have the same
meaning as the fields with the same name within `S_LPROC32`.
