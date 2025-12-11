
## `LF_PRECOMP` (0x1509)

This type is only present in compiler PDBs and debug information embedded in object files. It is not present in linker PDBs.

```c
struct Precomp {
    uint32_t start;
    uint32_t count;
    uint32_t signature;
    strz name;
};
```

This record specifies that the type records are included from the precompiled
types contained in another module in the executable. A module that contains this
type record is considered to be a user of the precompiled types. When emitting
to a COFF object the section name must be `.debug$P` rather than `.debug$T`. All
other attributes should be the same.

`start` is the starting type index that is included. This number must correspond
to the current type index in the current module.

`count` is the count of the number of type indices included. After including the
precompiled types, the type index must be `start + count`.

`signature` is the signature for the precompiled types being referenced by this
module. The signature will be checked against the signature in the `S_OBJNAME`
symbol record and the `LF_ENDPRECOMP` type record contained in the `.debug$T`
table of the creator of the precompiled types. The signature check is used to
detect recompilation of the supplier of the precompiled types without
recompilation of all of the users of the precompiled types. The method for
computing the signature is unspecified. It should be sufficiently robust to
detect failures to recompile.

`name` is the full path name of the module containing the precompiled types.
This name must match the module name in the `S_OBJNAME` symbol emitted by the
compiler for the object file containing the precompiled types.
