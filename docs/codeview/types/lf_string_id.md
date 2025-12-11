## `LF_STRING_ID` (0x1605)

```
struct StringId {
    ItemId id;
    strz text;
}
```

Contains one string. `LF_STRING_ID` records are pointed-to by these kinds of records:

* `LF_SUBSTR_ID`
* `LF_FUNC_ID`

Each `LF_STRING_ID` directly contains a string. Some records (such as
`LF_FUNC_ID`) simply always use `LF_STRING_ID`, while others appear to use
`LF_STRING_ID` or `LF_SUBSTR_LIST`, in the same field.

> TODO: What is the `id` field used for?

> TODO: Clarify exactly when `LF_STRING_ID` vs. `LF_SUBSTR_LIST` is used.
