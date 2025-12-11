# `LF_VFTPATH` (0x100d)

Describes the path to the virtual function table.

```c
struct VFTPath {
    uint32_t count;
    TypeIndex bases[count];
};
```

> TODO: Is this record actually used? We do not see any instances of this in
> Windows PDBs.
