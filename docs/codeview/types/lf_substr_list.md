# `LF_SUBSTR_LIST` (0x1604)

```c
struct SubStrList {
    ItemId substrs[];
};
```

Contains a list of `ItemId` values that point to `LF_STRING_ID` records. The
items in the `substr` list should be dereferenced and concatenated into one
large string, in the order implied by `substr`. This is similar to a
[Rope](https://en.wikipedia.org/wiki/Rope_(data_structure)).

The boundaries between the items in `substr` do not have any meaning. The
divisions are simply necessary in order to keep large strings from overflowing
the size limitations of the symbol record format.

## Example

```
00000000 : 09 00 00 00 f1 10 00 00 f2 10 00 00 f3 10 00 00 : ................
00000010 : f4 10 00 00 f6 10 00 00 f7 10 00 00 f8 10 00 00 : ................
00000020 : f9 10 00 00 fa 10 00 00                         : ........
```

This record contains 9 `ItemId` values, all of which point to `LF_STRING_ID` records. They are listed below:

* `-c -ID:\\dw.main\\Inc -ID:\\dw.main\\Inc\\public -ID:\\dw.main\\Inc\\internal -ID:\\dw.main\\src -Zi -nologo -W3 -WX- -diagnostics:column -Od -Ob0 -D_MBCS -DWIN32 -D_WINDOWS -DDWRITE_SUBSET_MIN=0 -DDWRITE_SUBSET_CORE=1 -DDWRITE_SUBSET=1 -DDWRITE_TARGET_WINDOWS=1`
* ` -DCMAKE_INTDIR=\\\"Debug\\\" -Gm- -EHs -EHc -RTC1 -MTd -GS -guard:cf -fp:precise -Qspectre -Zc:wchar_t- -Zc:forScope -Zc:inline -GR- -std:c++17 -permissive- -YuD:/dw.main/.build/Windows/x64/src/Common/CMakeFiles/Common.dir/Debug/cmake_pch.hxx`
* ` -FpD:\\dw.main\\.build\\Windows\\x64\\src\\Common\\Common.dir\\Debug\\cmake_pch.pch -external:W3 -Gz -TP -FID:/dw.main/.build/Windows/x64/src/Common/CMakeFiles/Common.dir/Debug/cmake_pch.hxx -errorreport:queue -validate-charset -I\"C:\\Program`
* ` Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\include\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\atlmfc\\include\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS`
* `\\include\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\ucrt\" -I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS\\UnitTest\\include\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\um\"`
* ` -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\shared\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\winrt\" -I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\cppwinrt\" -I\"C:\\Program`
* ` Files (x86)\\Windows Kits\\NETFXSDK\\4.8\\Include\\um\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\include\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.36.32532\\atlmfc\\in`
* `clude\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliary\\VS\\include\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\ucrt\" -external:I\"C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Auxiliar`
* `y\\VS\\UnitTest\\include\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\um\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\shared\" -external:I\"C:\\Program Files (x86)\\Windows Kits\\10\\Include\\10.0.19041.0\\winrt\"`

As you can see, command-line arguments are split among different `LF_STRING_ID`
records.

> TODO: It is not known whether `LF_SUBSTR_ID` can point to yet more `LF_SUBSTR_ID` records, forming a tree. This has not been observed in Windows PDBs.
