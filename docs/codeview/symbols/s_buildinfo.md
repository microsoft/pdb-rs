# `S_BUILDINFO` (0x114c) - Build Info

```c
struct BuildInfoSym {
    ItemId id;
};
```

This record associates the current module with an
[`LF_BUILDINFO`](type_records.md#lf_buildinfo-0x1603) record in the IPI Stream.
The `BuildInfoSym` record does not directly contain the build information; use
`id` to look up the corresponding record in the IPI Stream.

