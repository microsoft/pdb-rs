# Program Database (PDB) Files

This directory describes Program Database (PDB) files. PDBs contain a variety of
information about executable images, including:

* CodeView debugging information, 
* the list of modules (OBJ files)
* linker sections and subsections
* source file references
* NatVis debugger extensions
* etc.

## Container

PDB files are stored in a _container_ format. Containers provide the abstraction
of _streams_, which are similar to files within a ZIP archive. Streams are
identified by number, not by file name.

The [Multi-Stream File (MSF) Container Format](msf.md) is used by most compilers
and debuggers. This document is essential for understanding PDBs at the lowest
level.

The [MSF Compressed (MSFZ) Container Format](msfz.md) is a container format that
is optimized for storage efficiency. It provides the same stream abstraction as
the MSF container format but uses a different on-disk representation.

## PDB Information Stream

The [PDB Information Stream](pdbi_stream.md) contains important information
about the entire PDB, such as the identity (binding) key, the named streams
table, and the PDB version. Most programs that read PDBs will read the PDB
Information Stream as one of the first steps.

The PDB Information Stream also contains a table of _named streams_. Named
streams allow tools to insert arbitrary information into PDBs, such as NatVis
files, source code, SourceLink metadata, etc.

## Debug Information Stream (DBI)

The [Debug Information Stream (DBI)](dbi.md) is a central data structure for
debugging. The DBI contains:

* the list of all modules (OBJ files) that were linked into the program
* the list of all sources files that were compiled

- [DBI Modules Substream](dbi_modules.md) - Lists all of the modules (OBJ files)
  that were linked into this program. Many data structures refer to modules by
  index. This table defines the meaning of those module indexes.

- [DBI Sources Substream](dbi_sources.md) - DBI subsection listing source files
  used when compiling each module

- [Optional Debug Substreams](dbi_opt_debug.md) - Contains a set of optional
  debug substreams. These describe [fixups](dbi_fixups.md), exception data, COFF
  section headers, and a variety of other data.

- [Section Map and Contributions](dbi_sections.md) - Describes the contributions
  (fragments) of each module and how they map to the executable image sections.

## Global Symbols

The [Global Symbols Stream (GSS)](globals.md) contains information about symbols
that are used across the entire program, or are exported by an executable (DLL
exports).

## CodeView debug information

Debug information records, stored in Module Streams and in the Global System
Stream, are described in [CodeView Debugging Records](../codeview/codeview.md).

## Names Streams

The [Names Stream](names_stream.md) describes the `/names` stream, which contains
a set of names (mostly file names) that are referenced by many data structures.

## Module Streams

[Module Streams](module_stream.md) describes the code and data within each
module (OBJ file). Each module (OBJ file) has its own module stream. Module
streams are optional.

Use the [DBI Modules Substream](dbi_modules.md) to find the list of modules and
their stream indexes.

## Type Stream (TPI)

The [Type Stream (TPI)](tpi_stream.md) describes the TPI Stream, which contains
a sequence of related type records.

## Items Stream (IPI)

The [IPI Stream](ipi.md) describes the IPI Stream, which contains various ids
instruction addresses to source locations

## Hash Algorithms

- [Hash Algorithms](hashing.md) - Describes hash functions used by several PDB
  tables and records

## Relationships

[Relationships](relationships.md) describes values that "point" from one
data structure into another. These relationships must be preserved when PDBs are
modified.

## Mini PDB (obsolete)

[mini_pdb.md](mini_pdb.md) - Mini PDBs (fast PDBs) generated with `/DEBUG:FASTLINK`
