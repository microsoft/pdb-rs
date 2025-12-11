# Data symbols: `S_GDATA32` (0x1007) and `S_LDATA32` (0x1008)

```c
struct Data {
    TypeIndex type;
    uint32_t offset;
    uint16_t segment;
    strz name;
};
```

`S_GDATA32` describes a global variable, usually with external visibility
across all translation units.  For example, `int g_foo;` would be described
with an `S_GDATA32` record.

`S_LDATA32` describes a variable whose lifetime duration is global, but whose
visibility is limited to a single module.

In C, `static` variables are described using `S_LDATA32`. Example:

```c
// Static variables use S_LDATA32
static int g_foo;
```

In C++, variables at global scope defined within an anonymous namespace
are described using `S_LDATA32`:

```cpp
namespace {
    int g_foo;
}
```

The `type` field describes the type of the variable, and implicitly, its size.
`offset` and `segment` describe its location in the image.
