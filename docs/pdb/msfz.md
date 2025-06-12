# MSFZ Container Specification

This document specifies the MSFZ Container file format. MSFZ replaces one part
of the design of PDB in order to improve the storage and access costs associated
with PDB files.

PDB files store information in _streams_, which are similar to ordinary files.
PDB uses the Multi-Stream File (MSF) container format for storing streams. MSF
allows for easy modification of existing streams; streams may be added, deleted,
or modified without rebuilding the entire file or accessing unrelated parts of
the file.

Unfortunately, MSF's capabilities for modifying streams come with a significant
cost. MSF files divide streams into fixed-size pages, manage allocation state
for those pages, etc. Because most streams' length is not a multiple of the page
size, space is wasted in the last page of each stream due to fragmentation.
(Losses due to fragmentation have been observed to be between 5% and 8% of
typical PDB size.) MSF's complex structure also complicates the process of
decoding a PDB file because the pages that compose a particular stream can be
stored at any location in the file; a stream's pages are not contiguous.

MSFZ makes a different set of trade-offs. It optimizes for the reader, not the
writer. MSFZ files are intended to be written and then never modified
(in-place); if a tool needs to modify information within an MSFZ file then an
entirely new file generally needs to be created. This allows MSFZ to eliminate
the concept of pages and the bookkeeping related to them (the Free Page Map).

MSFZ also supports transparent, "chunked" compression. MSFZ may compress stream
data, and when it does, it may group together parts of different streams into
the same compression chunk. This allows very large streams to be broken into
fragments, where each fragment can be loaded from disk and decompressed without
needing to load and decompress other chunks. This also allows many small streams
to be grouped together into a single chunk and compressed together.

MSFZ uses a more efficient representation of its Stream Directory, compared to
that of MSF. MSF uses lists of page numbers; for many streams, these page
numbers are sequential. MSFZ does not use pages at all, but MSFZ does allow a
stream to be composed of multiple _fragments_. A fragment is a contiguous
sequence of bytes within a stream; a fragment may be compressed or uncompressed.
This allows contiguous streams to be described very efficiently in the Stream
Directory, even streams that are quite large (such as the TPI Stream).

MSFZ replaces only the MSF layer of PDBs. It does not change the model of
streams or the data stored within those streams. PDB files can be converted from
MSF to MSFZ container format, and the reverse, without losing any information.
MSFZ is thus a simple and low-risk change to how PDB files work, with
substantial benefits in size reduction, while still allowing tools to read only
those parts of a PDB that they require.

For the sake of brevity, we will refer to PDB files that use the MSFZ container
format as "PDZ files". These files use the same PDB data structures, but they
are stored using the MSFZ container format rather than MSF.

PDZ files cannot be read directly by tools (debuggers, etc.) that have not been
updated to read PDZ files. These tools will need to be updated, or the PDZ file
will need to be converted to PDB before it is used by the tool.

Many parts of the software ecosystem rely on the `*.PDB` file extension to
identify symbol files. It would be costly to update the ecosystem (ADO
pipelines, customer workflows, etc.) just to use a new file extension for
PDB/MSF files. However, when it is necessary to distinguish between PDB/MSF and
PDB/MSFZ file, we suggest using the `*.PDZ` file extension for PDB/MSFZ files.

## MSFZ concepts: Streams, Fragments, Chunks

A _stream_ is a sequence of bytes that contain PDB-related data. Streams are
stored within MSF and MSFZ files, but are not necessarily stored "directly" in
the sense of a 1:1 mapping between the bytes in the stream and the bytes in the
container. The concept of a stream is the same in MSF and MSFZ, although the
representation is quite different.

A _fragment_ is a contiguous portion of a stream. A stream is composed of zero
or more concatenated fragments. Each byte of stream data is stored in some
fragment and fragments are non-overlapping. Fragments may be stored _compressed_
or _uncompressed_; these will both be described below. Fragments can be stored
in different locations on disk, in the MSFZ file. (Fragment is a concept only in
MSFZ.) Each stream contains a list of its fragments.

A _chunk_ is the unit of compression for compressed stream data. If a fragment
of a stream is compressed, then it is stored within a chunk. Fragments from more
than one stream may be stored within the same chunk. The Chunk Table lists all
of the chunks in a given MSFZ file.

## MSFZ data structures

The MSFZ file consists of these data structures:

* The MSFZ File Header, which identifies the file format, specifies global
  parameters, and specifies the locations of other data structures. The MSFZ
  File Header is located at file offset 0 and is the only data structure that
  has a fixed location. All other data structures are stored at offsets that are
  specified as fields in other data structures.

