# `S_UNAMESPACE` (0x1124) - Using Namespace

```c
struct UsingNamespace {
    strz namespace;
};
```

This symbol is used to indicate that the compiler has added a namespace to the
lookup-scope of the lexical scope that contains this symbol. The use of this
symbol is restricted to procedures and blocks, as we felt it unreasonable to
burden the consumer side of the debugging information with having to search all
of the module's symbols to find them. Consequently, all functions that fall
under a module level `using namespace` directive will each have a `S_UNAMESPACE`
symbol record.
