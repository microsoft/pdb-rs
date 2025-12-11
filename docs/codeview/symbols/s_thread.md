
## `S_LTHREAD32` (0x1112) and `S_GTHREAD32` (0x1113) - Thread Storage

```c
struct ThreadStorage {
    TypeIndex type;
    uint32_t offset;
    uint16_t segment;
    strz name;
};
```

These symbols are used for data declared with the `__declspec(thread)` or
`thread_static` storage attribute. The `S_LTHREAD32` symbol is used for
variables that are local to a module (no external linkage) and `S_GTHREAD32` is
used for variables that have external linkage.

These symbols can appear in both module symbol streams and global symbol
streams. They are never nested within another symbol scope.