* The Stream Directory, which specifies the list of fragments that compose the
  contents of each stream. The list of fragments implicitly specifies the size
  of the stream.

* The Chunk Table, which specifies the size and location of each compressed
  chunk.

* Uncompressed stream data fragments, which contains the contents of streams
  that do not use compression.

* Compressed chunks, which contain stream data that has been compressed.
  Compressed chunks can contain stream data from more than one stream; the
  contents of a single stream may be stored in multiple chunks. The relationship
  between streams and chunks is clarified in later sections of this document.

All MSFZ structures use little-endian (LSB first) byte order. The `a-b` notation
for ranges specifies an inclusive range, e.g. the range `0-31` _includes_ values
0 and 31 (and all values between them).

### MSFZ File Header

An MSFZ file begins with this header, at file offset 0:

```c
struct MsfzFileHeader {
    uint8_t signature[32];                  // Identifies this as an MSFZ file.
    uint64_t version;                       // specifies the version number of the MSFZ file format
    uint64_t stream_dir_offset;             // file offset of the stream directory
    uint64_t chunk_table_offset;            // file offset of the chunk table
    uint32_t num_streams;                   // the number of streams stored within this MSFZ file
    uint32_t stream_dir_compression;        // compression algorithm used for the stream directory
    uint32_t stream_dir_size_compressed;    // size in bytes of the stream directory when compressed (on disk)
    uint32_t stream_dir_size_uncompressed;  // size in bytes of the stream directory when uncompressed (in memory)
    uint32_t num_chunks;                    // number of compressed chunks
    uint32_t chunk_table_size;              // size in bytes of the chunk table
};
// sizeof(MsfzFileHeader) == 80
```

All fields in `MsfzFileHeader` are at offsets that are naturally aligned for the type of the field.

The `signature` field identifies a file as being an MSFZ file. It has this value:

```text
00000000 :  4d 69 63 72 6f 73 6f 66 74 20 4d 53 46 5a 20 43 : Microsoft MSFZ C
00000010 :  6f 6e 74 61 69 6e 65 72 0d 0a 1a 41 4c 44 00 00 : ontainer...ALD..
```

The `version` field specifies the file format version used for the MSFZ container format. It is
not related to the version fields of the data stored within PDB streams; it specifies only the
format of the data specified by this specification. The only supported value is:

```c++
const uint64_t MSFZ_FILE_VERSION_V0 = 0;
```

New version numbers will be assigned if the MSFZ container format changes, such
as moving fields or changing their type. All changes to version are assumed to
be incompatible; a MSFZ reader must only decode an MSFZ file if it supports the
specified version number exactly.

`num_streams` field specifies the number of streams in the file. This value has
the same meaning as the number of streams stored in the PDB/MSF Stream
Directory. This value must always be greater than or equal to 1.

`stream_dir_compression` specifies the compression algorithm used for the Stream
Directory. This is described below.

The `stream_dir_size_compressed` and `stream_dir_offset` fields specify the
location and size in bytes of the compressed Stream Directory within the MSFZ
file. `stream_dir_compression` specifies the compression algorithm used to
compress the Stream Directory.

`stream_dir_size_uncompressed` specifies the size in bytes of the Stream
Directory after it has been decompressed in memory.

`num_chunks` specifies the number of compression chunks, which are described in
the Chunk Table. Each entry in the Chunk Table is described by a `ChunkEntry`
record. Chunk compression is described below. The `chunk_table_size` and
`chunk_table_offset` fields specify the location of the Chunk Table. The Chunk
Table itself is never compressed because it is usually small enough that
compression has an insignificant impact.

### Stream Directory

The Stream Directory describes the streams stored in the MSFZ file. It is
encoded using a variable-length encoding scheme because each stream may be
composed of a variable number of fragments.

The MSFZ File Header specifies number of streams as `num_streams`.

Streams may be _nil streams_. A nil stream does not contain any data but is
distinguishable from non-nil stream with zero-length. (Nil streams in MSFZ exist
for compatibility with MSF.) Nil streams are encoded by a `u32` with the value
0xffff_ffff, and are _not_ followed by any fragment records at all, not even a 0
terminator for the fragment list:

```text
00000000 : ff ff ff ff
```

Non-nil streams are encoded by a sequence of _fragment records_. Each fragment
record describes a contiguous sequence of bytes within a stream. Fragments can
be either _compressed_ or _uncompressed_. See below for a description of
compressed and uncompressed fragments.

