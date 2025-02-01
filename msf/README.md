# MSF container layer for Program Database (PDB) library

This crate contains code for reading, creating, and modifying PDB files that use
the MSF container format. Currently, all PDBs produced by Microsoft tools use
the MSF container format.
  
This is a lower-level building block for PDBs. Most developers should use the
[`ms-pdb`](https://crates.io/crates/ms-pdb) crate, instead of directly using the `ms-pdb-msf` crate.
The `ms-pdb-msf` crate is published separately to aid in minimizing dependencies and enforcing
good layering.

## All information in this implementation is based on publicly-available information

This implementation is based solely on public sources that describe the PDB and
MSF data structures. This repository does not contain any confidential Microsoft
intellectual property.

## **THIS IMPLEMENTATION IS NOT AUTHORITATIVE AND IS NOT A REFERENCE IMPLEMENTATION**

This implementation is **NOT** an authoritative reference. It may contain
defects or inaccuracies. As the `LICENSE` states, this implementation is
provided "as is", without warranty of any kind. Specifically, this
implementation **DOES NOT** make any guarantees about compatibility or
interoperability with any other toolset, including (but not limited to)
Microsoft Visual C++ (MSVC) and Clang.

The authors of this effort may make a good-faith effort to fix bugs, including
bugs discovered by the authors or the community. However, as the license states,
this repository is provided "as is" and there is absolutely no obligation or
expectation on levels of service for the implementation provided in this
repository.

## Contributing

This project welcomes contributions and suggestions. Most contributions require
you to agree to a Contributor License Agreement (CLA) declaring that you have
the right to, and actually do, grant us the rights to use your contribution. For
details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether
you need to provide a CLA and decorate the PR appropriately (e.g., status check,
comment). Simply follow the instructions provided by the bot. You will only need
to do this once across all repos using our CLA.

This project has adopted the
[Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the
[Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any
additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or
services. Authorized use of Microsoft trademarks or logos is subject to and must
follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must
not cause confusion or imply Microsoft sponsorship. Any use of third-party
trademarks or logos are subject to those third-party's policies.

## Repository

* <https://github.com/microsoft/pdb-rs>

## Contacts

* `sivadeilra` on GitHub
* Arlie Davis ardavis@microsoft.com
