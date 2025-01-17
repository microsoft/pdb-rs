# PDB tools

This repository contains libraries and tools for working with Microsoft Program
Database (PDB) files. All of the code is in Rust.

* The `msf` crate contains code for reading, creating, and modifying PDB files
  that use the MSF container format. Currently, all PDBs produced by Microsoft
  tools use the MSF container format.
  
  This is a lower-level building block for PDBs, and most developers will never
  need to directly use the `msf` crate. Instead, they should use the `mspdb`
  crate.

* The `msfz` crate contains code for reading and writing PDB files that use the
  MSFZ container format. MSFZ is a new container format that is optimized for
  "cold storage"; PDB/MSFZ files cannot be modified in-place in the way PDB/MSF
  files can, but MSFZ files use an efficient form of compression that allows
  data to be accessed without decompressing the entire file. MSFZ is intended to
  be a format for storing PDBs, not for local development.

  Most developers will not need to use the `msfz` crate directly. Instead, they
  should use the `mspdb` crate.

* The `mspdb` crate supports reading, creating, and modifying PDB files. It
  builds on the `msf` and `msfz` crate. The `msf` and `msfz` crates provide the
  container format for PDB, but they do not contain any code for working with
  the contents of PDBs. That is the job of the `mspdb` crate -- it provides
  methods for reading specific PDB data structures, such as debug symbols, line
  mappings, module records, etc.

## Contributing

This project welcomes contributions and suggestions.  Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft 
trademarks or logos is subject to and must follow 
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.

## Contacts

* `sivadeilra` on GitHub
