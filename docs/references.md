# References

This document brings together many of the existing resources that describe PDB
and describes the file format at a level of detail sufficient for implementing
some tools.

The primary sources for this document:

* Microsoft published a GitHub repository:
  <https://github.com/microsoft/microsoft-pdb>. This contains a snapshot of some
  of the PDB sources, including header files that describe data structures and a
  pdbdump tool. Microsoft has not updated this repo in years, and recently
  marked it as "archived." It does not accept pull requests.

  The following paths with the `microsoft-pdb` repo contain useful information:

  + `langapi/include/cvinfo.h` – Many essential definitions for CodeView debug
    info, including the `LF_*` and `S_*` constants and structures which define
    the fixed-size portion of many type and symbol records.
  
  + `langapi/include/cvexefmt.h` – CodeView definitions for executables.
  
  + `langapi/include/pdb.h` – The API for the PDB reader/writer library. This is
    an implementation detail and does not contain descriptions of the file
    structures. However, it is useful for seeing the operations defined by the
    implementation.
  
  + `PDB/src/tools/cvdump/dumppdb.cpp` – Dumps information in PDBs.
  
  + `PDB/dbi/pdb.cpp` – The PDB reader/writer library.
  
  + `PDB/dbi/dbi.cpp` and `dbi.h` – Code which can read the Debug Information
    (DBI) Stream.
  
  + `PDB/dbi/gsi.cpp` and `gsi.h` – Code which can read the Global Symbol Index
    (GSI) Stream.
  
  + `PDB/dbi/mod.cpp` and `mod.h` – Code which can read Module Information
    Streams (aka `mod` or `modi` streams).

  As a resource, this repository is useful but incomplete. It does not describe
  all of the information in PDBs and it does not describe invariants or
  relationships. Still, it has been a valuable resource for the external
  developers who created the LLVM web site, and various other tools for reading
  PDBs.

* The LLVM Project provides [The PDB File Format](https://llvm.org/docs/PDB/index.html).
  This web site covers some of the basics of PDB, enough to write a simple PDB
  reader for a subset of the information in PDBs. It is not comprehensive. Some
  of the information in this web site is incorrect.

* The <https://github.com/willglynn/pdb> repository. This is an open source
  library (MIT and Apache dual-licensed), implemented in Rust, which can read
  some information from PDBs.

* The <https://github.com/MolecularMatters/raw_pdb> repository. This is an open
  source library (BSD 2-clause), implemented in C++, which can read some
  information from PDBs.

* The [microsoft-pdb](https://github.com/microsoft/microsoft-pdb) repository.
  Contains documentation and implementation for PDB and CodeView. In many ways,
  this is the most authoritative public _implementation_ resource for PDB,
  but it is not a document and not a reference. The repository was provided as
  a resource by Microsoft, primarily as an aid for the LLVM Project. The source
  code in it is not complete (not buildable) and the repository has been
  archived.

  In the `microsoft-pdb` repository, the `cvinfo.h` header contains many
  CodeView definitions but does not document their semantics.

* [CODEVIEW] The CodeView specification for type records and symbol records.
  This information has been published by Microsoft previously, but is not
  currently offered.

* <https://en.wikipedia.org/wiki/Program_database>

* [CodeView, the MS debug info format, in LLVM](https://llvm.org/devmtg/2016-11/Slides/Kleckner-CodeViewInLLVM.pdf)

  (Reid Kleckner) Describes the progress and status of the LLVM Project's effort to support
  CodeView and PDB.

  [Reid's YouTube talk at 2016 LLVM Developers' Meeting](https://www.youtube.com/watch?v=5twzd06NqGU)

* Microsoft published `MS_Symbol_Type_v1.0.pdf` in 2004, which contains
  extensive documentation on the CodeView type and symbol records. It has been
  archived repeatedly on the Internet.

  * Example: <https://github.com/mfichman/jogo/blob/master/notes/MS_Symbol_Type_v1.0.pdf>
