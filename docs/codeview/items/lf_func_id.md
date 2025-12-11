# `LF_FUNC_ID` (0x1601)

Identifies a function. This is used for global functions, regardless of linkage
visibility. It is not used for member functions; member functions use
`LF_MFUNC_ID`.

```c
struct FuncId {
    ItemId scope;
    TypeIndex func_type;
    strz name;
    uint64_t decorated_name_hash;       // optional; may not be present
};
```

`scope` specifies the scope that contains this function definition. It is 0 for
the global scope. If it is non-zero, then it points to an `LF_STRING` record
that gives the scope. For C++, the scope is a C++ namespace. In C++, if the
scope contains nested namespaces, e.g. `namespace foo { namespace bar { ... }
}`, then the `LF_STRING` record will contain the namespaces, separated by `::`,
e.g. `foo::bar`.

`func_type` specifies the function signature type.

`name` is the undecorated name of the function, e.g. `CreateWindowExW`.

`decorated_name_hash` is a hash of the full decorated name of a function. This
field is optional; it was added as a later extension to the `LF_FUNC_ID` record.
Because symbol records are required to have a size that is a multiple of 4, and
because `LF_FUNC_ID` records contain a NUL-terminated string, it may be
necessary to insert padding bytes at the end of the record. However, we need to
be able to distinguish between padding bytes and the presence of
`decorated_name_hash`.

To do so, decode the record up to and including the `name` field. If the size of
the remaining data is at least 8 bytes, then `decorated_name_hash` is present
and should be decoded. The remainder of the record should be padded (as all
symbol records are padded) to a multiple of 4 bytes.

> TODO: clarify what hash function is used for `decorated_name_hash`.

## Examples

```
00000000 : 00 00 00 00 80 10 00 00 52 74 6c 43 61 70 74 75 : ........RtlCaptu
00000010 : 72 65 43 6f 6e 74 65 78 74 00 32 a1 6d 2c 95 ab : reContext.2.m,..
00000020 : 82 0d f2 f1                                     : ....
```

* `scope` is 0 (global)
* `type` is 0x1046
* `name` is `RtlCaptureContext`
* `decorated_name_hash` is 0x0d82ab95_2c6da132
* Note the presence of the `f2 f1` padding bytes at the end of the record. They
  are not part of `decorated_name_hash`.

```
00000000 : 06 10 00 00 78 13 00 00 43 72 65 61 74 65 43 61 : ....x...CreateCa
00000010 : 63 68 65 43 6f 6e 74 65 78 74 00 dd 56 10 27 2b : cheContext..V.'+
00000020 : 85 cd 21 f1                                     : ..!.
```

* `scope` is 0x0610 and points to an `LF_STRING_ID` record whose value is
  `DWriteCore::ApiImpl`.
* `type` is 0x1378
* `name` is `CreateCacheContext`.
* `decorated_name_hash` is 0x21cd852b_271056dd.
* Note the presence of the `f1` padding byte at the end of the record. It is not
  part of `decorated_name_hash`.
