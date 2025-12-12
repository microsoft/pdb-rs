# `S_ANNOTATION` (0x1019) - Annotation

```c
struct Annotation {
    uint32_t offset;
    uint16_t section;
    uint16_t count;
    strz strings[count];
};
```

This symbol stores annotations that point to a specific location in code
streams. This allows for analysis tools, such as debuggers, instrumentation
systems (ETW), etc. to process annotations.

`strings` contains a sequence of strings, whose count is given by `count`. PDB
does not specify how to interpret these strings.

The MSVC compiler provides an extension which allows it to insert `S_ANNOTATION`
records into module streams. This extension is invoked using the
`__annotation("format", ... args ...)` syntax.

For example, when MSVC compiles this program:

```c
int main(int argc, char** argv) {
    __annotation(L"Hello!", L"World!");
    return 0;
}
```

it produces this `S_ANNOTATION` symbol:

```
00000784 :   S_ANNOTATION: [0001:00006202]
    Hello!
    World!

00000784 :  1a 00 19 10 02 62 00 00 01 00 02 00 48 65 6c 6c : .....b......Hell
00000794 :  6f 21 00 57 6f 72 6c 64 21 00 00 00             : o!.World!...
```

Note the `segment:offset` value of `[0001:00006202]`. This shows that the
`S_ANNOTATION` points to a specific location in the code stream, even though the
annotation has no effect on code generation.
