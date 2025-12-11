## `LF_ARGLIST` (0x1201) - Argument List

```c
struct ArgList {
    uint32_t arg_count;
    TypeIndex args[arg_count];
};
```

Specifies the arguments for `LF_PROCEDURE` or `LF_MFUNCTION`.

This record should only be pointed-to by `LF_PROCEDURE` and `LF_MFUNCTION`, with
the TPI stream.
