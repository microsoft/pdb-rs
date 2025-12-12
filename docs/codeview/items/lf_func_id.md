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
scope contains nested namespaces, e.g. `namespace foo { namespace bar { ... }}`,
then the `LF_STRING` record will contain the namespaces, separated by `::`,
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

## Example

This example shows several `LF_FUNC_ID` records.

```
00000040 : 44 57 72 69 74 65 43 6f 72 65 00 f1 2a 00 01 16 : DWriteCore..*...
00000050 : 00 00 00 00 09 10 00 00 44 57 72 69 74 65 43 6f : ........DWriteCo
00000060 : 72 65 43 72 65 61 74 65 46 61 63 74 6f 72 79 00 : reCreateFactory.
00000070 : 04 18 80 14 ff 26 2f 8e 10 00 07 16 32 10 00 00 : .....&/.....2...
00000080 : 01 00 00 00 fe 01 00 00 01 00 22 00 01 16 00 00 : ..........".....
00000090 : 00 00 38 10 00 00 43 72 65 61 74 65 42 69 6e 64 : ..8...CreateBind
000000a0 : 69 6e 67 00 b4 83 61 8b 64 cc d4 15 f2 f1 2a 00 : ing...a.d.....*.
000000b0 : 01 16 00 00 00 00 38 10 00 00 43 72 65 61 74 65 : ......8...Create
000000c0 : 52 65 73 74 72 69 63 74 65 64 42 69 6e 64 69 6e : RestrictedBindin
000000d0 : 67 00 e3 71 3f 18 ba 7e a5 f6 10 00 07 16 3a 10 : g..q?..~......:.
000000e0 : 00 00 2b 00 00 00 b4 05 00 00 01 00 1a 00 05 16 : ..+.............
000000f0 : 00 00 00 00 44 57 72 69 74 65 43 6f 72 65 3a 3a : ....DWriteCore::
```

In this example, there are 3 LF_FUNC_ID records:

* The first `LF_FUNC_ID` record begins at offset 0x4c and has `scope == 0`,
  `name == "DWriteCoreCreateFactory"`. The `decorated_name_hash` is
  0x8E2F26FF_14801804. The record ends on natural alignment, so there are no
  alignment padding bytes.

* Between the first `LF_FUNC_ID` record and the second is an
  `LF_UDT_MOD_SRC_LINE` record, which is not described here.

* The second `LF_FUNC_ID` record begins at 0x8a. The `scope` is 0, func_type is
  0x1038 name is CreateBinding. The decorated_name_hash is 0x15d4cc64_8b6183b4.
  This record has two alignment padding bytes (F2 F1).

* The third `LF_FUNC_ID` record begins at 0xae. The `scope` is 0, `func_type` is
  0x1038, name is CreateRestrictedBinding. The decorated_name_hash is
  0xf6a57eba_183f71e3. The record ends on natural alignment, so there is no
  padding.

It is not understood why some records have alignment padding bytes after the
`decorated_name_hash` but others do not. It may be that record payload
(excluding the record length field but including the record kind field) is
padded to a length that is a multiple of 4, for `LF_FUNC_ID` records. Note that
the `decorated_name_hash` field does not begin on an alignment boundary because
it immediately follows a NUL-terminated UTF-8 string.
