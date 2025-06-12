# PDB Design Doc

This document describes the PDB file format. It describes the structure of the
PDBs with sufficient detail to read, create, and modify PDBs. It describes the
relationships between different sections of the PDB. It also describes sources
of non-determinism, and a set of normalization rules that may be applied to PDBs
to reach determinism.

## Non-authoritative

This document (and all related documents in this repository) are
**non-authoritative** decriptions of PDB, CodeView, and the data structures used
by them.

## Publicly-sourced information

With the exception of MSFZ, all of the information described in this document
comes from existing documents and source code published by Microsoft. This
document does not describe any trade secrets or confidential intellectual
property of Microsoft; all of it has been made public by Microsoft in other
forms.

## Goals and Scope

The PDB file format is an essential part of development workflows for Microsoft
and the developer community that uses Microsoft platforms. Unfortunately, there
is little internal documentation for PDB, and even less public documentation.
This document is intended to describe the structure of PDBs, clearly and
unambiguously, to a degree that it can serve as a primary reference for
implementation of tools that work with PDBs.

This document should (eventually) be sufficient for developers to implement
PDB-related tools without reference to other documents or sources.
Unfortunately, that level of detail will not be feasible for some of the debug
records, but nonetheless the aim of this document is to be as comprehensive as
possible.

These goals are in scope for this document:

* Describing the PDB/MSF container file format, including how streams are stored
  on disk, how the 2-phase commit protocol works, how the Free Page Map works,
  and how to correctly create or modify a PDB.
* Describing the set of well-known streams in the PDB. Each stream should be
  described at a level of detail sufficient for implementing a decoder or
  encoder, without reference to other implementations.
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

# Determinism and Normalization 

A goal of this document is to achieve determinism in PDBs. There are many
benefits to determinism (aka reproducible builds). See:
[Reproducible builds on Wikipedia](https://en.wikipedia.org/wiki/Reproducible_builds).

When a compiler produces an executable (assuming the compiler is deterministic),
the corresponding PDB should also be deterministic. Currently, this is not true
for the MSVC linker. The MSVC linker uses multiple threads to accelerate linking
(and compilation, when LTCG is active), and linker performance is a vital part
of the development workflow for Microsoft and its customers.

DevDiv conducted their own investigation into PDB determinism. Their conclusion
was that the best approach was to continue to allow the linker to produce
non-deterministic PDBs, and then to add a post-processing step that constructed
a deterministic PDB from non-deterministic inputs. This would allow the linker
to continue to work as it does, and would have only a modest performance impact.
It is also much easier to reconstruct determinism when all input information is
available at once, rather than trying to achieve determinism incrementally.

Throughout this document, as the file structures are described, each section
will also specify what rules could be used to impose a deterministic order on a
given file structure. For example, a table might contain records that could
occur in any order. A determinism requirement would state one arbitrary sorting
order that could be applied to the table. The sorting order does not affect the
semantics of the information, but it does remove a degree of freedom from its
encoding.

These determinism requirements will be called out in each section that describes
a file structure. Again, determinism requirements are stronger than invariants.
For example, let’s assume we are dealing with a hash table, where each hash
record contains a hash code and a reference count field. Our correctness
requirement only governs the ordering of the records with respect to their hash
codes:

> Invariant: The `hash_records` array must be sorted in increasing order by hash
> code.

Unfortunately, this leaves us with an unwanted degree of freedom because two
consecutive hash records could have the same hash code but have different
reference counts. We specify this as a determinism requirement:

> Determinism: The `hash_records` array must also be sorted in increasing order
> by reference count.
