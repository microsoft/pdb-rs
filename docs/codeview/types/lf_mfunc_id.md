## `LF_MFUNC_ID` (0x1602)

```c
struct MFuncId {
    TypeIndex parent_type;
    TypeIndex func_type;
    strz name;
    uint64_t decorated_name_hash;       // optional; may not be present
};
```

Identifies a member function. This includes both static and non-static member
functions.

`parent_type` specifies the type (`LF_CLASS`, `LF_STRUCTURE`, etc.) that defines
the member function.

`func_type` specifies the function signature type.

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

### Example

```
00000000 : 7f 10 00 00 a3 10 00 00 47 65 74 4e 65 78 74 45 : ........GetNextE
00000010 : 76 65 6e 74 53 6f 75 72 63 65 4f 62 6a 65 63 74 : ventSourceObject
00000020 : 49 64 00 46 48 d4 bc ac 43 c4 43 f1             : Id.FH...C.C.
```

* `parent_type` is 0x107f
* `func_type` is 0x10a3
* `name` is `GetNextEventSourceObjectId`
* `decorated_name_hash` is 0x43c443ac_bcd44846
* Note the presence of the `f1` padding byte at the end of the record. It is not
  part of `decorated_name_hash`.
