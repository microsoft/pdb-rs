
# Terminology

age
: A `uint32_t` value which is initialized to 1, and is incremented whenever an executable and a PDB are modified as related pair. Tools which read PDBs, such as debuggers, should verify that the `age` stored in an executable matches the `age` stored in the PDB.

binding key
: The combination of a GUID (`unique_id`) and `age` value, which uniquely identifies a PDB and links it to the executable that it describes.

COFF, PE/COFF
: The Common Object File Format, which describes the structure of compiler outputs, linker outputs, executables. This is the executable format used by Windows. Also known as PE/COFF (Portable Executable).

contribution
: A fragment of a module that was included into an executable by the linker, such as a function, vtable, global variable, etc. Each module typically contains many contributions to the final executable.

DIA, MS-DIA
: Microsoft Debug Interface Access, a library for accessing symbolic debugging information within PDBs and other sources. Used by tools such as debuggers.

executable
: A PE/COFF executable, either a `*.exe` or `*.dll` file, which has been produced by the linker and can be loaded and executed.

[Global Symbol Stream (GSS)](globals.md)
: A sequence of symbol records that describe global functions, types, annotations, etc. This is an "index" of the per-module symbols, or in PDBs that have been stripped, this is the only description of global symbols.

interval
: A fixed-size group of pages in the MSF, which have 2 pages reserved for the Free Page Maps (FPM1 and FPM2). See [Free Page Map](msf.md#free-page-map).

module index
: A `uint16_t` value that identifies a module within a PDB. The order of the records in the [DBI Modules Substream](dbi_modules.md) determines the range and meaning of the module index. Module indexes are 0-based. The value 0xffff is reserved and cannot identify a valid module.

MSF
: Multi-Stream File, a file container format which can contain multiple internal file streams.

Free Page Map (FPM)
: A set of pages that contain a bitmap. The elements of the bitmap correspond to pages, and indicate which pages are free (1) or allocated (0). See the MSF section.

`ItemId`
: A 32-bit integer which identifies a record in the [IPI Stream](ipi.md).

module
: An object file that was linked into an executable by the linker. Also called a compiland or a translation unit. This sense of "module" is not related to Rust modules or C++20 modules.

`NameIndex`
: A 32-bit integer which identifies a string stored in the [Names Stream](names_stream.md).

page
: The unit of allocation / storage of data within a PDB/MSF file. All pages within a PDB file are the same size and are stored at PDB file offsets that are a multiple of the page size. The most common page size is 4096 bytes.

stream
: A logical sequence of bytes stored within a PDB/MSF file. Analogous to a file stored within a ZIP or TAR file.

Type Database, TPI Stream
: A stream which contains records that describe C/C++ types. Also known as TPI Stream.

`TypeIndex`
: A 32-bit integer which identifies either a primitive type (such as `unsigned short`) or identifies a type record defined in the (TPI Stream)[tpi_stream.md].

UDT
: A _user-defined type_, such as a `class`, `struct`, or `union`.
