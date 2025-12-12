# `S_END` (0x0006) - End of Scope

The `S_END` record terminates a nested scope. Nested scopes are created by
`S_LPROC32`, `S_GPROC32`, `S_THUNK32`, `S_INLINESITE`, etc. See
[Procedures](./s_procs.md).

The `S_END` symbol has no payload.

This symbol can appear only within module symbol streams.