Each fragment record starts with a `u32` value which specifies the size in bytes
of the fragment. If the fragment is compressed, then this value gives the
_uncompressed_ size of the fragment. That is, the value always describes the
number of bytes of stream data. The fragment size is always non-zero; the value
0 encodes "end of fragment list", not "zero-length fragment".

The fragment size is followed by a `u64` value that specifies the location and
encoding of the fragment.

The following pseudo-structure describes the structure of a non-nil stream
record:

```c
struct Stream {
    Fragment fragments[];       // variable-length list of fragments
    uint32_t end;               // value is always zero
};

struct Fragment {
    uint32_t size;              // size (uncompressed) of this fragment; is never zero
    uint64_t location;          // bit-packed field containing location of this fragment
};
```

Fragments may be stored in _compressed_ or _uncompressed_ form. The `location`
field specifies whether the fragment is compressed or uncompressed mode, and for
each mode, where to find the contents of the fragment. Bit 63 of `location` is
set to 0 for uncompressed mode and 1 for compressed mode.

The `location` field within `Fragment` is _not_ aligned. The size of `Fragment`
is 12 bytes, not 16 bytes.

The `location` field for uncompressed fragments:

```text
+----+---------------+--------------------------------+
| 63 | 62 ... 48     | 47 ... 0                       |
+----+---------------+--------------------------------+
|  0 | reserved (0)  | file_offset                    |
+----+---------------+--------------------------------+
```

The `location` field for compressed fragments:

```text

+----+---------------------+--------------------------+
| 63 | 62 ... 32           | 31 ... 0                 |
+----+---------------------+--------------------------+
|  1 | first_chunk         | offset_within_chunk      |
+----+---------------------+--------------------------+
```

Keep in mind that the `location` field is stored as 64-bit LSB-first value. Bit
63 is stored in bit 7 of byte 7, not in bit 7 of byte 0.

Fragments always have a non-zero size. We use the value 0 to indicate the end of
a sequence of fragments. There is _not_ a `location` field after the `size` if
`size` is 0.

The size of each stream is computed as the sum of the size of the stream's
fragments. The stream size is not directly represented in the encoded form of
the Stream Directory. Typically, most streams have only a single fragment.

Because the size of a stream is computed as the sum of the size of the stream's
fragments, stream size is not limited to 32-bit lengths. Large streams (those
larger than `1 << 32`) can be encoded using multiple fragments. Decoders should
be prepared for recognizing large streams, even if their implementation cannot
otherwise handle large streams.

If a fragment record describes a compressed fragment, then the `first_chunk`
field is the index, within the Chunk Table, of the first fragment that
contributes to this stream. If the size of the fragment and the
`offset_within_chunk` field exceed the size of that first chunk, then the reader
advances to the next chunk, and reads data from its decompressed form. This
repeats until we reach the end of the fragment records for this stream.

For this reason, the order of entries in the Chunk Table is very important
because it determines the order of the uncompressed data that is stored within
the chunks. You can think of the decompressed form of the chunks as forming a
single "virtual address space" and the fragment records as reading from that
address space.

The location of the _compressed_ form of the chunks on-disk is not important,
however. The encoder may write chunks in any order, as long as the bytes that
compose each chunk are contiguous within that chunk (compressed chunks cannot be
interleaved) and the order of the entries in the Chunk Table is correct.

### Example

This is an example of a stream directory.

```text
offset   : contents
00000000 : 00 00 00 00            // stream 0; has no fragments, size is 0
00000004 : a0 01 00 00            // stream 1, fragment 0, fragment size is 0x1a0
00000008 : 50 00 00 00            // fragment 0 location: uncompressed, file offset 0x50
0000000c : 00 00 00 00
00000010 : 00 00 00 00            // end of stream 1
00000014 : ff ff ff ff            // stream 2 is nil
00000018 : 84 ac 06 00            // stream 3, fragment 0, fragment size is 0x6ac84
0000001c : e0 10 00 00            //     offset_within_chunk: 0x10e0
00000020 : 80 00 00 e5            //     chunk_index e5, with bit 63 set (meaning: compressed chunk)
```

It is expected that most streams will be stored compressed. Compression not only
reduces the size of PDZ files, but typically makes I/O more efficient.

The Stream Directory itself can be compressed. The `stream_dir_compression`
field of the MSFZ File Header specifies the compression algorithm used for the
Stream Directory.

### Uncompressed streams

In uncompressed form, the stream data is stored in a single contiguous sequence
of bytes in the MSF file. The `location` field sets bit 63 to 0. Bits 0-47
contains the file offset of the stream data. Bits 48-62 of `location` are
reserved and are set to 0. The contents of the stream are stored contiguously in
the MSFZ file; MSFZ does not use pages. The stream data may be read directly
from the MSFZ file without any processing.

