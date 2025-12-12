# Program Database (PDB) Files

This directory describes Program Database (PDB) files. PDBs contain a variety of
information about executable images, including:

* CodeView debugging information, 
* the list of modules (OBJ files)
* linker sections and subsections
* source file references
* NatVis debugger extensions
* etc.

## Container and Streams

PDB files are stored in a _container_ format. Containers provide the abstraction
of _streams_, which are similar to files within a ZIP archive. Most streams are
identified by number, not by file name. Some streams are identified by name; the
mapping from stream name to stream number is stored in the 
[PDB Information Stream](./pdbi_stream.md).

The [Multi-Stream File (MSF) Container Format](msf.md) is used by most compilers
and debuggers. This document is essential for understanding PDBs at the lowest
level.

The [MSF Compressed (MSFZ) Container Format](msfz.md) is a container format that
is optimized for storage efficiency. It provides the same stream abstraction as
the MSF container format but uses a different on-disk representation.

## PDB Information Stream - Fixed Stream 1

The [PDB Information Stream](pdbi_stream.md) contains important information
about the entire PDB, such as the binding key (GUID and age), the named streams
table, and the PDB version. Most programs that read PDBs will read the PDB
Information Stream as one of the first steps.

The PDB Information Stream also contains a table of _named streams_. Named
streams allow tools to insert arbitrary information into PDBs, such as NatVis
files, source code, SourceLink metadata, etc.

The PDB Information Stream is always stream 1.

## Debug Information Stream (DBI) - Fixed Stream 3

The [Debug Information Stream (DBI)](dbi.md) is a central data structure for
debugging. It is always stream 3.

The DBI contains:

* the list of all modules (OBJ files) that were linked into the program
* the list of all sources files that were compiled

- [DBI Modules Substream](dbi_modules.md) - Lists all of the modules (OBJ files)
  that were linked into this program. Many data structures refer to modules by
  index. This table defines the meaning of those module indexes.

- [DBI Sources Substream](dbi_sources.md) - DBI subsection listing source files
  used when compiling each module

- [Optional Debug Streams](dbi_opt_debug.md) - Contains a set of optional debug
  streams. These describe [fixups](dbi_fixups.md), exception data, COFF section
  headers, and a variety of other data.

- [Section Map and Contributions](dbi_sections.md) - Describes the contributions
  (fragments) of each module and how they map to the executable image sections.

## Global Symbols Stream

The [Global Symbols Stream (GSS)](globals.md) contains information about symbols
that are used across the entire program, or are exported by an executable (DLL
exports).

The stream number of the Global Symbol Stream (and its related index streams)
can be found in the Debug Information Stream Header.

The debug records (symbols) stored within the Global Symbols Stream are
described in [CodeView Debugging Records](../codeview/codeview.md).

## Module Streams

[Module Streams](module_stream.md) describes the code and data within each
module (OBJ file). Each module (OBJ file) has its own module stream. Module
streams are optional.

Use the [DBI Modules Substream](dbi_modules.md) to find the list of modules and
their stream indexes.

The debug records (symbols) stored within Module Streams are described in
[CodeView Debugging Records](../codeview/codeview.md).

## Type Stream (TPI) - Fixed Stream 2

The [Type Stream (TPI)](tpi_stream.md) describes the TPI Stream, which contains
a sequence of related type records.

## Items Stream (IPI) - Fixed Stream 4

The [IPI Stream](ipi_stream.md) describes the IPI Stream, which contains various
ids instruction addresses to source locations

## Names Streams - `/names`

The [Names Stream](names_stream.md) describes the `/names` stream, which
contains a set of names (mostly file names) that are referenced by many data
structures. Several record types within the IPI point into the Names Stream.
The integers which point into the Names Stream use the `NameIndex` alias.

## Named Streams

PDB files may also contain named streams, which are identified by name and are
listed within the Named Stream Table within the
[PDB Information Stream](./pdbi_stream.md).

* NatVis streams, which contain XML type descriptions for visualizing types
  during debugging. These are identified by file name as named streams, e.g.
  `my_types.natvis`.

* Source file contents (not merely source file names but their entire contents)
  may be stored within streams. These are identified by file name as named
  streams, e.g. `my_generated_code.cpp`.

* Source code linking information, which allows debuggers to find
  the correct source code for a given binary. See [Source Link](https://github.com/dotnet/designs/blob/main/accepted/2020/diagnostics/source-link.md#source-link-file-specification).  Source Link information is stored in
  a named stream.

## Hash Algorithms

[Hash Algorithms](hashing.md) describes hash functions used by several PDB
tables and records

## Relationships

[Relationships](relationships.md) describes values that "point" from one
data structure into another. These relationships must be preserved when PDBs are
modified.

## Mini PDB (obsolete)

[mini_pdb.md](mini_pdb.md) - Mini PDBs (fast PDBs) generated with
`/DEBUG:FASTLINK`
