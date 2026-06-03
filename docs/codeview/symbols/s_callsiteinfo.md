# `S_CALLSITEINFO` (0x1139) - Indirect Call Site Information

```c
struct CallSiteInfo {
    uint32_t offset;
    uint16_t segment;
    uint16_t reserved;
    TypeIndex type;
};
```

Describes an indirect call site by binding the address of a call instruction
to a CodeView type that gives the callee's function-pointer signature.

`offset` and `segment` together give the section-relative address of the call
instruction in the code stream.

`reserved` is padding and must be zero.

`type` is a [`TypeIndex`](../types/types.md) that identifies the callee's
function-pointer type. In practice this is an `LF_PROCEDURE` or `LF_MFUNCTION`
record (occasionally an `LF_POINTER` that refers to one) describing the same
prototype as the function-pointer expression in source. Tools use this to
recover parameter and return types for call sites where the textual disassembly
shows only an indirect target such as `call qword ptr [__imp_X]`.

`S_CALLSITEINFO` is emitted for *indirect* call sites: function-pointer calls,
virtual calls, and calls through import thunks (`__imp_*`) which execute as
indirect calls. Direct calls do not need an `S_CALLSITEINFO` record because the
callee is named by an [`S_GPROC32`](./s_procs.md) or [`S_LPROC32`](./s_procs.md)
symbol whose own type field already supplies the signature.

Tail-call optimization does not suppress the record. A tail-called indirect
call lowers to a `jmp [reg/mem]` rather than a `call [reg/mem]`, but a record
is still emitted at the address of that `jmp` instruction.

Because compilers may tail-merge several indirect calls that share a callee
expression into a single branch, more than one `S_CALLSITEINFO` record may
share the same `segment:offset` address.

`S_CALLSITEINFO` records can only appear in module symbol streams, never in
the global symbol stream, and must be nested directly within the procedure
that contains the call site.
