# `LF_PROCEDURE` (0x1008)

Defines the type of a function or of a static method. Instance methods use the
`LF_MFUNCTION` record.

```c
struct Procedure {
    TypeIndex return_value_type;
    uint8_t calling_convention;
    uint8_t reserved;
    uint16_t num_params;
    TypeIndex arg_list;
};
```

`return_value_type` specifies the return value type, e.g. `T_VOID` for `void`.

`calling_convention` specifies the calling convention of the procedure. See
[`CV_call`](../calling_convention.md).

`arg_list` points to an [`LF_ARGLIST`](./lf_arglist.md) record, which describes the arguments
of the procedure type.