### Chunk-compressed streams

If a stream contains compressed fragments, then the `location` field of
`Fragment` has bit 63 set to 1 and also contains two bit-packed subfields:
`chunk` (stored in bits 32-62) and `offset_within_chunk` (stored in bits 0-31).
`chunk` specifies the index of the chunk that contains the fragment's data.
`offset_within_chunk` specifies the _decompressed_ byte offset within the chunk
where the stream's data begins. The Chunk Table is described below.

Chunk boundaries are not required to be aligned to fragment boundaries. A chunk
may contain data from more than one fragment and these fragments may come from
different streams. The contents of a single stream may be spread across multiple
fragments, possibly using different compression modes (compressed vs.
uncompressed).

Compressed chunks may be stored in any order on disk.

This diagram illustrates some of the cases:

```text
                               --> layout of file contents -->

-------------------+------------------------------------+---------------------+-----------------
    compressed     | compressed                         | uncompressed        | compressed
... chunk 42       | chunk 43                           | stream data         | chunk 44 ...
-------------------+------------------------------------+---------------------+-----------------
       ↑                        ↑                       ↑                     ↑
       |                        |                       |                     |
------/ \---- stream 10 -------/ \-- stream 11 --------/ \--- stream 12 -----/ \--- stream 13 --
```

* Stream 10 is compressed and spans chunks 42 and 43.
* Stream 11 is compressed and is stored entirely within chunk 43.
* Stream 12 is uncompressed. Its contents are stored in the MSFZ file but there is no entry in the
  Chunk Table for it.
* Stream 13 is compressed and begins in chunk 44.

### Chunk Table

The Chunk Table lists the compressed chunks within the MSFZ file. The location
and size of the Chunk Table is specified in the MSFZ File Header. The Chunk
Table is an array of `ChunkEntry` records:

```c
struct ChunkEntry {
    uint64_t file_offset;               // file offset of the compressed data for this chunk
    uint32_t compression;               // compression algorithm for this chunk
    uint32_t compressed_size;           // size in bytes of the compressed (on-disk) chunk data
    uint32_t uncompressed_size;         // size in bytes of the uncompressed (in-memory) chunk data
};
```

Note that `ChunkEntry` contains unaligned data. The size of `ChunkEntry` is 20
bytes, not 24, and the alignment is 1, not 8.

