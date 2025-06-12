- [Multi-Stream File (MSF) Container](#multi-stream-file-msf-container)
- [Basic structure](#basic-structure)
  - [Pages](#pages)
  - [Streams](#streams)
  - [Small MSF vs Big MSF](#small-msf-vs-big-msf)
- [Big MSF Encoding](#big-msf-encoding)
  - [MSF File Header](#msf-file-header)
  - [Stream Directory](#stream-directory)
  - [Stream Directory location on disk](#stream-directory-location-on-disk)
  - [Free Page Map](#free-page-map)
  - [Intervals (Big MSF only)](#intervals-big-msf-only)
    - [Example](#example)
    - [Wasted space in the FPMs](#wasted-space-in-the-fpms)
- [Small MSF Encoding](#small-msf-encoding)
  - [MSF File Header - Small MSF](#msf-file-header---small-msf)
- [Stream Directory](#stream-directory-1)
  - [Small MSF](#small-msf)
  - [Determinism](#determinism)
  - [Location on disk - Small MSF](#location-on-disk---small-msf)
- [Stream usage](#stream-usage)
- [2-phase commit protocol](#2-phase-commit-protocol)
- [Example - Big MSF](#example---big-msf)
- [References](#references)

# Multi-Stream File (MSF) Container

PDBs store information in _streams_ where each stream is analogous to a file and
an MSF container is analogous to a filesystem volume (e.g. a raw disk image).
MSF allows processes to create, delete, and modify streams without updating the
entire MSF file. Updates to existing PDBs are efficient because a writer can
update only those on-disk pages that need to be modified. Modifying a stream,
including changing the size of the stream, does not require moving other
streams.

This section will use the term MSF for the overall file structure of the PDB.
MSF is the container format for all of the information stored within PDBs.
However, the MSF format itself does not contain any PDB-specific information,
aside from a signature field in its header that identifies it as being a PDB.

# Basic structure

The MSF is organized as:

* The MSF File Header, which describes global parameters that control the file
  layout.
* An array of fixed-size pages, each of which is typically 4096 bytes in length.
* An ordered set of streams, where each stream is a logical sequence of bytes,
  similar to an ordinary file. Streams are the main abstraction that the MSF
  creates. The Stream Directory specifies the length (in bytes) of each stream
  and the pages that compose each stream.
* The Free Page Map, which is used for allocating pages. The Free Page Map also
  organizes pages into regions called intervals. Intervals are fixed-size
  sequences of pages, related to a specific portion of the Free Page Map.
  Intervals are relevant only to the page allocator and are not meaningful to
  the user of the PDB/MSF.

## Pages

The MSF file is organized as a sequence of fixed-size pages. Pages are the unit
of allocation and organization; all stream data is stored in pages. Pages are
identify by _page number_, which is a `uint32_t` value.

All pages have the same page size. The `page_size` field in the MSF File Header
specifies this value. The most common value for `page_size` size is 4096 bytes.

> Invariant: `page_size` is a power of 2 and is in the inclusive range 512-65536
> (0x200-0x10000).

> Invariant: Each page is stored at a file offset within the MSF that is a
> multiple of `page_size`.

The default and most common value for page_size is 4096 (0x1000). Many tools
that process PDBs, such as older builds of the MSPDB and DIA libraries, can only
work with PDBs that have a page_size of 4096.

> Determinism: Use a `page_size` of 4096, unless the total PDB data size is
> large enough that it requires a larger page size. Or allow `page_size` to be
> an input parameter, not a free variable.

The `num_pages` field of the MSF File Header specifies the number of valid pages
in the file. Valid means that the bits in the FPM are meaningful and that a
valid page may be assigned to a stream. In the example above, the num_pages
field is 20923 (0x51BB). The total file length is 85,700,608 (0x51B_B000) bytes,
so the `num_pages` field covers exactly the file size.

> Invariant: The num_pages field must be less than or equal to the byte length
> of the MSF file, divided by the page size.

> Determinism: The byte length of the MSF file must be a multiple of `page_size`
> size and the `num_pages` field must be equal to the byte length of the MSF
> file divided by `page_size`.

## Streams

Streams store the important information within PDBs. A stream is a logical
sequence of bytes, similar to a file. A stream can be any size, as long as the
size can be represented as a `uint32_t`. The contents of each stream are stored
in a sequence of pages. The pages in a stream may be stored at any valid page
offset within the MSF file; there is no requirement that the pages be stored
sequentially (contiguously). Pages from different streams may be intermixed in
arbitrary orders.

Streams are identified by `stream index`. Small MSF uses `uint16_t` for
`num_streams`, so Small MSF has a limit of 65535 usable streams (stream 0 is
reserved for the Old Stream Directory). Big MSF uses `uint32_t` for
`num_streams`, so it can encode a much larger number of streams, although due to
encoding requirements for the Stream Directory it is not possible to create
`2^32 - 1` streams. Unfortunately, while MSF itself allows for a large number of
streams (when using Big MSF), many of the data structures defined by PDB (e.g.
the Named Streams Table) use `uint16_t` for stream indexes. This limits the
number of streams to 65535.

Each stream "owns" the pages that are assigned to it. If a page is allocated to
a stream then that page may not be allocated to any other stream. The page may
occur at only one position within the stream.

The Free Page Map stores a bitmap of the set of pages that are free (available
for allocation to a stream). The Free Page Map is an important data structure
for the implementation of the MSF, but is not directly visible or relevant to
the user of the MSF.

## Small MSF vs Big MSF

There are two variants of the MSF file format: "Small MSF" and "Big MSF". Big
MSF is an improvement on Small MSF which enabled PDBs to be much larger. Small
MSF is obsolete; all encoders should use Big MSF. Small MSF is documented here
because there are still some PDBs in storage that are encoded using Small MSF.

Big MSF and Small MSF differ in the structure of the Stream Directory, Free Page
Map, MSF File Header, and in storage limits, but provide the same abstraction
(streams).

Decoders should be capable of reading Small MSF files, or reliably identifying
them and rejecting them. New encoder implementations should not implement
support for Small MSF.

# Big MSF Encoding

## MSF File Header

The MSF File Header is stored at file offset 0. It contains a `magic` field
which unambiguously identifies this file as an MSF file and specifies either the
Big MSF or Small MSF encoding.

The structure of the MSF File Header, Stream Directory, and Free Page Map are
dependent on whether the MSF file is encoded using Big MSF or Small MSF. This
document will describe each part of the MSF file using the Big MSF encoding,
then specify each part of the file using Small MSF encoding. Some of the
descriptions of the Small MSF encoding are given in terms of Big MSF.

The MSF File Header is stored at file offset 0. When using Big MSF the MSF File
Header has this structure:

```text
// sizeof = 52 bytes (fixed-size prefix)
struct BigMsfFileHeader {
    uint8_t magic[32];
    uint32_t page_size;
    uint32_t active_fpm;
    uint32_t num_pages;
    uint32_t stream_dir_size;
    uint32_t small_stream_pages_map;      // not used; do not read
    uint32_t big_stream_pages_map[];
}
```

The `magic` field has the following contents:

```text
const uint8_t MSF_BIG_MAGIC[32] = {
    0x4d, 0x69, 0x63, 0x72, 0x6f, 0x73, 0x6f, 0x66, // Microsof
    0x74, 0x20, 0x43, 0x2f, 0x43, 0x2b, 0x2b, 0x20, // t C/C++
    0x4d, 0x53, 0x46, 0x20, 0x37, 0x2e, 0x30, 0x30, // MSF 7.00
    0x0d, 0x0a, 0x1a, 0x44, 0x53, 0x00, 0x00, 0x00, // ...DS...
}
```

`page_size` specifies the size in bytes of each page.

> Invariant: When using Big MSF, `page_size` is a power of 2 and is in the range
> 512-65536 (0x200-0x10000).

`active_fpm` specifies the Active Free Page Map.

> Invariant: `active_fpm` is 1 or 2. No other values are valid.

`num_pages` specifies the total number of pages in the MSF file. This value
includes all pages: free pages, the MSF Header Page, FPM pages, stream pages,
etc. Normally, `num_pages = file_size / page_size`, where `file_size` is the
size of the MSF file in bytes.

`small_stream_pages_map` is obsolete. Decoders should ignore this field. Encoders should set it to 0.

`stream_dir_size` specifies the size in bytes of the Stream Directory.
`big_stream_pages_map` specifies the location of the Stream Directory. They will
be described in the Stream Directory section, below. Note that
`big_stream_pages_map` is an array, not just a single `uint32_t` field.

## Stream Directory

The Stream Directory is a variable-length data structure that lists the streams
defined in the MSF file, their size in bytes, and the page numbers for the pages
assigned to each stream. When using Big MSF the Stream Directory has this
structure:

```text
struct BigStreamDirectory {
    uint32_t num_streams;
    uint32_t stream_sizes[num_streams];
    uint32_t stream_pages[num_pages_in_all_streams];  // num_pages_in_all_streams is defined below
}
```

The `stream_dir_size` field of the MSF File Header specifies the size in bytes
of the Stream Directory.

> Invariant: `stream_dir_size` is a multiple of 4.

`num_streams` specifies the number of streams in the MSF file.

> Invariant: `num_streams` is at least 1.

`stream_sizes` specifies the size in bytes of each stream. A stream may be a
"nil stream". Nil streams have a `stream_size[i]` value of `NIL_STREAM_SIZE`,
whose value is 0xFFFF_FFFF. Nil streams do not store any data and do not have
any pages assigned to them.

The number of pages in a non-nil stream is computed by dividing
`stream_sizes[i]` by `page_size` and rounding up. Rounding up accounts for the
last page in the stream which will be partially-filled if the stream size is not
a multiple of `page_size`. A nil stream does not have any pages assigned to it.
Let
`num_pages_in_stream(s) = stream_sizes[s] == NIL_STREAM_SIZE ? 0 : divide_round_up(stream_sizes[s], page_size)`
be the number of pages in stream `s`.

> Determinism: If `stream_sizes[s]` is not a multiple of `page_size`, then the
> unused bytes in the last page are set to zero.

`stream_pages` contains the list of pages that are assigned to each stream,
concatenated, and in the same order as the streams. To traverse (or build) this
array, use the `num_pages_in_stream(s)` function to compute the number of pages
for each stream, starting at `stream_sizes[0]` and ending at
`stream_sizes[num_streams - 1]`. Use the value of `num_pages_in_stream(s)` to
traverse the correct number of values in `stream_pages`.

Let `num_pages_in_all_streams` be the sum of `num_pages_in_stream(s)` for all
streams. This gives the size of `stream_pages`. Since `stream_pages` contains
the concatenated list of pages in each stream, `stream_pages` begins with the
list of pages in stream 0, followed by the pages in stream 1, then stream 2, ...
then the pages for stream `num_streams - 1`.

> Invariant: The contents of the Stream Directory must be consistent with the
> value of `stream_dir_size`.
> `stream_dir_size = sizeof(uint32_t) * (1 + num_streams + num_pages_in_all_streams)`.

Stream 0 is special; it points to the "Old Stream Directory" and is part of the
2-phase protocol. Stream 0 must always be present (even if it is zero-length or
invalid).

> Invariant: If a page is allocated to a stream then it is not used in any other
> stream or used again within the same stream. Stated differently, the
> `stream_pages` array does not contain any duplicates.

## Stream Directory location on disk

The Stream Directory is stored in a 3-level hierarchy of pages:

1. The MSF File Header contains a list of page numbers which point to the Stream
   Directory Page Map.
2. The Stream Directory Page Map is a set of pages which contain page numbers
   which point to the Stream Directory.
3. The Stream Directory contains the byte representation, described in the
   previous section.

It is easiest to understand this hierarchy by walking through the process of
buiding the Stream Directory "from the bottom up". That is, first you construct
the byte representation of the Stream Directory, then you build the Page Map,
then you store the pointers to the Page Map in MSF Page 0.

* First, build the Stream Directory as described in the previous section. This
  is a variable-size data structure which specifies the number of streams, their
  size in bytes, and the pages assigned to each stream.

* Next, we define several values:

  * Let `stream_dir_size` be the size in bytes of the Stream Directory that we
    have just constructed.
  * Let `stream_dir_pages` be an array of `uint32_t` page numbers that contains
    the page numbers that were allocated for storing the Stream Directory.
  * Let `num_stream_dir_pages = divide_round_up(stream_dir_size, page_size)`.
    This is the number of pages that contain the Stream Directory.
  * Let
    `num_page_map_pages = divide_round_up(num_stream_dir_pages * sizeof(uint32_t), page_size)`.
    This is the number of pages in the Stream Directory Page Map.

* Next, allocate pages for storing the Stream Directory and write the Stream
  Directory to those pages. This step allocates `num_stream_dir_pages` pages.
  Let `stream_dir_pages` be an array of `uint32_t` page numbers that contains
  the page numbers that were allocated for storing the Stream Directory.

* Next, allocate pages and write `stream_dir_pages` to those pages. Let
  `page_map` be the list of page numbers that were just allocated.

* Finally, write `page_map` into the `big_stream_pages_map` field of the
  `BigMsfFileHader`.

> Determinism: If `stream_dir_size` is not a multiple of `page_size`, then the
> unused bytes in the last page of the Stream Directory should be set to zero.

> Determinism: If `num_stream_dir_pages` is not a multiple of `page_size` then
> the unused bytes in the last page of the Stream Directory Page Map should be
> set to zero.

Decoders should ignore the `small_stream_pages_map` field. Encoders should set
it to zero.

## Free Page Map

Each MSF file contains two Free Page Maps (FPMs), called FPM1 and FPM2. One FPM
is the _active FPM_ while the other is the _inactive FPM_. The `active_fpm`
field of the MSF File Header specifies which FPM is the active FPM.

The Active FPM describes the committed state of the MSF file. In the Big MSF
encoding, the only legal values for `active_fpm` are 1 and 2.

The inactive FPM has the same size and structure as the Active FPM, but its
contents are undefined. The `active_fpm` field specifies which FPM is currently
in use.

The reason for storing two FPMs is to enable MSF to support a 2-phase commit
protocol works. When a PDB is being modified, changes are written to the
Inactive FPM. To commit changes, a single write to the MSF File Header (page 0)
reverses the roles of the active and inactive FPMs. The FPM that was active
becomes inactive, and now describes the state of the MSF file before the commit
point. The FPM that was inactive becomes active, and now describes the new
committed state of the MSF.

In this document, "Free Page Map" (or FPM) refers to the Active FPM, unless
clarified by context.

> Invariant: When using Big MSF the `active_fpm` field can only be 1 or 2.

> Determinism: The `active_fpm` field must be 1.

> Determinism: The inactive FPM must have its contents set to some deterministic
> contents, such as "all busy" or "all free".

Each FPM is stored in a set of pages that are reserved for its use. The pages
that store the FPM are not allocated using the FPM; that would be a
self-referential definition. Instead, pages are allocated for the FPM using a
simpler approach, called _intervals_. Intervals are described below.

Each FPM is stored as a bitmap. The bits are assigned in little-endian format,
so bit 0 in byte 0 of the FPM corresponds to page 0. The byte offset within the
FPM for page `p` is `p / 8`. The bit index within that byte for a page `p` is
`p % 8` (remainder).

Each bit in the FPM specifies whether the corresponding page is free (FPM bit is
1) or busy (FPM bit is 0). The definition of "free" or "busy" requires
clarification, depending on what is stored in that page.

FPM bit value   | Page usage
----------------|-----------
0 (busy)        | The page is part of the Stream Directory.
0 (busy)        | The page is part of the Stream Directory Page Map, which contains page numbers of the Stream Directory.
1 (free)        | The page is part of the Old Stream Directory (stream 0).
0 (busy)        | The page is part of a stream other than the Stream Directory (streams 1 and higher).
0 (busy)        | The page is part of the FPM itself. See the "Intervals" section for how FPM pages are assigned.
1 (free)        | The page is not used for anything, and is not part of the FPM itself.

It may seem surprising that pages that are assigned to the Stream Directory are
considered "free". This is because the MS-PDB implementation reads the Stream
Directory into memory, then marks these pages as "deleted" in an in-memory
bitmask. This protects the pages from being overwritten, until the next commit.

## Intervals (Big MSF only)

An interval is a fixed-size sequence of pages. Every MSF file consists of 1 or
more intervals. The number of pages in each interval is equal to the number of
bytes in each page.

Each FPM is composed of one or more pages. Those pages come from the reserved
pages in each interval. Page 1 within each interval is reserved for FPM1 and
page 2 within each interval is reserved for FPM2. Page 0 within interval 0 is
reserved for the MSF File Header; page 0 within all other intervals is available
for storing stream data.

We compute the number of intervals in a file from the number of pages in a file
by dividing the `num_pages` field (from the MSF File Header) by `page_size` and
rounding up.

> Let `num_intervals = divide_round_up(num_pages, page_size)`

Intervals are quite large, since their size is the square of `page_size`. For
the typical page size of 4096, the interval size would be 16,777,216 bytes (16
MiB). For this reason, there is no requirement that a complete interval be
stored on disk.

### Example

Let's examine the intervals within our example file, whose header is show above.
The page size is 4096 bytes and `num_pages` = 20,923 (0x51B). Then
`num_intervals = divide_round_up(20923, 4096)`, giving 6 intervals. Numbers are
shown in hex.

Interval | Page number of start of interval = `page_size * interval` | File byte offset of start of interval = `page_size * page_size * interval` | FPM1 byte offset = `(page_size + 1) * page_size` | FPM2 byte offset = `(page_size + 2) * page_size`
--|--------|-------------|-------------|------------
0 | 0      | 0           |      0x1000 |      0x2000
1 | 0x1000 | 0x0100_0000 | 0x0100_1000 | 0x0100_2000
2 | 0x2000 | 0x0200_0000 | 0x0200_1000 | 0x0200_2000
3 | 0x3000 | 0x0300_0000 | 0x0300_1000 | 0x0300_2000
4 | 0x4000 | 0x0400_0000 | 0x0400_1000 | 0x0400_2000
5 | 0x5000 | 0x0500_0000 | 0x0500_1000 | 0x0500_2000

### Wasted space in the FPMs

The design of the intervals allocation scheme for FPMs implies that there is one
_byte_ allocated in each FPM for each page. This is more storage space than,
since the FPM is a _bitmap_.

The FPM bitmap is stored within the pages that are allocated to it, in the
sequence of pages that are allocated to it. This means that the pages that store
the FPM are not necessarily in the same interval as the pages that are described
by the FPM.

We will use an example to illustrate this. We will continue using the example
above, where `num_pages` is 20,923 (0x51B). In this example:

* FPM1 is stored in 6 pages, whose page numbers are 1, 0x1001, 0x2001, 0x3001,
  0x4001, and 0x5001.
* The number of **bits** needed to store the FPM bitmap is equal to `num_pages`,
  which is 0x51B.
* The number of **bytes** needed to store the FPM bitmap is equal to
  `divide_round_up(num_pages, 8)`, which is 164.
* Because 164 is less than 4096 (our `page_size`), the entire FPM is stored
  **only** in page 1. The rest of the storage space in page 1, as well as _all_
  of the storage space in pages 0x1001, 0x2001, 0x3001, 0x4001, and 0x5001, is
  wasted.
* Be aware that FPM bits that are _stored_ in page 1 (interval 0) nevertheless
  _describe_ pages in intervals 1, 2, 3, 4, and 5.

As we see, the storage space assigned to the FPM is always 8 times as large as
it actually needs to be. This wasted space is not substantial, but it is often
confusing when trying to understand the design of the FPM.

# Small MSF Encoding

## MSF File Header - Small MSF

The Small MSF variant uses this structure for the MSF File Header:

```text
struct SmallMsfFileHeader {
    uint8_t magic[44];
    uint32_t page_size;
    uint16_t active_fpm;
    uint16_t num_pages;
    uint32_t stream_dir_size;
    uint32_t stream_dir_ptr;      // obsolete; do not read
    uint32_t stream_dir_pages[];
}
```

The `magic` field must have exactly the following contents:

```text
const uint8_t MSF_SMALL_MAGIC[44] = {
    0x4d, 0x69, 0x63, 0x72, 0x6f, 0x73, 0x6f, 0x66, // Microsof
    0x74, 0x20, 0x43, 0x2f, 0x43, 0x2b, 0x2b, 0x20, // t C/C++
    0x70, 0x72, 0x6f, 0x67, 0x72, 0x61, 0x6d, 0x20, // program
    0x64, 0x61, 0x74, 0x61, 0x62, 0x61, 0x73, 0x65, // database
    0x20, 0x32, 0x2e, 0x30, 0x30, 0x0d, 0x0a, 0x1a, //  2.00...
    0x4a, 0x47, 0x00, 0x00,                         // JG..
}
```

`page_size` has the same meaning as in Big MSF.

> Invariant: When using Small MSF, `page_size` is a power of 2 and is in the
> range 512-4096 (0x200-0x1000).

`active_fpm` specifies the page number of the Active FPM. Unlike Big MSF,
`active_fpm` can have several different values.

> TODO: Document what the `active_fpm` constraints are. It's not important right
> now, because we do not need to generate new Small MSF files, only read them.

`num_pages` has the same meaning as in Big MSF.

`stream_dir_size` has the same meaning as in Big MSF.

# Stream Directory

The Stream Directory specifies the number of streams, the size in bytes of each
stream, and the page numbers that make up each stream.

We will first describe the structure of the contents of the Stream Directory.
Then we will describe the on-disk location of the Stream Directory.

There are two variants of the Stream Directory: "Small MSF" and "Big MSF". Both
store the same semantic information, but represent that information differently
and have different limitations. Old MSF is obsolete; all encoders should encode
Big MSF.

## Small MSF

When using Small MSF, the Stream Directory has this structure:

```text
struct SmallStreamDirectory {
    uint16_t num_streams;
    uint16_t padding;
    SmallStreamEntry streams[num_streams];
    uint16_t stream_pages[num_pages_in_all_streams];
}

struct SmallStreamEntry {
    uint32_t stream_size;   // size in bytes of this stream
    uint32_t ignored;       // This used to be an in-memory pointer, so it was meaningless.
}
```

In Small MSF, finding the size of a stream requires indexing into `streams` and
accessing the `stream_size` filed, i.e. `streams[s].stream_size`. Aside from
that difference, the stream size is interpreted in the same way as
`stream_sizes` as defined in Big MSF.

Note that `stream_pages` uses `uint16_t`, not `uint32_t`. Aside from that
detail, `stream_pages` works the same way as defined in Big MSF.

Big MSF improved on Small MSF by upgrading page numbers and `num_streams` from
`uint16_t` to `uint32_t` and by eliminating the useless `ignored` field.

## Determinism

> Determinism: There are no "nil" streams (streams with
> `stream_sizes[i] == NIL_STREAM_SIZE`).

> Determinism: The page numbers of all pages allocated to all streams are
> assigned in strictly-increasing order. That is, the elements in `stream_pages`
> are sorted in ascending order.

These determinism rules are guidelines, not invariants. They are one way to
achieve determinism, but apps that generate PDB/MSF files are not required to
follow these guidelines, as long as the apps still generate data in a
deterministics order.

## Location on disk - Small MSF

# Stream usage

The MSF Container format enables the storage of streams, which store the
significant data of a PDB. This lists some of the important streams in a PDB:

* [PDB Information Stream](pdbi_info.md) - Contains global information about the
  PDB. This is usually the first stream that any tool reads. It provides the
  binding key (unique GUID, age), PDB version number, and contains the Named
  Streams Table.
* [Debug Information Stream](dbi.md) - Contains pointers to debugging
  information
* [Module Streams](module_stream.md) - Each describes a given module
  (compiland).
* [Names Stream](names_stream.md) - Contains a set of strings, which are
  referenced by other data structures using a small integer (`NameIndex`).
* [TPI Stream](tpi_stream.md) - Describes the types used by a program.
* [IPI Stream](ipi.md) - Contains identifiers and metadata about compilation.

# 2-phase commit protocol

MSF provides a 2-phase commit protocol, which allows making a series of
modifications to a PDB file and only committing them with a single update to the
PDB header file.

> TODO: Specify this protocol. Not urgent, since we do not _modify_ PDBs
> in-place currently.

# Example - Big MSF

This is an example of page 0 of a PDB file:

```text
00000000 : 4d 69 63 72 6f 73 6f 66 74 20 43 2f 43 2b 2b 20 : Microsoft C/C++
00000010 : 4d 53 46 20 37 2e 30 30 0d 0a 1a 44 53 00 00 00 : MSF 7.00...DS...
00000020 : 00 10 00 00 01 00 00 00 bb 51 00 00 00 6b 01 00 : .........Q...k..
00000030 : 00 00 00 00 ba 51 00 00 00 00 00 00 00 00 00 00 : .....Q..........
00000040 : ... zeroes ...
00001000 : (end)
```

* The `magic` field is clearly visible. It is using the "Big MSF" magic value.
* At offset 0x20, `page_size` is 0x1000.
* At offset 0x24, `active_fpm` is 1.
* At offset 0x28, `num_pages` is 0x51bb (20,923). When multipied by `page_size`,
  this gives a file size of 0x51bb000 (85,700,608).
* At offset 0x2c, `stream_dir_size` is 0x16b00 (92,928).
* At offset 0x30, `small_stream_pages_map` is 0. Since `magic` specifies "Big
  MSF", it is expected that this field is zero.
* At offset 0x3c, `big_stream_pages_map` contains one page pointer, whose value
  is 0x51ba.

The page pointer 0x51ba points to a page that contains page numbers that contain
the Stream Directory. Remember that we multiply page numbers by `page_size`
(which is 0x1000 in this example) to get file offsets. At page 0x51ba we see:

```text
051ba000 : a3 51 00 00 a4 51 00 00 a5 51 00 00 a6 51 00 00 : .Q...Q...Q...Q..
051ba010 : a7 51 00 00 a8 51 00 00 a9 51 00 00 aa 51 00 00 : .Q...Q...Q...Q..
051ba020 : ab 51 00 00 ac 51 00 00 ad 51 00 00 ae 51 00 00 : .Q...Q...Q...Q..
051ba030 : af 51 00 00 b0 51 00 00 b1 51 00 00 b2 51 00 00 : .Q...Q...Q...Q..
051ba040 : b3 51 00 00 b4 51 00 00 b5 51 00 00 b6 51 00 00 : .Q...Q...Q...Q..
051ba050 : b7 51 00 00 b8 51 00 00 b9 51 00 00 00 00 00 00 : .Q...Q...Q......
051ba060 : ... zeroes ...
051bb000 : (end)
```

This shows the page numbers for the Stream Directory. The first few page numbers
are 0x51a3, 0x51a4, 0x51a5, etc.

Page 0x51a3 contains the beginning of the Stream Directory:

```text
051a3000 : 2b 09 00 00 e0 66 01 00 db 00 00 00 64 6d ed 00 : +....f......dm..
051a3010 : a2 f1 38 00 70 5a 3a 00 00 00 00 00 00 00 00 00 : ..8.pZ:.........
051a3020 : 7d d1 07 00 b4 38 00 00 4c 8d 01 00 40 fb 00 00 : }....8..L...@...
051a3030 : bc 9d 24 00 18 01 00 00 48 00 00 00 28 01 00 00 : ..$.....H...(...
051a3040 : a4 49 02 00 1c 4d 00 00 9c 32 00 00 2c 3a 00 00 : .I...M...2..,:..
051a3050 : 44 8e 00 00 1c 36 00 00 18 51 00 00 14 37 00 00 : D....6...Q...7..
051a3060 : d4 1f 00 00 b0 34 00 00 b8 2e 00 00 60 4e 00 00 : .....4......`N..
051a3070 : 00 35 00 00 3c 23 00 00 94 4c 00 00 78 3c 00 00 : .5..<#...L..x<..
051a3080 : 80 23 00 00 ec 7d 00 00 78 2c 00 00 ec 28 00 00 : .#...}..x,...(..
051a3090 : 2c 1c 00 00 e4 48 00 00 90 c8 00 00 80 77 02 00 : ,....H.......w..
051a30a0 : b4 15 00 00 d4 2c 00 00 a4 20 00 00 80 d4 00 00 : .....,... ......
051a30b0 : e0 3c 00 00 f4 22 00 00 14 78 00 00 04 36 00 00 : .<..."...x...6..
051a30c0 : f4 2a 00 00 dc 34 00 00 e4 10 00 00 40 44 00 00 : .*...4......@D..
051a30d0 : 7c 25 00 00 00 34 00 00 a8 10 00 00 50 42 02 00 : |%...4......PB..
051a30e0 : 58 58 00 00 08 eb 00 00 4c 6d 00 00 dc 2e 00 00 : XX......Lm......
051a30f0 : ec 3e 00 00 94 54 00 00 44 1f 00 00 44 c6 00 00 : .>...T..D...D...
051a3100 : 08 25 00 00 40 58 00 00 d8 3e 00 00 fc f4 00 00 : .%..@X...>......
051a3110 : bc 69 00 00 bc b8 00 00 7c 21 00 00 f4 39 00 00 : .i......|!...9..
051a3120 : 74 fd 03 00 00 43 00 00 dc 39 00 00 64 4f 00 00 : t....C...9..dO..
051a3130 : fc 77 00 00 98 70 00 00 6c 60 00 00 c4 15 00 00 : .w...p..l`......
051a3140 : 54 2b 00 00 2c e5 01 00 c8 2e 00 00 e0 77 00 00 : T+..,........w..
051a3150 : e0 25 00 00 6c 33 00 00 1c 36 00 00 34 23 00 00 : .%..l3...6..4#..
051a3160 : 7c 58 00 00 ec 10 00 00 a0 38 00 00 3c 3a 00 00 : |X.......8..<:..
051a3170 : d4 09 00 00 58 e6 00 00 40 22 00 00 ec 3d 00 00 : ....X...@"...=..
051a3180 : 3c 1d 00 00 20 57 00 00 e4 31 00 00 e0 45 00 00 : <... W...1...E..
051a3190 : e0 61 00 00 c4 b6 00 00 04 09 00 00 ac 5c 00 00 : .a...........\..
051a31a0 : fc 54 00 00 64 26 00 00 e0 20 00 00 30 47 00 00 : .T..d&... ..0G..
051a31b0 : 28 22 00 00 2c 6e 00 00 64 21 00 00 9c 2c 00 00 : ("..,n..d!...,..
051a31c0 : d4 4c 00 00 74 33 00 00 28 12 00 00 9c 18 00 00 : .L..t3..(.......
051a31d0 : ec 31 00 00 dc 1f 00 00 90 2d 00 00 40 30 00 00 : .1.......-..@0..
051a31e0 : 80 10 00 00 0c 3b 00 00 2c 22 00 00 a0 29 00 00 : .....;..,"...)..
051a31f0 : a8 3d 00 00 a0 25 00 00 7c 2c 00 00 54 50 00 00 : .=...%..|,..TP..
```

At file offset 0x51a3000 we see the `uint32_t` value 0x92b (2347), which is
`num_streams`. The next 2347 values are the contents of `stream_sizes`. Here are
the first few entries from `stream_sizes`:

File Offset | Stream | `stream_sizes[i]` | Num pages needed for this stream | Description
------------|--------|-------------------|----------------------------------|------------
051a3004    | 0      | 000166e0          | 00017                            | "Old Stream Directory"
051a3008    | 1      | 000000db          | 00001                            | [PDB Stream](pdb_stream.md)
051a300c    | 2      | 00ed6d64          | 00ed7                            | [TPI Stream](tpi_stream.md)
051a3010    | 3      | 0038f1a2          | 00390                            | [DBI Stream](dbi.md)
051a3014    | 4      | 003a5a70          | 003a6                            | [IPI Stream](ipi.md)

The "num pages needed for this stream" is computed by dividing
`stream_sizes[i]` by `page_size` and rounding up.

To find `stream_pages`, we add `num_streams * sizeof(uint32_t)` to the offset of
`stream_sizes`, which is `0x51a3004 + 0x92b * 4 = 0x51a54b0`. At that location
we see:

```text
051a54b0 : 04 00 00 00 05 00 00 00 06 00 00 00 07 00 00 00 : ................
051a54c0 : 0a 00 00 00 a0 50 00 00 a1 50 00 00 a2 50 00 00 : .....P...P...P..
051a54d0 : a3 50 00 00 a4 50 00 00 a5 50 00 00 a6 50 00 00 : .P...P...P...P..
051a54e0 : a7 50 00 00 a8 50 00 00 a9 50 00 00 aa 50 00 00 : .P...P...P...P..
051a54f0 : ab 50 00 00 ac 50 00 00 ad 50 00 00 ae 50 00 00 : .P...P...P...P..
051a5500 : af 50 00 00 b0 50 00 00 b1 50 00 00 a2 51 00 00 : .P...P...P...Q..  <-- stream 1 pages
051a5510 : 85 50 00 00 2b 3c 00 00 2c 3c 00 00 2d 3c 00 00 : .P..+<..,<..-<..
051a5520 : 2e 3c 00 00 2f 3c 00 00 30 3c 00 00 31 3c 00 00 : .<../<..0<..1<..
051a5530 : 32 3c 00 00 33 3c 00 00 34 3c 00 00 35 3c 00 00 : 2<..3<..4<..5<..
051a5540 : 36 3c 00 00 37 3c 00 00 38 3c 00 00 39 3c 00 00 : 6<..7<..8<..9<..
051a5550 : 3a 3c 00 00 3b 3c 00 00 3c 3c 00 00 3d 3c 00 00 : :<..;<..<<..=<..
```

As seen from the table above, the first 0x17 values store the pages for the Old
Stream Directory. The Old Stream Directory contains data from the previous
version of the stream (before the most recent commit). The information in the
Old Stream Directory is not relevant for decoding the current version of the
PDB, so we ignore it.

If we seek ahead by `0x17 * 4` bytes to stream offset 0x51a550c, then we will
find the pages for stream 1, the "PDB Stream". Stream 1 is small (0xdb bytes),
so it occupies only a single stream. That page number is 0x51a2.

At page 0x51a2 we find:

```text
051a2000 : 94 2e 31 01 3d 27 f1 8e 02 00 00 00 63 b7 fc 1c : ..1.='......c...
051a2010 : 72 76 f1 91 c2 b1 f0 28 b6 29 60 bb 63 00 00 00 : rv.....(.)`.c...
051a2020 : 2f 4c 69 6e 6b 49 6e 66 6f 00 2f 54 4d 43 61 63 : /LinkInfo./TMCac
051a2030 : 68 65 00 2f 6e 61 6d 65 73 00 2f 55 44 54 53 52 : he./names./UDTSR
051a2040 : 43 4c 49 4e 45 55 4e 44 4f 4e 45 00 73 6f 75 72 : CLINEUNDONE.sour
051a2050 : 63 65 6c 69 6e 6b 24 31 00 73 6f 75 72 63 65 6c : celink$1.sourcel
051a2060 : 69 6e 6b 24 32 00 73 6f 75 72 63 65 6c 69 6e 6b : ink$2.sourcelink
051a2070 : 24 32 00 65 6d 62 65 64 73 70 64 00 73 72 63 73 : $2.embedspd.srcs
051a2080 : 72 76 00 07 00 00 00 0e 00 00 00 01 00 00 00 f4 : rv..............
051a2090 : 05 00 00 01 00 00 00 01 00 00 00 2c 00 00 00 28 : ...........,...(
051a20a0 : 09 00 00 1a 00 00 00 26 09 00 00 13 00 00 00 07 : .......&........
051a20b0 : 00 00 00 46 00 00 00 2a 09 00 00 00 00 00 00 05 : ...F...*........
051a20c0 : 00 00 00 0a 00 00 00 06 00 00 00 5c 00 00 00 29 : ...........\...)
051a20d0 : 09 00 00 00 00 00 00 dc 51 33 01 00 00 00 00 00 : ........Q3......
051a20e0 : ... zeroes ...
051a3000 : (end)
```

This is the contents of Stream 1, the PDB Stream.

# References
 
* [LLVM: The MSF File Format](https://releases.llvm.org/8.0.0/docs/PDB/MsfFile.html)
* [`msf.h`](https://github.com/microsoft/microsoft-pdb/blob/master/PDB/include/msf.h)
