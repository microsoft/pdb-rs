# `LF_REFSYM` (0x020c) - Referenced Symbol

# ID records

These records can only be placed in the IPI Stream. They cannot be placed in the
TPI Stream.

Value  | Name               | Description
-------|--------------------|------------
0x1601 | `LF_FUNC_ID`       | ID of a function
0x1602 | `LF_MFUNC_ID`      | ID of a member function
0x1603 | `LF_BUILDINFO`     | Describes the environment of the compiler when the module was compiled.
0x1604 | `LF_SUBSTR_LIST`   | A list of strings
0x1605 | `LF_STRING_ID`     | A string
