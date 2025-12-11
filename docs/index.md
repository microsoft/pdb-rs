
- [Introduction](intro.md)
- [Terminology](terminology.md) - Lists terms used within this specification.
- [References](references.md)
- [Data Types](data_types.md) - Data types that are used within this specification.
- [MSF](pdb/msf.md) - Describes the Multi-Stream Format (MSF), which is the container format used by PDB. MSF allows PDB to organize its data structures into _streams_.
- [DBI](pdb/dbi.md) - Describes the Debug Info Stream, a central data structure which points to many other data structures.
- [Names Stream](pdb/names_stream.md) - Describes the `/names` stream, which contains a set of names (mostly file names) that are referenced by many data structures.
- Modules
  - [Module Stream](pdb/module_stream.md) - Describes the struct of Module Streams, which contain module symbols and C13 Line Data.
  - [C13 Line Data](codeview/line_data.md) - Describes C13 Line Data, which allows debuggers to translate from instruction streams (code) to source locations.
- [Global Symbols](pdb/globals.md) - Describes the Global Symbol Stream (GSS) and its indexes
- [IPI Stream](pdb/ipi.md) - Describes the IPI Stream, which contains various ids
instruction addresses to source locations
- [Relationships](pdb/relationships.md) - Describes values that "point" from one data structure into another. These relationships must be preserved when PDBs are modified.
- CodeView symbols and types
  - [Type Records](codeview/types/types.md) - Describes _type records_ that are stored in the TPI Stream.
  - [CodeView Number](codeview/codeview_number.md) - Describes the `Number` data type,
    which is used by type and symbol records.
  - [Primitive types](pdb/primitive_types.md) - Describes primitive types, such as `unsigned long`.
  - [TPI Stream](pdb/tpi_stream.md) - Describes the TPI Stream, which contains a sequence of related type records.
  - [Symbols](codeview/symbols/symbols.md) - Describes _symbol records_, which describe data types, procedures, and other language concepts.
- [Hash Algorithms](pdb/hashing.md) - Describes hash functions used by several hash table algorithms
