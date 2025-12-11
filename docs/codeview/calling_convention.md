# Calling Convention

T
This document specifies constants (named values and enums) that are used by
CodeView.

The values in this file come from these sources:

* <https://github.com/microsoft/microsoft-pdb/blob/master/include/cvinfo.h>
* <https://github.com/microsoft/microsoft-pdb/blob/master/include/cvconst.h>

# `CV_call` - Function Calling Convention

`CV_call` specifies the calling convention of the procedure. It can take one of
the following values:

| Value | Name           | Description                                                      |
|-------|----------------|------------------------------------------------------------------|
| 0x00  | `NEAR_C`       | near right to left push, caller pops stack                      |
| 0x01  | `FAR_C`        | far right to left push, caller pops stack                       |
| 0x02  | `NEAR_PASCAL`  | near left to right push, callee pops stack                      |
| 0x03  | `FAR_PASCAL`   | far left to right push, callee pops stack                       |
| 0x04  | `NEAR_FAST`    | near left to right push with regs, callee pops stack            |
| 0x05  | `FAR_FAST`     | far left to right push with regs, callee pops stack             |
| 0x06  | `SKIPPED`      | skipped (unused) call index                                      |
| 0x07  | `NEAR_STD`     | near standard call                                               |
| 0x08  | `FAR_STD`      | far standard call                                                |
| 0x09  | `NEAR_SYS`     | near sys call                                                    |
| 0x0a  | `FAR_SYS`      | far sys call                                                     |
| 0x0b  | `THISCALL`     | this call (this passed in register)                             |
| 0x0c  | `MIPSCALL`     | Mips call                                                        |
| 0x0d  | `GENERIC`      | Generic call sequence                                            |
| 0x0e  | `ALPHACALL`    | Alpha call                                                       |
| 0x0f  | `PPCCALL`      | PPC call                                                         |
| 0x10  | `SHCALL`       | Hitachi SuperH call                                              |
| 0x11  | `ARMCALL`      | ARM call                                                         |
| 0x12  | `AM33CALL`     | AM33 call                                                        |
| 0x13  | `TRICALL`      | TriCore Call                                                     |
| 0x14  | `SH5CALL`      | Hitachi SuperH-5 call                                            |
| 0x15  | `M32RCALL`     | M32R Call                                                        |
| 0x16  | `CLRCALL`      | clr call                                                         |
| 0x17  | `INLINE`       | Marker for routines always inlined and thus lacking a convention|
| 0x18  | `NEAR_VECTOR`  | near left to right push with regs, callee pops stack            |
| 0x19  | `RESERVED`     | first unused call enumeration                                    |
