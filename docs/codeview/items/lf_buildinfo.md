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
