## `LF_ARGLIST` (0x1201) - Argument List

```c
struct ArgList {
    uint32_t arg_count;
    TypeIndex args[arg_count];
};
```

Specifies the arguments for `LF_PROCEDURE` or `LF_MFUNCTION`.

This record should only be pointed-to by `LF_PROCEDURE` and `LF_MFUNCTION`,
within the TPI stream.

> Invariant: Each `LF_ARGLIST` record is pointed-to by exactly one
> `LF_PROCEDURE` or `LF_MFUNCTION` record, and is pointed-to by the `arg_list`
> field.

Effectively, the `LF_PROCEDURE` (or `LF_MFUNCTION`) record "owns" the
`LF_ARGLIST`.