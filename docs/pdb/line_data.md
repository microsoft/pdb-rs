# Line Data (C13)

Module Streams may contain line number information, which describes the mapping between source locations and instruction streams. The length of the C13 Line Data section is found in the Module Information structure in the DBI Modules substream, not in the corresponding Module Stream.

Older versions of MSVC use C11 Line Data. C11 Line Data is obsolete and will not be described.

The location of the C13 Line Data is specified in the Module Information record in the [DBI Modules Substream](dbi.md#dbi-modules-substream). See the `DbiModuleInfo` record. Each module may have a corresponding module stream, and thus a C13 Line Data. The `stream` field specifies the stream that contains the C13 Line Data (as well as several other substreams).  The offset of the C13 Line Data is computed by `sym_byte_size + c11_byte_size`, and the size of the C13 Line Data is given by `c13_byte_size`.

> Invariant: The offset of the Line Data within the Module Information Stream is a multiple of 4.

> Invariant: The size of the C13 Line Data is a multiple of 4.

The C13 Line Data substream contains a sequence of variable-length subsections:

```
struct Subsection {
  uint32_t subsection_kind;
  uint32_t subsection_size;
  uint8_t subsection_data[subsection_size];
  uint8_t alignment_padding[];
}
```

If bit 31 of `subsection_kind` is 1 (mask 0x8000_0000), then the subsection should be ignored.

These are the known values for `subsection_kind`:

Name                           | Value | Description
-------------------------------|-------|------------
`DEBUG_S_SYMBOLS`              |  0xF1 | 
`DEBUG_S_LINES`                |  0xF2 | Line information
`DEBUG_S_STRINGTABLE`          |  0xF3 | 
`DEBUG_S_FILECHKSMS`           |  0xF4 | File checksums
`DEBUG_S_FRAMEDATA`            |  0xF5 | 
`DEBUG_S_INLINEELINES`         |  0xF6 | 
`DEBUG_S_CROSSSCOPEIMPORTS`    |  0xF7 | 
`DEBUG_S_CROSSSCOPEEXPORTS`    |  0xF8 | 
`DEBUG_S_IL_LINES`             |  0xF9 | 
`DEBUG_S_FUNC_MDTOKEN_MAP`     |  0xFA | 
`DEBUG_S_TYPE_MDTOKEN_MAP`     |  0xFB | 
`DEBUG_S_MERGED_ASSEMBLYINPUT` |  0xFC | 
`DEBUG_S_COFF_SYMBOL_RVA`      |  0xFD | 
`DEBUG_S_INLINEE_MDTOKEN_MAP`  |  0xFE | 
`DEBUG_S_XFGHASH_TYPE`         |  0xFF | 
`DEBUG_S_XFGHASH_VIRTUAL`      | 0x100 | 

There is no defined order for the subsections; subsections may appear in any order.

> Determinism: Sort the subsections of the Line Data table. Place the `DEBUG_S_FILE_CHECKSUMS` record before all `DEBUG_S_LINES` subsections. Sort the `DEBUG_S_LINES` subsections by their contribution segment and offset, and then by their byte contents.

> Invariant: The total size in bytes of each subsection is a multiple of 4. Alignment padding bytes are added after `subsection_data` if `subsection_size` is not a multiple of 4.

This invariant guarantees 4-byte alignment for all subsections and their data.

# Subsection: DEBUG_S_LINES (0xF2)

A `DEBUG_S_LINES` subsection contains line mapping data. A C13 Line Data section may contain any number of `DEBUG_S_LINES` subsections. It describes the line number mappings for a single "contribution". The subsection data has this header:

```
struct DebugSLinesHeader {
    uint32_t contribution_offset;       // address of the instruction stream
    uint16_t contribution_section;
    uint16_t contribution_flags;
    uint32_t contribution_size;         // size in the instruction stream
}
```

The `contribution_section` and `contribution_offset` are the COFF section and offset of this contribution. The `contribution_size` field is the size in bytes of this contribution in the COFF section; it is not related to the size of the data in the `DebugSLinesHeader` record.

Immediately after `DebugSLinesHeader` is a sequence of variable-length `Block` records:

```
// sizeof = dynamic
struct Block {
    uint32_t file_offset;           // points into DEBUG_S_FILE_CHECKSUMS
    uint32_t num_lines;
    uint32_t block_size;
    LineRecord lines[num_lines];
    ColumnRecord columns[contribution_flags & HAVE_COLUMNS ? num_lines : 0];
}
```

Each `Block` describes a set of line mappings to a single file. The line mappings use byte offsets that are relative to the `contribution_offset` specified in the `DebugSLinesHeader` header.

`file_offset` is the byte offset into a File Checksums subsection. This is required even if the file is not using a checksum because the File Checksum record contains the pointer to the file name string.

> TODO: Is the previous statement true? If file checksums are not being used, is `file_offset` actually a `NameIndex` or some other reference to the DBI Sources Substream?

> Invariant: If any Block record exists in a Lines Data substream, then there must be exactly one File Checksums subsection in the Lines Data substream, and the file_offset of the Block Record must point to a valid File Checksum record.

> Invariant: `num_lines` is greater than zero. There is no purpose in a Block that describes no lines.

> Invariant: The value of `num_lines` must always specify a set of line records (and optional column records) that fits within the size of the subsection.

> Invariant: In the sequence of Block records within a `DEBUG_S_LINES` record, the `Block` `offset` field must be sorted in increasing order.

```
// sizeof = 8
struct LineRecord {
    uint32_t offset;
    uint32_t flags;
}
```

The `offset` field specifies the byte offset from the start of this contribution (in the instruction stream, not the Lines Data) for this line.

The `flags` field encodes three bit-fields:

* Bits 0-23 are `line_num_start`. This is the 1-based starting line number within the source file of this line record.
* Bits 24-30 are `delta_line_end`. It specifies a value to add to line_num_start to find the ending line. If this value is zero, then this line record encodes only a single line, not a span of lines.
* Bit 31 is the `statement` bit field. If set to 1, it indicates that this line record describes a statement.

# Columns

```
// sizeof = 4
struct ColumnRecord {
    uint16_t start_offset;
    uint16_t end_offset;
}
```

The header of `DEBUG_S_LINES` specifies whether it contains column records, by setting a bit within the `contribution_flags` field.  If the subsection contains column records, then each block will contain an array of ColumnRecord, immediately following the `LineRecord` array.  The number of elements in the ColumnRecord is num_lines, which is the same as the LineRecord array.  These two arrays are parallel arrays; entries at the same index are related.

The `start_offset` field specifies the offset in bytes from the start of a line in the source file. Similarly, `end_offset` gives the end of the region in bytes.

# Example

This is an example of a `DEBUG_S_LINES` subsection. The entire subsection is highlighted, including the subsection header.

This is the `DEBUG_S_LINES` header:
 
* `contribution_offset` is 0x000B_4DB0.
* `contribution_section` is 1.
* `contribution_flags` is 0. This means there are no Column records.
* `contribution_size` is 0x4E.

This `DEBUG_S_LINES` subsection contains a single Block.  The header of that Block is highlighted:
 
* `file_offset` is 0xE88. This is an offset into the File Checksums Subsection for this module.
* `num_lines` is 0xB (11)
* `block_size` is 0x64 (100)

As mentioned above, this `DEBUG_S_LINES` subsection does not have Column records, so we see only Line records in this block.  The Line records are highlighted:
 
Decoding these Line Records gives:

Offset (hex) | Statement? | `line_num_start`
-------------|------------|-----------------
0            | Yes        | 0x157
0x0004       | Yes        | 0x15D
0x000F       | Yes        | 0x15E
0x0013       | Yes        | 0x160
0x0018       | Yes        | 0x165
0x002E       | Yes        | 0x166
0x0032       | Yes        | 0x16A
0x003F       | Yes        | 0x171
0x0045       | Yes        | 0x172
0x0047       | Yes        | 0x167
0x0049       | Yes        | 0x173

# Subsection: DEBUG_S_FILE_CHECKSUMS (0xF4)

The `DEBUG_S_FILE_CHECKSUMS` subsection contains file checksum entries. Each entry identifies a file using an index into another table and contains the file's checksum, if any. Each entry has this structure:

```
// sizeof = dynamic
// offset within DEBUG_S_FILE_CHECKSUMS is always aligned at multiple of 4
struct FileChecksum {
    NameIndex file_name;
    uint8_t checksum_size;
    uint8_t checksum_kind;
    uint8_t checksum[checksum_size];
}
```

`file_name` is a `NameIndex` value which points into the [Names Stream](names_stream.md).

`checksum_kind` identifies the algorithm which computed the checksum.  `checksum_size` is the size in bytes of the checksum. The following table lists the known checksum algorithms, the `checksum_kind` which identifies them, and the expected value of `checksum_size`.

Algorithm | `checksum_kind`  | `checksum_size` | MSVC compiler arg
----------|------------------|-----------------|------------------
None      | 0                | 0               | (none)
MD5       | 1                | 16              | `/ZH:MD5`
SHA1      | 2                | 20              | `/ZH:SHA1`
SHA256    | 3                | 32              | `/ZH:SHA_256`

Decoders should always use `checksum_size` when reading records, rather than making assumptions about the size of `checksum` based on `checksum_kind`.

## Single instance

Within each module's C13 Line Data there should be at most one `DEBUG_S_FILE_CHECKSUMS` subsection. This is an implication of the fact that there are pointers (byte offsets) from `DEBUG_S_FILE_CHECKSUMS` into `DEBUG_S_FILE_CHECKSUMS`. If there were more than one `DEBUG_S_FILE_CHECKSUMS` subsection, then these pointers would be ambiguous.

If a C13 Line Data section contains any `DEBUG_S_LINES` subsections, then the `DEBUG_S_FILE_CHECKSUMS` subsection must be present.

## Addressing and iterating checksum records

`DEBUG_S_LINES` subsections contain byte offsets that point into the `DEBUG_S_FILE_CHECKSUMS` subsection. These pointers are the `file_offset` field within each `Block` record. Because of this, it is required that these `file_offset` values point to valid locations within the `DEBUG_S_FILE_CHECKSUMS` table. The pointer should point to a complete `FileChecksum` record. 

> Invariant: Each `Block.file_offset` pointer points to a valid location within a `DEBUG_S_FILE_CHECKSUMS` subsection. The `FileChecksum` record must be completely within the bounds of the subsection; it cannot overlap the end of the subsection or start outside of it.

> Invariant: `FileChecksum` records must not overlap (share bytes).  For two `Block.file_offset` values `f1` and `f2`, if `f1 != f2`, then the `FileChecksum` records for `f1` and `f2` must not overlap (must not share any bytes in common).

> Invariant: `FileChecksum` records within the `DEBUG_S_FILE_CHECKSUMS` subsection must not have any unused bytes between them. This does not apply to alignment bytes at the end of the subsection.

> Invariant: The size of the `DEBUG_S_FILE_CHECKSUMS` subsection must be a multiple of 4. If necessary, padding bytes are added to the end of the subsection.

> Determinism: If padding bytes are present at the end of `DEBUG_S_FILE_CHECKSUMS`, then they must have a value of zero.

## Alignment

> Invariant: The offset of each `FileChecksum` record within the `DEBUG_S_FILE_CHECKSUMS` section is a multiple of 4.

Most `FileChecksum` records will not naturally have a size that is a multiple of 4 because the fixed-size portion of the record has a length of 6 and all of the current checksum sizes have lengths that are a multiple of 4.  Decoders must skip padding bytes after each `FileChecksum` so that the current decoding position (relative to the start of `DEBUG_S_FILE_CHECKSUMS`) is a multiple of 4.  Encoders must insert these padding bytes.

> Determinism: Set the padding bytes to zero.

## Correlation of `FileChecksum` and DBI Sources Substream

There is an unusual implementation detail, which associates the order of `FileChecksum` entries with the entries of source code locations within the [DBI Sources Substream](dbi.md#dbi-sources-substream).

# References

* <https://github.com/microsoft/microsoft-pdb/blob/master/include/cvinfo.h>
  + `DEBUG_S_SUBSECTION_TYPE`
  + `CV_DebugSSubectionHeader_t`
  + `CV_DebugSLinesHeader_t`
  + `CV_Line_t`
  + `CV_Column_t`
* MSVC repo, `/src/vctools/PDB/doc/lines.docx`.  This contains an outdated design doc for the Lines Data Substream.
