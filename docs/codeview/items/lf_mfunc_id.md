# `LF_MFUNC_ID` (0x1602)

Identifies a member function. This includes both static and non-static member
functions.

```c
struct MFuncId {
    TypeIndex parent_type;
    TypeIndex func_type;
    strz name;
    uint64_t decorated_name_hash;       // optional; may not be present
};
```

`parent_type` is the type of the class or struct that this member function is
defined on. This field points into the TPI. The type record that it points to
should be one of `LF_STRUCTURE`, `LF_CLASS`, `LF_UNION`, or `LF_ENUM`.
(`LF_ENUM` is used only by Rust.)

`func_type` field is the type of the member function. It may identify static
methods and instance methods. It is not known whether `LF_MFUNC_ID` is used for
constructors, destructors, or other special methods.

`name` is the undecorated name of the function, e.g. `AddRef`. For special
functions, such as constructors and conversion operations, each language has its
own conventions for how to encode those names. The following is a non-exhaustive
list of the special function names that have been observed in `LF_MFUNC_ID`
records:

* `{ctor}` - constructors
* `{dtor}` - destructors
* `operator unsigned __int64` - conversion method
* `operator new`
* `operator=`
* `operator==`
* `operator!=`
* `operator++`

`decorated_name_hash` has the same meaning as in the `LF_FUNC_ID` record,
including its interaction with padding bytes.

## Example

```
00000000 : 7f 10 00 00 a3 10 00 00 47 65 74 4e 65 78 74 45 : ........GetNextE
00000010 : 76 65 6e 74 53 6f 75 72 63 65 4f 62 6a 65 63 74 : ventSourceObject
00000020 : 49 64 00 46 48 d4 bc ac 43 c4 43 f1             : Id.FH...C.C.
```

* `parent_type` is 0x107f
* `func_type` is 0x10a3
* `name` is `GetNextEventSourceObjectId`
* `decorated_name_hash` is 0x43c443ac_bcd44846

Note the presence of the `f1` padding byte at the end of the record. It is not
part of `decorated_name_hash`.

## Example

```text
00000400 : 98 10 00 00 e8 00 00 00 1a 12 00 00 01 00 2e 00 : ................
00000410 : 02 16 7f 10 00 00 a3 10 00 00 47 65 74 4e 65 78 : ..........GetNex
00000420 : 74 45 76 65 6e 74 53 6f 75 72 63 65 4f 62 6a 65 : tEventSourceObje
00000430 : 63 74 49 64 00 46 48 d4 bc ac 43 c4 43 f1 1e 00 : ctId.FH...C.C...
```

In this example, `parent_type` is 0x107F, `func_type` is 0x10A3, `name` is
`GetNextEventSourceObjectId`, and the `decorated_name_hash` is
0x43c443ac_bcd44846. Note the presence of a single alignment padding byte (F1).
