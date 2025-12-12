# `S_INLINESITE` (0x114d) and `S_INLINESITE2` (0x115c)

`S_INLINESITE` describes where one procedure was inlined into another.
`S_INLINESITE2` is a refinement of `S_INLINESITE`.

`S_INLINESITE` records may only appear within [Procedures](./s_procs.md).

```c
struct InlineSite {
    uint32_t p_parent;
    uint32_t p_end;
    ItemId inlinee;
};

struct InlineSite2 {
    uint32_t p_parent;
    uint32_t p_end;
    ItemId inlinee;
    uint8_t binary_annotations[];
};
```
