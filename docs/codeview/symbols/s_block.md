## `S_BLOCK32` (0x1103) - Block Start

Describes the start of an inner block of lexically scoped symbols within a
[Procedure](./s_procs.md). The lexical scope is terminated by a matching `S_END`
symbol.

```c
struct Block {
    uint32_t p_parent;
    uint32_t p_end;
    uint32_t length;
    uint32_t offset;
    uint16_t segment;
    strz name;
};
```

This symbol must be nested (directly or indirectly) within a procedure. It may
be nested within another `S_BLOCK32` or inline call site. This also implies that
`S_BLOCK32` can only occur within module symbol streams.
