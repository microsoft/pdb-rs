# Program Database (PDB) tool

This is a simple tool for reading Program Database files. It uses the
[`ms-pdb`](https://crates.io/crates/ms-pdb) crate to read and write PDB.

This tool is published as a separate crate in order to minimize the dependencies of the `ms-pdb`
crate.

## Installation

```batch
cargo install pdbtool
```

## Examples

```
pdbtool dump hello_world.pdb streams
```

produces:

```text
Stream #     0 : (size          0) OldStreamDir
Stream #     1 : (size        665) PDB
Stream #     2 : (size   15341432) TPI
Stream #     3 : (size    1668945) DBI
Stream #     4 : (size    1871280) IPI
Stream #     5 : (size          0) Named { name: "/LinkInfo" }
Stream #     6 : (size          0) Named { name: "/TMCache" }
Stream #     7 : (size     140696) Named { name: "/names" }
[...]
```

Use `pdbtool --help` for a list of commands. Use `pdbtool dump --help` for help with the `dump`
command, which has many subcommands.

## Help

```batch
pdbtool --help
```

```text
Usage: pdbtool [OPTIONS] <COMMAND>

Commands:
  add-src     Adds source file contents to the PDB. The contents are embedded directly within the PDB. WinDbg and Visual Studio can both extract the source files
  copy        Copies a PDB from one file to another. All stream contents are preserved exactly, byte-for-byte. The blocks within streams are laid out sequentially
  test        
  dump        
  save        
  find        Searches the DBI Section Contributions table
  find-name   Searches the TPI Stream for a given type
  counts      Counts the number of records and record sizes for a given set of PDBs
  hexdump     Dumps part of a file (any file, not just a PDB) as a hex dump. If you want to dump a specific stream, then use the `dump <filename> hex` command instead
  pdz-encode  
  help        Print this message or the help of the given subcommand(s)

Options:
      --quiet       Reduce logging to just warnings and errors in `mspdb` and `pdbtool` modules
      --verbose     Turn on debug output in all `mspdb` and `pdbtool` modules. Noisy!
      --timestamps  Show timestamps in log messages
      --tracy       Connect to Tracy (diagnostics tool). Requires that the `tracy` Cargo feature be enabled
  -h, --help        Print help
```

## Help - dump

The `dump` subcommand takes a path to a PDB file as its next argument, followed by a subcommand,
as seen in the example above.

```batch
pdbtool dump --help
```

```text
Usage: pdbtool dump [OPTIONS] <PDB> <COMMAND>

Commands:
  names                
  globals              
  tpi                  Dump the Type Stream (TPI)
  ipi                  Dump the Id Stream (TPI)
  dbi                  Dump DBI header
  dbi-enc              Dump DBI Edit-and-Continue Substream
  dbi-type-server-map  
  gsi                  Global Symbol Index. Loads the GSI and iterates through its hash records. For each one, finds the symbol record in the GSS and displays it
  psi                  Public Symbol Index. Loads the PSI and iterates through its hash records. For each one, finds the symbol record in the GSS and displays it
  modules              
  streams              Dump the Stream Directory
  lines                Dumps C13 Line Data for a given module
  sources              Dump the DBI Stream - Sources substream
  section-map          
  section-contribs     Dump section contributions (quite large!)
  pdbi                 Dump the PDB Info Stream
  module-symbols       Displays the symbols for a specific module
  hex                  Dump the contents of a stream, or a subsection of it, using a hexadecimal dump format. By default, this will only show a portion of the stream; use `--len` to increase it
  help                 Print this message or the help of the given subcommand(s)

Arguments:
  <PDB>  The PDB to dump

Options:
      --lines-like-cvdump  
  -h, --help               Print help
```

## Converting PDB files to Compressed PDB (PDZ/MSFZ)

> **WARNING** **WARNING** **WARNING** **WARNING** **WARNING**
>
> This tool provides an _experimental_ capability for compressing PDBs into a new PDB container
> format, called MSFZ. See the [MSFZ Container Specification](https://github.com/microsoft/pdb-rs/blob/main/msfz/src/msfz.md).
> This container specification is _experimental_ and is not guaranteed to be stable. It may change
> at any time; there is no guarantee of compatibility, stability, or reliability. It is defined
> for the purposes of experimentation.
>
> **WARNING** **WARNING** **WARNING** **WARNING** **WARNING**

MSFZ is intended to reduce development costs, by defining a compression format for PDB, which
still allows tools (such as debuggers) to read from compressed PDBs by decompressing only those
parts of the PDB that are needed. This avoids the need to decompress the entire PDB. See the
specification (linked above) for details.

To convert a PDB file to a compressed PDB:

```batch
pdbtool pdz-encode hello_world.pdb d:\compressed_pdbs\hello_world.pdb
```

This reads the `hello_world.pdb`, compresses its contents, and writes it to
`d:\compressed_pdbs\hello_world.pdb`. Recent versions of the Windows Debugger
(WinDbg) can directly read compressed PDB files. _It cannot be over-emphasized
that this support is experimental, and subject to change without notice._

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

