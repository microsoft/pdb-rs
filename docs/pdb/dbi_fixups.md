# DBI Subsection: Fixups

The DBI stream may contain an Optional Debug Substream containing fixups.
See [dbi_sections.md] for locating this substream.

This substream contains fixup records described by this structure:

```
struct Fixup {
    uint16_t fixup_type;
    uint16_t fixup_extra;
    uint32_t rva;
    uint32_t rva_target;
}
```

# References

See `pdbdump.cpp` sources.

