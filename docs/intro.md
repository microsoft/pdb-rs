# Introduction

## Publicly-sourced information

All of the information described in this document comes from existing documents
and source code published by Microsoft. This document does not describe any
trade secrets or confidential intellectual property of Microsoft; all of it has
been made public by Microsoft in other forms.

## Goals and Scope

The PDB file format is an essential part of development workflows for Microsoft
and the developer community that uses Microsoft platforms. Unfortunately,
documentation for PDB is inconsistent and sparse. This document is intended to
describe the structure of PDBs, clearly and unambiguously, to a degree that it
can serve as a primary reference for implementation of tools that work with
PDBs.

This document should be sufficient for developers to implement _some_
PDB-related tools without reference to other documents or sources.
Unfortunately, that level of detail will not be feasible for some of the debug
records, but nonetheless the aim of this document is to be as comprehensive as
possible.

These goals are in scope for this document:

* Describing the PDB/MSF container file format, including how streams are stored
  on disk, how the 2-phase commit protocol works, how the Free Page Map works,
  and how to correctly create or modify a PDB.
* Describing the well-known streams in the PDB. Each stream should be described
  at a level of detail sufficient for implementing a decoder or encoder, without
  reference to other implementations.
* Describing (or referencing) the set of CodeView type and symbol records that
  are produced by current-generation MSVC and LLVM tools (circa 2023). LLVM is
  explicitly in-scope because Microsoft makes extensive use of LLVM-based
  compilers (Clang and Rust) and these compilers must be able to interoperate
  with MSVC when describing debug information.
* Describing the records that contain compiler flags (CFLAGS).
* Describing how NatVis files are embedded within PDBs.
* Describing how SourceLink information is embedded within PDBs.
* Describing the Module Information streams.

The following are not within the scope of this document:

* CodeView records for older versions of the MSVC toolset, such as 16-bit CPUs,
  IA64, MIPS, etc.
* "Portable PDBs" are out of scope. Portable PDBs use a very different file
  format, which is based on the .NET platform’s CIL metadata.
* "Compiler PDBs" (PDBs produced by the MSVC compiler) are out of scope. This
  document focuses only on "linker PDBs".

# Invariants

When possible, this document specifies invariants for file structures that it
specifies. For example, if a table of values must be sorted in increasing order,
then this will be called out:

> Invariant: The `foo_entries` table must be sorted in increasing order, using
> the `bar_offset` field.

For a PDB file to be well-formed (valid) all invariants must be met. If an
invariant is conditional then the invariant will give the condition that governs
it. For example:

> Invariant: If the `foo_stream` header is using the Paired Encoding, then the
> number of items in the `foo_items` table must be a multiple of 2.

Some invariants apply only to specific versions of the PDB file format, or to
specific versions of streams, substreams, etc. These version dependencies will
be explicitly called out in invariants.
