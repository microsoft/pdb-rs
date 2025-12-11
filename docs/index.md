# PDB and CodeView

This document describes the PDB file format. It describes the structure of the
PDBs with sufficient detail to read, create, and modify PDBs for some purposes.
It describes the relationships between different sections of the PDB. It also
describes sources of non-determinism, and a set of normalization rules that may
be applied to PDBs to reach determinism.

This document is a **non-authoritative** decriptions of PDB, CodeView, and the
data structures used by them. It is the synthesis of public information drawn from
many sources.

## Contents

- [Introduction](intro.md)
- [Terminology](terminology.md) - Lists terms used within this specification.
- [Data Types](data_types.md) - Data types that are used within this specification.
- [PDB](pdb/index.md) - Describes Program Database (PDB) files.
- [CodeView](codeview/codeview.md) - Describes debugging types and symbol records.
- [Determinism](./determinism.md) - Discusses deterministic (reproducible) builds.
- [References](references.md)

## Maintainers

* Arlie Davis - ardavis@microsoft.com
