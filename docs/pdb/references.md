# References

This document intends to bring together many of the existing resources that
describe PDB, and to describe the file format at a level of detail that it could
be implemented without reference to other documents.

The primary sources for this document:

* The LLVM project provides The PDB File Format. This web site covers some of
  the basics of PDB, enough to write a simple PDB reader for a subset of the
  information in PDBs. It is nowhere near comprehensive. Some of the information
  in this web site is incorrect.

* Microsoft published a GitHub repository:
  <https://github.com/microsoft/microsoft-pdb>. This contains a snapshot of some
  of the PDB sources, including header files that describe data structures and a
  pdbdump tool. Microsoft has not updated this repo in years, and recently
  marked it as "archived." It does not accept pull requests.

  The following paths with the `microsoft-pdb` repo contain useful information:

  + `langapi/include/cvinfo.h` – Many essential definitions for CodeView debug info, including the `LF_*` and `S_*` constants and structures which define the fixed-size portion of many type and symbol records.
  + `langapi/include/cvexefmt.h` – CodeView definitions for executables.
  + `langapi/include/pdb.h` – The API for the PDB reader/writer library. This is an implementation detail and does not contain descriptions of the file structures. However, it is useful for seeing the operations defined by the implementation.
  + `PDB/src/tools/cvdump/dumppdb.cpp` – Dumps information in PDBs.
  + `PDB/dbi/pdb.cpp` – The PDB reader/writer library.
  + `PDB/dbi/dbi.cpp` and `dbi.h` – Code which can read the Debug Information (DBI) Stream.
  + `PDB/dbi/gsi.cpp` and `gsi.h` – Code which can read the Global Symbol Index (GSI) Stream.
  + `PDB/dbi/mod.cpp` and `mod.h` – Code which can read Module Information Streams (aka `mod` or `modi` streams).

  As a resource, this repository is useful but incomplete. It does not describe
  all of the information in PDBs, and it does not describe invariants or
  relationships. Still, it has been a valuable resource for the external
  developers who created the LLVM web site, and various other tools for reading
  PDBs.

* The <https://github.com/willglynn/pdb> repository. This is an open source
  library (MIT and Apache dual-licensed), implemented in Rust, which can read
  some information from PDBs.

* The <https://github.com/MolecularMatters/raw_pdb> repository. This is an open
  source library (BSD 2-clause), implemented in C++, which can read some
  information from PDBs.

* [LLVM-PDB] "The PDB File Format": Describes the PDB File Format, using
  information from public Microsoft documentation.

* [MS-PDB] github.com/microsoft/microsoft-pdb: Contains some documentation on
  the PDB file format. Also contains source code for CodeView records, the MSPDB
  library, and header files that list many important PDB/CodeView definitions.

* [CODEVIEW] The CodeView specification for type records and symbol records.
  This information has been published by Microsoft previously, but is not
  currently offered.

* <https://en.wikipedia.org/wiki/CodeView>

* In [MS-PDB], the `cvinfo.h` header contains all of the relevant definitions, but
  does not document their semantics.

* LLVM presentation on CodeView: CodeView in LLVM

* Microsoft published `MS_Symbol_Type_v1.0.pdf` in 2004, which contains extensive documentation on the CodeView type and symbol records. It has been archived repeatedly on the Internet.
  * Example: <https://github.com/mfichman/jogo/blob/master/notes/MS_Symbol_Type_v1.0.pdf>
