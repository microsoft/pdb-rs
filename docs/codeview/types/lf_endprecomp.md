# `LF_ENDPRECOMP` (0x0014) - End of Precompiled Types 

This record specifies that the preceding type records in this module can be
referenced by another module in the executable. A module that contains this type
record is considered to be the creator of the precompiled types. The subsection
index for the `.debug$T` segment for a precompiled types creator is emitted as
`.debug$P` instead of `.debug$T` so that the packing processing can pack the
precompiled types creators before the users.

Precompiled types must be emitted as the first type records within the
`.debug$T` segment and must be self-contained. That is, they cannot reference a
type record whose index is greater than or equal to the type index of the
`LF_ENDPRECOMP` type record.

```c
struct EndPrecompiledTypes {
    uint32_t signature;
};
```

`signature` is the signature of the precompiled types. The signatures in the
`S_OBJNAME` symbol record, the `LF_PRECOMP` type record and this signature must
match.

# Records referenced from other type records

These records define parts of types, but they do not define types. For example,
`LF_ARGLIST` specifies the argument list for a procedure, but the argument list
alone does not specify a procedure type.

None of the records described in this section should be pointed-to by records
outside of the type stream (TPI Stream).
