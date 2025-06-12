# DBI Optional Debug Header Substream

The DBI Optional Debug Header Substream contains a list of stream indexes. Each
stream, if present, contains a copy of a data structure from the corresponding
executable.

```
struct OptionalDebugHeaderSubstream {
    uint16_t streams[];
}
```

The size of the Optional Debug Header Substream is determined by the number of
stream indexes stored in it. The number of elements in `streams` is implied by
the size of the DBI Optional Debug Header Substream; simply divide the size
of the `OptionalDebugHeaderSubstream` by `sizeof(uint16_t)` to compute the
number of streams.

The order of the items in `streams` is significant. The index of each entry is
described in this list:

Index | Name                            | Description
------|---------------------------------|------------
0     | `FPO_DATA`                      | Frame-pointer omission data
1     | `EXCEPTION_DATA`                | Contains a debug data directory of type `IMAGE_DEBUG_TYPE_EXCEPTION`.
2     | `FIXUP_DATA`                    | Contains a debug data directory of type `IMAGE_DEBUG_TYPE_FIXUP`.
3     | `OMAP_TO_SRC_DATA`              | Contains a debug data directory of type `IMAGE_DEBUG_TYPE_OMAP_TO_SRC`. This is used for mapping addresses from instrumented code to uninstrumented code.
4     | `OMAP_FROM_SRC_DATA`            | Contains a debug data directory of type `IMAGE_DEBUG_TYPE_OMAP_FROM_SRC`. This is used for mapping addresses from uninstrumented code to instrumented code.
5     | `SECTION_HEADER_DATA`           | A dump of all section headers from the original executable.
6     | `TOKEN_TO_RECORD_ID_MAP`        | 
7     | `XDATA`                         | Exception handler data
8     | `PDATA`                         | Procedure data
9     | `NEW_FPO_DATA`                  |
10    | `ORIGINAL_SECTION_HEADER_DATA`  |

The `streams` array may contain stream indices whose value is 0xffff. If an
entry in this list has the value 0xffff, then it means that the stream is not
present. This is necessary because the `streams` array may be sparse. For
example, if a PDB does not contain an `EXCEPTION_DATA` stream but _does_ contain
a `SECTION_HEADER_DATA` stream, then the `EXCEPTION_DATA` entry should be set to
0xffff.

This document does not specify the contents of the optional debug headers. Some
of them are publicly documented while others are not.

## Alignment

It is not clear if there is an alignment requirement for the starting offset of
the DBI Optional Debug Header Substream or for its length. To be on the safe
side, encoders should pad this substream to an alignment of 4, by adding a
stream index of 0xFFFF if necessary.

## Determinism

The contents of the streams appear to be deterministic, since they are copies of
data structures found in the executable. We assume the executable is
deterministic.

Because stream indexes are not deterministic it is necessary to copy streams in
a deterministic order. It is recommended to use the order of the streams as
found in this table.

## References

* [DBGTYPE](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/langapi/include/pdb.h#L438)
* [DBI1::fGetDbgData](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/PDB/dbi/dbi.cpp#L3860)

