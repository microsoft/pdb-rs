# `LF_METHODLIST` (0x1206) - Method List

```c
struct MethodList {
    MethodEntry methods[];
};

struct MethodEntry {
    uint16_t attribute;
    uint16_t padding;
    TypeIndex type;
    uint32_t vtab_offset;     // This field is only present if 'attribute' introduces a new vtable slot
};
```
