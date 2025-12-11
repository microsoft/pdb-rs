## `S_TRAMPOLINE` (0x112c) - Trampoline

```c
struct Trampoline {
    uint16_t trampoline_kind;
    uint16_t thunk_size;
    uint32_t thunk_offset;
    uint32_t target_offset;
    uint16_t thunk_section;
    uint16_t target_section;
};
```

This symbol is emitted only by a linker to indicate a fairly simple and short,
light-weight thunk to the debugger. It was introduced due to the more complex
code requirements of the RISC platforms whereas on x86, a thunk typically
doesn't need any more code that a single instruction where it is simple to
decode the destination. These are typically used when the debugger is expected
to step through the thunk to the other side. Hence, there is a need for the
target information in the debug symbols to locate the target in a machine
independent manner.

`trampoline_kind` is 0 for linker incremental thunks, and 1 for linker
branch-island thunks.

`thunk_size` is the size of the thunk's code.
