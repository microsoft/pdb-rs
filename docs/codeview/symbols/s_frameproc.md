# `S_FRAMEPROC` (0x1012) - Frame Procedure Information

Describes the stack frame layout of a [Procedure](./s_procs.md). 

The `S_FRAMEPROC` symbol is optional, for any given procedure. If any of the
`flags` are non-zero, this record should be added to the symbols for that
procedure. If present, there should be only one `S_FRAMEPROC` record for a given
procedure; it is not legal to have two or more. Also, if present, the
`S_FRAMEPROC` symbol should be nested directly within the procedure symbol, not
contained within a nested scope such as `S_BLOCK32`.

```c
struct FrameProc {
    uint32_t frame_size;
    uint32_t pad_size;
    uint32_t pad_offset;
    uint32_t save_regs_size;
    uint32_t exception_handler_offset;
    uint16_t exception_handler_segment;
    uint32_t flags;
};
```

`flags` describes various attributes of the function:

Name               | Bits | Description
-------------------|------|------------
`has_alloca`       | 0    | function uses `_alloca()`
`has_set_jmp`      | 1    | function uses `setjmp()`
`has_long_jmp`     | 2    | function uses `longjmp()`
`has_inl_asm`      | 3    | function uses inline asm
`has_eh`           | 4    | function has EH states
`inl_spec`         | 5    | function was specified as inline
`has_seh`          | 6    | function has SEH
`naked`            | 7    | function is `__declspec(naked)`
`security_checks`  | 8    | function has buffer security check
`pad`              | 9-31 | must be zero
