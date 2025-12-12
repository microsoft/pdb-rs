# `LF_STRING_ID` (0x1605)

Contains a single string, and optionally a pointer to another list of
substrings.

```c
struct StringId {
  ItemId substrings;
  strz string;
};
```

The substrings field is the `ItemId` of an `LF_SUBSTR_LIST` record, or 0 if
there is none. If this field is non-zero, then this string is concatenated with
the substrings identified by the `LF_SUBSTR_LIST`.

### Example

At `ItemId` 0x10ec, we find this `LF_STRING_ID` record:

```
00001230 : 00 00 20 01 00 00 02 00 32 00 05 16 00 00 00 00 : .. .....2.......
00001240 : 44 3a 5c 64 77 2e 6d 61 69 6e 5c 2e 62 75 69 6c : D:\dw.main\.buil
00001250 : 64 5c 57 69 6e 64 6f 77 73 5c 78 36 34 5c 73 72 : d\Windows\x64\sr
00001260 : 63 5c 43 6f 6d 6d 6f 6e 00 f3 f2 f1 72 00 05 16 : c\Common....r...
```

The substrings field is zero (no substring list). The value of the string is `D:\dw.main\.build\Windows\x64\src\Common`.
