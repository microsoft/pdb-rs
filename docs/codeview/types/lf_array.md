## `LF_ARRAY` (0x1503)

```c
struct Array {
    TypeIndex element_type;
    TypeIndex index_type;
    Number length;
    strz name;
};
```

Specifies an array type. The array type has a fixed size.

`element_type` is the type of the element, e.g. `int` for `int FOO[100]`.

`index_type` is the type of index used. TODO: what is this used for?

`length` is the size of the array.

`name` is the name of the array type. This is often an empty string.
