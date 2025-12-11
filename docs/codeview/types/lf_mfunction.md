# `LF_MFUNCTION` (0x1009)

```c
struct MFunction {
    TypeIndex return_value_type;
    TypeIndex class;
    TypeIndex this;
    uint8_t calling_convention;
    uint8_t reserved;
    uint16_t num_params;
    TypeIndex arg_list;
    uint32_t this_adjust;
};
```

Defines the type of a non-static instance method. The `return_value_type`, `calling_convention`, `num_params`, and `arg_list` values have the same meaning as the fields with same name defined on `LF_PROCEDURE`.

The `this` field specifies the type of the implicit `this` argument. The `arg_list` _does not_ include an argument for the implicit `this` argument. The `num_params` field _does not_ count the implicit `this` field.

The `class` field specifies the type of the class that defines the method.

> TODO: clarify `this_adjust`
