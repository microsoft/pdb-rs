# IPI Stream (Fixed Stream 4)

The IPI Stream uses many of the same data structures as the TPI Stream and the
[TPI Stream](tpi_stream.md) specification should serve as the specification for
the IPI Stream. However, The IPI Stream and TPI Stream store different kinds
records and serve different purposes.

These aspects of the TPI Stream and IPI Stream are identical:

* Stream header
* Record framing, but not record contents
* Hash value substream
* Hash stream

See [CodeView Item Records](../codeview/items/items.md) for a description of
the records that can be stored in the IPI Stream.  **Only item records can be
stored in the IPI Stream**.  Although the IPI Stream uses many of the same
data structures as the TPI Stream, they do not contain the same records and
serve different purposes.

To find a specific record in the IPI given its `ItemId`, first subtract
`type_index_begin` from the `ItemId`. This gives the 0-based index of the record
within the stream; let this be the value `R`. Then, begin decoding records
within the IPI Stream, counting them as they are decoded. When `R` records have
been decoded, the next record is the desired record.

The value of `type_index_begin` (in the IPI Stream Header) is typically 0x1000.
No other value has been observed.

## IPI Hash Value Substream and IPI Hash Stream

The IPI Stream contains an IPI Hash Value Substream, which has the same
structure as the TPI Hash Value Substream.

The IPI Stream also has a corresponding IPI Hash Stream, which has the same
structure as the TPI Hash Stream but it describes records in the IPI Stream, not
TPI Stream.