Each chunk specifies its compression algorithm, and hence different chunks may
use different compression algorithms. Currently, the only supported algorithm is
[`Zstd`](https://github.com/facebook/zstd).

To read any data from a chunk, usually the entire compressed chunk must be read
from disk and decompressed. For this reason, the encoder of an MSFZ file chooses
a chunk size that is a good tradeoff between encoding efficiency and the cost of
reading and decompressing chunks. There is no requirement that different chunks
have the same size, either before or after decompression. This gives encoders a
lot of freedom.

The _uncompressed_ form of consecutive chunks forms a virtual byte array.
Streams that cross chunk boundaries rely on this. If a stream crosses one or
more chunk boundaries, then all of the chunks that contribute to the stream must
be contiguous in the Chunk Table.

Each entry in the Chunk Table specifies the file offset where the compressed
form of that chunk begins, the size in bytes of the compressed chunk (on-disk),
and the size in bytes of the decompressed chunk (in memory). This allows
decoders to allocate buffers of the correct size before reading data from disk.

### Order of chunk contents and uncompressed stream contents

The location and size of each compressed chunk is specified by that chunk's
entry in the Chunk Table. Similarly, the size and location of each uncompressed
stream is stored in that stream's entry in the Stream Directory. Let _fragment_
refer to either a portion of a compressed chunk or to a contiguous sequence of bytes that
are stored in the MSFZ file without any compression.

Fragments may be stored anywhere in the MSFZ file, except for the following
constraints:

* No two fragments may overlap.
* No fragment may overlap the MSFZ File Header.
* No fragment may overlap the Stream Directory.
* No fragment may overlap the Chunk Table.

These constraints _allow_ the following:

* Uncompressed fragments and compressed chunks may appear in any order within
  the MSFZ file.

* The order of records in the Chunk Table is significant, but the location of
  the compressed chunk data within the MSFZ file is not significant. Encoders
  may write compressed chunks in any order, as long as the Chunk Table correctly
  describes their location.

* There may be unused bytes between pieces. These unused bytes can be used for
  alignment padding. Compressed chunks do not need alignment padding. However,
  encoders may choose to align the starting offset of uncompressed file
  structures, to enable direct memory mapping of uncompressed streams.

  Encoders should ensure that unused bytes do not contain data that was not
  intended to be written to disk, such as accidentally-copied data from memory.
  Encoders are encouraged to always zero-fill unused bytes.

This gives encoders considerable freedom when writing MSFZ files. The following
lists some (non-normative, non-exclusive) ideas for using this freedom when
encoding MSFZ files:

* Simplicity: Since encoders may write pieces in any order, they can simply
  write data as it becomes available.
* Locality: Encoders may write the data that they expect to be accessed most
  frequently in neighboring locations within the file. For example, the Stream
  Directory could be written immediately after the MSF File Header, followed by
  the contents of the PDBI Stream and the DBI Stream. This would allow a reader
  to read a lot of useful data in a single read, at the start of the file,
  without decoding any of the contents of file.
* Encoding efficiency: An encoder may compress chunks using multiple threads
  (CPUs). As each CPU finishes writing a chunk, it may directly append the chunk
  data to the output file. This would minimize lock contention and serialization
  within the encoding process. However, this does introduce non-determinism,
  since different CPUs would race for access to the file.

There are other hypothetical ways to use this freedom. The point is, the
specification permits encoders to write pieces in any order, as long as the
Stream Directory and Chunk Table correctly describe the streams.

## Compression algorithms

Several fields identify a compression algorithm. This section enumerates the specified
compression algorithms.

```c++
const uint32_t COMPRESSION_NONE = 0;
const uint32_t COMPRESSION_ZSTD = 1;
const uint32_t COMPRESSION_DEFLATE = 2;
```

## Procedure for opening and validating an MSFZ file

* Read the MSFZ File Header.
  * Verify that `signature` matches `MSFZ_FILE_SIGNATURE`.
  * Verify that `version` matches a supported version. (Currently, the only supported version is `MSFZ_FILE_VERSION_V0`.)
  * Verify that `stream_dir_compression` is a supported compression algorithm.
  * Verify that
    `num_streams * size_of::<StreamEntry>() == stream_dir_size_uncompressed`.
  * Verify that `num_chunks * size_of::<ChunkEntry>() == chunk_table_size`.
  * Verify that the file range for the Stream Directory is valid, given the
    length of the MSFZ file.
  * Verify that the file range for the Chunk Table is valid, given the length of
    the MSFZ file.
  * Read the Stream Directory.
  * Read the Chunk Table.
* For each `ChunkEntry` in the Chunk Table:
  * Verify that the `uncompressed_size` is non-zero
  * Verify that the `compressed_size` is non-zero and that the file range
    implied by the `file_offset` and `compressed_size` is valid, given the
    length of the MSFZ file.
* For each `StreamEntry` in the Stream Directory:
  * If the stream is a nil stream (`size == NIL_STREAM_SIZE`) or a zero-length stream (`size == 0`), then verify that `location == 0`.
  * If the stream is uncompressed, verify that the file range implied by the `location` and `uncompressed_size` is valid.
  * If the stream is compressed, verify that the range of chunks implied by the
    `location` (when unpacked into `first_chunk` and `offset_within_chunk`) and
    `uncompressed_size` is valid. This requires checking the contents of the
    Chunk Table. Also verify that the `uncompressed_size` is non-zero.
* Verify that the byte ranges for all compressed chunks and uncompressed streams
  are non-overlapping and that they do not overlap the File Header, Stream
  Directory, or Chunk Table.

## Procedure for reading data from a stream

Given a stream `s`, an `offset` within the stream to read, and `len` bytes to read:

* Locate `s` in the `StreamEntry` in the Stream Directory. `s` is required to be
  less than `num_streams`.
* If `compression` is `COMPRESSION_NONE`, then use the `location` and
  `uncompressed_size` to read the contents of the stream directly from the MSFZ
  file.
* If `compression` is `COMPRESSION_CHUNKED`, then:
  * Decompose the `location` into `first_chunk` and `offset_within_chunk`. These
    identify the chunk that contains the data at the start of the stream.
    Remember that the chunk may contain data from other streams, which is why
    the `offset_within_chunk` value is necessary.
  * Use the `offset` and `offset_within_chunk` values to scan forward within the
    chunk list. The goal is to find the first chunk that contains the desired
    data (the data at `offset`), rather than simply the first chunk that
    contains data at stream offset 0.
  * If necessary, read chunks from disk and decompress them.
  * Scan forward in the chunk list. Consume `len` bytes of data from chunks,
    crossing chunk boundaries if necessary.

## Author

* [Arlie Davis](ardavis@microsoft.com)
