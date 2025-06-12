# DBI Sections

The DBI Section Map Substream and DBI Section Contributions Substream together
form the DBI Sections data structure. DBI Sections describes _contributions_,
which are fragments of object files that contribute to the executable.

# DBI Section Map Substream

The DBI Section Map Substream describes the number of physical segments and
logical segments within a binary and provides an array that describes these
segments.

```
// sizeof = dynamic
struct SectionMap {
    uint16_t num_segments;
    uint16_t num_logical_segments;
    SectionMapEntry entries[];
}

// sizeof = 20
struct SectionMapEntry {
    uint16_t flags;
    uint16_t overlay;
    uint16_t group;
    uint16_t frame;
    uint16_t section_name;
    uint16_t class_name;
    uint32_t offset;
    uint32_t section_length;
}
```

The Section Map Substream begins with a small fixed-size header, which specifies
the number of physical segments and logical segments. The remainder of the
section map is an array of `SectionMapEntry` records.

The `section_name` and `class_name` fields are byte offsets into a name table,
but it is not clear what that name table is. In all observed PDBs, these fields
are 0xFFFF.

## Alignment

> Invariant: The stream offset of the DBI Section Map is a multiple of 4.

The stream offset of the DBI Section Map is computed by
`module_info_size + section_contributions_size` and both `module_info_size` and
`section_contributions_size` also have an invariants that they are multiples of
4, so this invariant should always be met.

> The size of the DBI Section Map (`section_map_size`) is a multiple of 4.

## Determinism

The DBI Section Map Substream appears to already be deterministic. Nothing needs
to be done.

There do not appear to be pointers from the DBI Section Map into other tables.

Many other tables (such as the DBI Section Contributions Substream and DBI
Modules Substream) refer to sections by index, and so they point into the DBI
Section Map. However, since we do not need to reorder the section table, we do
not need to update section indexes in other tables.

# DBI Section Contributions Substream

The DBI Section Contributions Substream lists the contributions (code and data)
of each module to the executable. It is one of the largest substreams of the DBI
Stream. Its size typically accounts for 50% to 70% of the size of the DBI
Stream.

The DBI Section Contributions Substream consists of a fixed-size header and an
array of fixed-size "contribution" structures which describe the section
contributions. The definition and size of the contribution structures can vary,
depending on the version field in the header.

> Invariant: The stream offset of the DBI Section Contributions Substream is a
> multiple of 4.

> Invariant: The size of the DBI Section Contributions Substream is a multiple
> of 4. The size may be 0, indicating an empty substream.

```
struct SectionContribSubstreamHeader {
    uint32_t version;
}
```

`version` specifies the version of this structure:

Name            | Value expression       | Value
----------------|------------------------|------------
`DBISCImpvV60`  | 0xeffe_0000 + 19970605 | 0xf12e_ba2d
`DBISCImpv2`    | 0xeffe_0000 + 20140516 | 0xf131_51e4

## Version `DBISCImpvV60`

All observed PDBs have used the `DBISCImpvV60` version. It uses `SectionContrib` for the records that follow `SectionContribSubstreamHeader`:

```
struct SectionContribSubstream {
    uint32_t version;
    SectionContrib contribs[];      // when version == DBISCImpvV60
}

// sizeof = 28
struct SectionContrib {
    uint16_t section;
    uint16_t padding1;
    int32_t offset;
    int32_t size;
    uint32_t characteristics;
    uint16_t module_index;
    uint16_t padding2;
    uint32_t data_crc;
    uint32_t reloc_crc;
}
```

## Version `DBISCImpV2`

This version appears not to be used; no PDBs have been found that use
`DBISCImpV2`. It uses `SectionContrib2` for the records that follow
`SectionContribSubstreamHeader`:

```
struct SectionContribSubstream {
    uint32_t version;
    SectionContrib2 contribs[];     // when version == DBISCImpvV2
}

// sizeof = 32
struct SectionContrib2 {
    uint16_t section;
    uint16_t padding1;
    int32_t offset;
    int32_t size;
    uint32_t characteristics;
    uint16_t module_index;
    uint16_t padding2;
    uint32_t data_crc;
    uint32_t reloc_crc;
    uint32_t coff_section;      // new field
}
```

## Contribution records

Clearly, most of the fields in `SectionContrib` and `SectionContrib2` are
shared; only the additional `coff_section` field is different.

> Invariant: The `contribs` array is sorted by `(section, offset)`, in
> increasing order.

This invariant allows binary searching of the contributions. Contributions
_should_ be unique; no two records should have the same `(section, offset)`
values.

> Determinism: Set `padding1` and `padding2` to zero.

`module_index` is the zero-based index of the module for this section
contribution. If `SectionContrib` is embedded in a `ModuleInfo` record (in the
DBI Modules Substream), and the module does not have any section contributions,
then the `module_index` field can be 0xffff. If `SectionContrib` is embedded in
the DBI Section Contributions Substream, then the `module_index` cannot be
0xffff, and must be a valid zero-based module index.

## Determinism

The records in the Section Contributions Substream are already sorted and the
sorting key is stable (is determined by information in the corresponding
executable). Therefore, we do not need to sort the records.
