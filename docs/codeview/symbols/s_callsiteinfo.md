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
to a CodeView type that gives the callee's signature.

`offset` and `segment` together give the section-relative address of the call
instruction in the code stream.

`reserved` is padding and must be zero.

`type` is a [`TypeIndex`](../types/types.md) that identifies the callee's
signature. Tools use this to recover parameter and return types for call sites
where the textual disassembly shows only an indirect target such as
`call qword ptr [__imp_X]`.

The `type` field mimics the static type of the callee *expression* in source
rather than the bare function prototype, so its exact shape depends on the kind
of indirect call. The format itself only requires a `TypeIndex`; the shapes are:

| Call kind | `type` shape |
| --- | --- |
| Function-pointer or import-thunk (`__imp_*`) call | `LF_POINTER` (near, target width) -> `LF_PROCEDURE` |
| Virtual or other non-static member-function call | a bare `LF_MFUNCTION` |
| Member-function-pointer call (`(obj.*pmf)()`) | `LF_POINTER` (`PointerToMemberFunction` mode) -> `LF_MFUNCTION` |

The pointer wrapper around a function-pointer call is a near pointer whose width
The pointer wrapper around a function-pointer call is a near pointer whose width
matches the target (`Near32` on 32-bit, `Near64` on 64-bit). An `LF_MFUNCTION`
is used wherever the callee is a non-static member function because, unlike
`LF_PROCEDURE`, it records the containing class and the implicit `this`
pointer; a virtual call references that record directly (unwrapped), while a
member-function pointer references it through a member-function pointer.

For a member-function-pointer call the wrapping `LF_POINTER` is in
`PointerToMemberFunction` mode only when the member pointer uses single
inheritance. Under multiple or virtual inheritance the member pointer is a wider
structure, but the value actually invoked at the call site is still just a code
address, so the wrapper is a plain near `LF_POINTER` (`Pointer` mode) to the
`LF_MFUNCTION` instead.

Consumers should therefore be prepared to follow an `LF_POINTER` of any mode to
the underlying `LF_PROCEDURE` or `LF_MFUNCTION`, and to accept a bare
`LF_PROCEDURE` or `LF_MFUNCTION` directly.

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
