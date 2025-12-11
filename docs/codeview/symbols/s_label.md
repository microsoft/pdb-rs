## `S_LABEL32` (0x1105) - Code Label

```c
struct Label {
    uint32_t offset;
    uint16_t segment;
    uint8_t flags;
    strz name;
};
```

Identifies a named label, such as jump targets within machine code.

`S_LABEL32` records are always nested within [Procedures](./s_procs.md).
