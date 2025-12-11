# `LF_PROCEDURE` (0x1008)

```c
struct Procedure {
    TypeIndex return_value_type;
    uint8_t calling_convention;
    uint8_t reserved;
    uint16_t num_params;
    TypeIndex arg_list;
};
```

Defines the type of a function or of a static method. Instance methods use the
`LF_MFUNCTION` record.

`return_value_type` specifies the return value type, e.g. `T_VOID` for `void`.

`calling_convention` specifies the calling convention of the procedure. See
[`CV_call`](../codeview_consts.md#cv_call---function-calling-convention).
