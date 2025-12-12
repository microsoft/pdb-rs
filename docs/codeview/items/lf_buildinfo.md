# `LF_BUILDINFO` (0x1603)

Describes the environment and arguments to an invocation of a tool or compiler.

```c
struct BuildInfo {
    ItemId cwd;
    ItemId build_tool;
    ItemId source_file;
    ItemId pdb_file;
    ItemId args;
};
```

Unlike most records, this record can be truncated after any field. The record
can also be seen as a single array of type `ItemId`, with meanings assigned to
each fixed array index.

Each field in `BuildInfo` is an `ItemId` that refers to an `LF_STRING_ID` or
`LF_SUBSTR_LIST` record. See `LF_SUBSTR_LIST` for details on how string records
are concatenated to form whole strings.

Each module stream may contain at most one
[`S_BUILDINFO`](../symbols/symbols.md#s_buildinfo-0x114c---build-info) record. If present,
the `S_BUILDINFO` contains the `ItemId` that points to the `LF_BUILDINFO` record
in the IPI Stream. This is the only way to associate a module with an
`LF_BUILDINFO` record.

* `cwd` - The current directory when the tool ran.

* `build_tool` - The path to the tool executable, e.g. `d:\...\cl.exe`.

* `source_file` - The primary source file that was passed to the tool. For
  C/C++, this is usually the source file that was passed on the command-line to
  the compiler. For Rust, this is the path to the root module source file.

* `pdb_file` - The path to the compiler PDB (not linker PDB), if applicable. For
  MSVC, this will only be non-empty if the compiler was invoked with `/Zi` or
  `/ZI`. See: [Debug Information
  Format](https://learn.microsoft.com/en-us/cpp/build/reference/z7-zi-zi-debug-information-format)

* `args` - Command-line arguments that were passed to the tool.

> TODO: It appears that MSVC replaces response file arguments (e.g.
> `@d:\foo\args.rsp`) with their contents, when generating the string records
> that `LF_BUILDINFO` points to. However, we should confirm (or disprove) this.

Fields in `LF_BUILDINFO` may be absent entirely (because the structure is too
small to contain the field), may have a value of 0, or may point to an empty
`LF_STRING_ID` record. Decoders should make very few assumptions about the
information in this record.

## Example

```
00000000 : 05 00 4c 41 00 00 ed 10 00 00 4d 41 00 00 4e 41 : ..LA......MA..NA
00000010 : 00 00 54 41 00 00 f2 f1                         : ..TA....
```

* `cwd` = `LF_STRING_ID` : `D:\\dw.main\\.build\\Windows\\x64\\src\\Binding\\FontBindingShared`
* `tool` = `LF_STRING_ID` : `C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\bin\\HostX64\\x64\\CL.exe`
* `source_file` = `LF_STRING_ID` : `D:\\dw.main\\src\\Binding\\FontBindingShared\\FontBindingShared.cpp`
* `pdb` = `LF_STRING_ID` : `D:\\dw.main\\.build\\Windows\\x64\\src\\Binding\\FontBindingShared\\Debug\\FontBinding.pdb`
* `args` = `LF_STRING_ID` : ` -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\cppwinrt\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\NETFXSDK\\4.8\\Include\\um\" -X`

## Example

First, we find an `LF_BUILDINFO` record. The record begins at offset 0x1d82.

```text
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

To decode the strings, we subtract the `type_index_begin` value from the IPI
Stream Header from the string ID, then count records from the start of the IPI
record stream. The `type_index_begin` value is typically 0x1000; no other value
has been observed.

For `cwd` at `ItemID` 0x10ec, we find this `LF_STRING_ID` record:

```
00001230 : 00 00 20 01 00 00 02 00 32 00 05 16 00 00 00 00 : .. .....2.......
00001240 : 44 3a 5c 64 77 2e 6d 61 69 6e 5c 2e 62 75 69 6c : D:\dw.main\.buil
00001250 : 64 5c 57 69 6e 64 6f 77 73 5c 78 36 34 5c 73 72 : d\Windows\x64\sr
00001260 : 63 5c 43 6f 6d 6d 6f 6e 00 f3 f2 f1 72 00 05 16 : c\Common....r...
```

Note that the record offset (0x1238) cannot be directly computed from the
`ItemID` (0x10ec). It is necessary to sequentially scan records from the
beginning of the IPI Stream. Note also that `ItemId` values are subject to the
same "backward-pointing" constraint as `TypeIndex` values in the TPI; an
`ItemId` in a record `R` must point to an `ItemId` that is numerically less than
the ItemId of `R` itself.

This gives the current working directory of the tool when it was executed, which
is `D:\dw.main\.build\Windows\x64\src\Common` in this example.

Let's also examine the args `ItemId`, whose value is 0x10FC. In this case,
0x10FC points to this `LF_STRING_ID` record:

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

The command-line arguments many C/C++ compiler invocations is quite long. In
this case, the `LF_STRING_ID` field has a non-zero "substring pointer" field,
which points to 0x10FB. This is the 0x10FB record, which starts at 0x1CAE:

```
00001ca0 : 34 31 2e 30 5c 77 69 6e 72 74 22 00 f2 f1 2a 00 : 41.0\winrt"...*.
00001cb0 : 04 16 09 00 00 00 f1 10 00 00 f2 10 00 00 f3 10 : ................
00001cc0 : 00 00 f4 10 00 00 f6 10 00 00 f7 10 00 00 f8 10 : ................
00001cd0 : 00 00 f9 10 00 00 fa 10 00 00 a6 00 05 16 fb 10 : ................
```

This record is `LF_SUBSTR_LIST` (0x1604), and contains a list of substrings. To
reconstruct the full command-line string, it is necessary to traverse these
substring pointers.
