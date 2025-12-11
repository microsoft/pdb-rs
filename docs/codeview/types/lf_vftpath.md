# `LF_VFTPATH` (0x100d)

```c
struct VFTPath {
    uint32_t count;
    TypeIndex bases[count];
};
```

Describes the path to the virtual function table.

> TODO: Is this record actually used? We do not see any instances of this in
> Windows PDBs.
