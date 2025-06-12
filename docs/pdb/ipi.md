- [IPI Stream (Fixed Stream 4)](#ipi-stream-fixed-stream-4)
  - [`ItemId`](#itemid)
  - [IPI Hash Value Substream and IPI Hash Stream](#ipi-hash-value-substream-and-ipi-hash-stream)
- [Records](#records)
  - [`LF_FUNC_ID` (0x1601) - Function Id](#lf_func_id-0x1601---function-id)
    - [Example](#example)
  - [`LF_MFUNC_ID` (0x1602) - Member Function Id](#lf_mfunc_id-0x1602---member-function-id)
    - [Example](#example-1)
  - [`LF_BUILDINFO` (0x1603) - Build Info](#lf_buildinfo-0x1603---build-info)
    - [Example](#example-2)
  - [`LF_SUBSTR_LIST` (0x1604) - Substring List](#lf_substr_list-0x1604---substring-list)
  - [`LF_STRING_ID` (0x1605) - String](#lf_string_id-0x1605---string)
    - [Example](#example-3)
  - [`LF_UDT_SRC_LINE` (0x1606) - UDT Source Line](#lf_udt_src_line-0x1606---udt-source-line)
  - [`LF_UDT_MOD_SRC_LINE` (0x1607) - UDT Module and Source Line](#lf_udt_mod_src_line-0x1607---udt-module-and-source-line)

# IPI Stream (Fixed Stream 4)

The IPI Stream uses many of the same data structures as the TPI Stream, and the [TPI Stream](tpi_stream.md) specification should serve as the specification for the IPI Stream. However, The IPI Stream and TPI Stream store different kinds records and serve different purposes.

These aspects of the TPI Stream and IPI Stream are identical:

* Stream header
* Record framing, but not record contents
* Hash value substream
* Hash stream

The records in the IPI Stream store function ids (`LF_FUNC_ID` and `LF_MFUNC_ID`), information about the build environment of tools (`LF_BUILDINFO`), and the source locations of user-defined types (`LF_UDT_SRC_LINE` and `LF_UDT_MOD_SRC_LINE`).

Only the following records kinds can be stored within the IPI. These record kinds cannot be stored in the TPI Stream.

Kind (Hex) | Kind Name               | Description
-----------|-------------------------|------------
1601       | `LF_FUNC_ID`            | Describes a function at global or namespace scope
1602       | `LF_MFUNC_ID`           | Describes a member function defined on a class, struct, etc.
1603       | `LF_BUILDINFO`          | Describes the environment of a tool (compiler) when it was executed
1604       | `LF_SUBSTR_LIST`        | Composes a list of substring IDs into a virtual string (rope)
1605       | `LF_STRING_ID`          | A string value, with optional pointer to a substring list.
1606       | `LF_UDT_SRC_LINE`       | Gives the source location of a UDT.
1607       | `LF_UDT_MOD_SRC_LINE`   | Gives the source location of a UDT, with a module identifier.

In the definitions of records found in the IPI Stream, `TypeIndex` refers to records in the TPI Stream, not the IPI Stream.

## `ItemId`

The `ItemId` type is an alias for `uint32_t`, and it identifies a record within the IPI Stream. The value zero is reserved for a nil `ItemId`, meaning nil points to no record at all.  All other `ItemId` values must be within the range of `type_index_begin` (inclusive lower bound) to `type_index_end` (exclusive upper bound), which is specified in the IPI Stream Header.

To find a specific record in the IPI given its `ItemId`, first subtract `type_index_begin` from the `ItemId`. This gives the 0-based index of the record within the stream; let this be the value `R`.  Then, begin decoding records within the IPI Stream, counting them as they are decoded. When `R` records have been decoded, the next record is the desired record.

The value of `type_index_begin` (in the IPI Stream Header) is typically 0x1000. No other value has been observed.

## IPI Hash Value Substream and IPI Hash Stream

The IPI Stream contains an IPI Hash Value Substream, which has the same structure as the TPI Hash Value Substream.

The IPI Stream also has a corresponding IPI Hash Stream, which has the same structure as the TPI Hash Stream but it describes records in the IPI Stream, not TPI Stream.

# Records

## `LF_FUNC_ID` (0x1601) - Function Id

```
struct FuncId {
  ItemId scope;
  TypeIndex func_type;
  strz name;
  uint64_t decorated_name_hash;
};
```

This record identifies a function. The function is defined either at global scope (not within a namespace) or within a namespace. `LF_FUNC_ID` is not used for member functions of classes and structs; for that, `LF_MFUNC_ID` is used. 

`scope` identifies the containing scope of this function. If `scope` is non-zero, then the function is nested within a namespace, identified by `scope`. The record that `scope` points to should be an `LF_STRING_ID` record. If `scope` is zero, then the function is global (is not within a namespace).

`func_type` gives the type of the function signature. This points into the TPI.

`name` is the short name of the function, e.g. `CreateWindowExW`. The name is not decorated.

The `decorated_name_hash` field appears to have been added later. It is optional; if it is not present, then the record will end after the name field. However, because IPI records are alignment-padded, there may be alignment bytes after the name field. Decoders should be careful not to confuse the presence of any alignment bytes with the presence of the decorated_name_hash field. They can be distinguished by counting the number of remaining bytes. Since the alignment requirement of the IPI record format is 2 bytes, the maximum number of alignment bytes is 1. The size of the decorated_name_hash is 8 bytes, so if the remaining bytes after name is greater than or equal to 8, then the decorated_name_hash field is present.

> Purpose unclear: It is not clear why `LF_FUNC_ID` exists, or what uses these records.

### Example

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

* The first `LF_FUNC_ID` record begins at offset 0x4c and has `scope == 0`, `name == "DWriteCoreCreateFactory"`. The `decorated_name_hash` is 0x8E2F26FF_14801804. The record ends on natural alignment, so there are no alignment padding bytes.

* Between the first `LF_FUNC_ID` record and the second is an `LF_UDT_MOD_SRC_LINE` record, which is not described here.

* The second `LF_FUNC_ID` record begins at 0x8a. The `scope` is 0, func_type is 0x1038 name is CreateBinding. The decorated_name_hash is 0x15d4cc64_8b6183b4.  This record has two alignment padding bytes (F2 F1).

* The third `LF_FUNC_ID` record begins at 0xae. The `scope` is 0, `func_type` is 0x1038, name is CreateRestrictedBinding. The decorated_name_hash is 0xf6a57eba_183f71e3. The record ends on natural alignment, so there is no padding.

It is not understood why some records have alignment padding bytes after the `decorated_name_hash` but others do not. It may be that record payload (excluding the record length field but including the record kind field) is padded to a length that is a multiple of 4, for `LF_FUNC_ID` records. Note that the `decorated_name_hash` field does not begin on an alignment boundary because it immediately follows a NUL-terminated UTF-8 string.

## `LF_MFUNC_ID` (0x1602) - Member Function Id

```
struct MFuncId {
  TypeIndex parent_type;
  TypeIndex func_type;
  strz name;
  uint64_t decorated_name_hash;
};
```

The `LF_MFUNC_ID` record is similar to `LF_FUNC_ID`. `LF_MFUNC_ID` is used for member functions defined on classes and structs.

The `parent_type` field is the type of the class or struct that this member function is defined on. This field points into the TPI. The type record that it points to should be one of `LF_STRUCTURE`, `LF_CLASS`, `LF_UNION`, or `LF_ENUM`. (`LF_ENUM` is used only by Rust.) 

The `func_type` field is the type of the member function. It may identify static methods and instance methods. It is not known whether `LF_MFUNC_ID` is used for constructors, destructors, or other special methods.

The `name` field is the simple name of the method, e.g. `AddRef`, and is not decorated.

The `decorated_name_hash` has the same structure and meaning as in the `LF_FUNC_ID` record. It is optional.

### Example

```
00000400 : 98 10 00 00 e8 00 00 00 1a 12 00 00 01 00 2e 00 : ................
00000410 : 02 16 7f 10 00 00 a3 10 00 00 47 65 74 4e 65 78 : ..........GetNex
00000420 : 74 45 76 65 6e 74 53 6f 75 72 63 65 4f 62 6a 65 : tEventSourceObje
00000430 : 63 74 49 64 00 46 48 d4 bc ac 43 c4 43 f1 1e 00 : ctId.FH...C.C...
```

In this example, `parent_type` is 0x107F, `func_type` is 0x10A3, `name` is `GetNextEventSourceObjectId`, and the `decorated_name_hash` is 0x43c443ac_bcd44846. Note the presence of a single alignment padding byte (F1).

## `LF_BUILDINFO` (0x1603) - Build Info

```
struct BuildInfo {
  uint16_t num_strings;
  ItemId strings[num_strings];
};
```

This record provides information about the environment of the tools (compilers and linkers) that produced this executable.

The fixed-length prefix of this record consists only of the `num_strings` field. This field specifies the number of string IDs in this `BuildInfo`. Each string ID points to a record in the IPI or is 0 meaning "absent".

The index of each string ID in the `strings[]` array defines the meaning of that string. These are the currently-known string meanings:

Index | Name     | Usage
------|----------|------
0     | `cwd`    | Current directory of the tool, e.g. `d:\my_repo\src`.
1     | `tool`   | The tool which was executed, e.g. `cl.exe`.
2     | `source` | Source file that was compiled. For C/C++, this is the primary translation unit source file, e.g. `d:\my_repo\src\main.cpp`.
3     | `pdb`    | PDB file. The meaning of this field is not clear. This may be the "compiler" PDB, rather than the linker PDB.
4     | `args`   | Command arguments that were passed to the tool, e.g. `-Id:\sdk\include -Id:\zlib\include ...`.

The value of each string ID is a pointer to an `LF_STRING_ID` or `LF_SUBSTR_LIST` record in the IPI.

### Example

First, we find an `LF_BUILDINFO` record. The record begins at offset 0x1d82.

```
00001d80 : 00 f1 1a 00 03 16 05 00 ec 10 00 00 ed 10 00 00 : ................
00001d90 : ef 10 00 00 f0 10 00 00 fc 10 00 00 f2 f1 1a 00 : ................
00001da0 : 03 16 05 00 ec 10 00 00 ed 10 00 00 ee 10 00 00 : ................
```

The `num_strings` field is 5. Each of the string IDs decodes as follows:

Index | Name     | Value
------|----------|------
0     | `cwd`    | 0x10ec
1     | `tool`   | 0x10ed
2     | `source` | 0x10ef
3     | `pdb`    | 0x10f0
4     | `args`   | 0x10fc

The record ends with two alignment padding bytes (F2 F1).

To decode the strings, we subtract the `type_index_begin` value from the IPI Stream Header from the string ID, then count records from the start of the IPI record stream. The `type_index_begin` value is typically 0x1000; no other value has been observed.

For `cwd` at `ItemID` 0x10ec, we find this `LF_STRING_ID` record:

```
00001230 : 00 00 20 01 00 00 02 00 32 00 05 16 00 00 00 00 : .. .....2.......
00001240 : 44 3a 5c 64 77 2e 6d 61 69 6e 5c 2e 62 75 69 6c : D:\dw.main\.buil
00001250 : 64 5c 57 69 6e 64 6f 77 73 5c 78 36 34 5c 73 72 : d\Windows\x64\sr
00001260 : 63 5c 43 6f 6d 6d 6f 6e 00 f3 f2 f1 72 00 05 16 : c\Common....r...
```

Note that the record offset (0x1238) cannot be directly computed from the `ItemID` (0x10ec). It is necessary to sequentially scan records from the beginning of the IPI Stream. Note also that `ItemId` values are subject to the same "backward-pointing" constraint as `TypeIndex` values in the TPI; an `ItemId` in a record `R` must point to an `ItemId` that is numerically less than the ItemId of `R` itself.

This gives the current working directory of the tool when it was executed, which is `D:\dw.main\.build\Windows\x64\src\Common` in this example.

Let's also examine the args `ItemId`, whose value is 0x10FC. In this case, 0x10FC points to this `LF_STRING_ID` record:

```
00001cd0 : 00 00 f9 10 00 00 fa 10 00 00 a6 00 05 16 fb 10 : ................
00001ce0 : 00 00 20 2d 65 78 74 65 72 6e 61 6c 3a 49 22 43 : .. -external:I"C
00001cf0 : 3a 5c 50 72 6f 67 72 61 6d 20 46 69 6c 65 73 20 : :\Program Files 
00001d00 : 28 78 38 36 29 5c 57 69 6e 64 6f 77 73 20 4b 69 : (x86)\Windows Ki
00001d10 : 74 73 5c 31 30 5c 49 6e 63 6c 75 64 65 5c 31 30 : ts\10\Include\10
00001d20 : 2e 30 2e 31 39 30 34 31 2e 30 5c 63 70 70 77 69 : .0.19041.0\cppwi
00001d30 : 6e 72 74 22 20 2d 65 78 74 65 72 6e 61 6c 3a 49 : nrt" -external:I
00001d40 : 22 43 3a 5c 50 72 6f 67 72 61 6d 20 46 69 6c 65 : "C:\Program File
00001d50 : 73 20 28 78 38 36 29 5c 57 69 6e 64 6f 77 73 20 : s (x86)\Windows 
00001d60 : 4b 69 74 73 5c 4e 45 54 46 58 53 44 4b 5c 34 2e : Kits\NETFXSDK\4.
00001d70 : 38 5c 49 6e 63 6c 75 64 65 5c 75 6d 22 20 2d 58 : 8\Include\um" -X
00001d80 : 00 f1 1a 00 03 16 05 00 ec 10 00 00 ed 10 00 00 : ................
```

The command-line arguments many C/C++ compiler invocations is quite long. In this case, the `LF_STRING_ID` field has a non-zero "substring pointer" field, which points to 0x10FB. This is the 0x10FB record, which starts at 0x1CAE:

```
00001ca0 : 34 31 2e 30 5c 77 69 6e 72 74 22 00 f2 f1 2a 00 : 41.0\winrt"...*.
00001cb0 : 04 16 09 00 00 00 f1 10 00 00 f2 10 00 00 f3 10 : ................
00001cc0 : 00 00 f4 10 00 00 f6 10 00 00 f7 10 00 00 f8 10 : ................
00001cd0 : 00 00 f9 10 00 00 fa 10 00 00 a6 00 05 16 fb 10 : ................
```

This record is `LF_SUBSTR_LIST` (0x1604), and contains a list of substrings.  To reconstruct the full command-line string, it is necessary to traverse these substring pointers.
 
## `LF_SUBSTR_LIST` (0x1604) - Substring List

```
struct SubstrList {
  uint32_t num_strings;
  ItemId strings[num_strings];
};
```

The `LF_SUBSTR_LIST` record allows composing strings from a set of substrings. This is similar to the well-known [Rope](https://en.wikipedia.org/wiki/Rope_(data_structure)) data structure.

Each `ItemId` in the `strings` array points to another record in the IPI. It may point only to `LF_STRING_ID` records.

> TODO: Is this true? So far, we have only observed pointers from `LF_SUBSTR_LIST` to `LF_STRING_ID`, but can `LF_SUBSTR_LIST` also point to `LF_SUBSTR_LIST`, forming a dag?
 
## `LF_STRING_ID` (0x1605) - String

```
struct StringId {
  ItemId substrings;
  strz string;
};
```

The `LF_STRING_ID` record contains a single string, and optionally a pointer to another list of substrings.

The substrings field is the `ItemId` of an `LF_SUBSTR_LIST` record, or 0 if there is none. If this field is non-zero, then this string is concatenated with the substrings identified by the `LF_SUBSTR_LIST`.

### Example

At `ItemId` 0x10ec, we find this `LF_STRING_ID` record:

```
00001230 : 00 00 20 01 00 00 02 00 32 00 05 16 00 00 00 00 : .. .....2.......
00001240 : 44 3a 5c 64 77 2e 6d 61 69 6e 5c 2e 62 75 69 6c : D:\dw.main\.buil
00001250 : 64 5c 57 69 6e 64 6f 77 73 5c 78 36 34 5c 73 72 : d\Windows\x64\sr
00001260 : 63 5c 43 6f 6d 6d 6f 6e 00 f3 f2 f1 72 00 05 16 : c\Common....r...
```

The substrings field is zero (no substring list). The value of the string is `D:\dw.main\.build\Windows\x64\src\Common`.

## `LF_UDT_SRC_LINE` (0x1606) - UDT Source Line

```
// sizeof = 12
struct UdtSrcLine {
  TypeIndex type;
  ItemId source_file;
  uint32_t line;
};
```

Describes the source location of a user-defined type (UDT). This allows debuggers to display the source location of a struct, class, enum, etc.

The `type` field is the type that is being described.

The `source_file` field is the ItemId of an `LF_STRING` record, which gives the file name of the source file where this UDT is defined.

The `line` field is the 1-based line number within the source file where the UDT is defined.

There is some suggestion that `LF_UDT_SRC_LINE` records do not appear in linker PDBs, but instead are found in compiler PDBs, and that the linker converts `LF_UDT_SRC_LINE` records to `LF_UDT_MOD_SRC_LINE` during linking.
 
## `LF_UDT_MOD_SRC_LINE` (0x1607) - UDT Module and Source Line

The `LF_UDT_MOD_SRC_LINE` describes the source location of a user-defined type (UDT). It is identical to the `LF_MOD_SRC_LINE` record, but it adds a new field, which is a module index.

```
// sizeof = 14
struct UdtModSrcLine {
  TypeIndex type;
  ItemId source_file;
  uint32_t line;
  uint16_t module;
};
```

The meaning of the type, source_file, and line fields is identical to the same in the `LF_UDT_SRC_LINE` record.

The module field is a module index.

> Determinism: It is assumed that this is the index of the module which defined this UDT. However, there is a problem: What if the same UDT is defined in more than one module? It is not clear if there is a deterministic result. Because that information (the set of modules which defined the same UDT) is not present in the linker, we cannot replace the module index with a deterministic selection from that set. Instead, we can only set the module index to some well-known value, like 0.
