# `S_OBJNAME` (0x1101) - Object Name

```c
struct ObjectName {
    uint32_t signature;
    strz name;
};
```

`signature` is a robust signature that will change every time that the module
will be compiled or different in any way. It should be at least a CRC32 based
upon module name and contents.

`name` is the full path of the object file.
