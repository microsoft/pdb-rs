# CodeView

CodeView was originally a standalone debugger for early versions of Windows. The
name "Code View" now refers to the system of type records, symbol records, and
line records that have evolved from the original Code View debugger.

CodeView data structures may be stored in several places:

* Within PDB files that describe fully-linked executable images. These are
  produced by a linker.

* Within "compiler PDB" files, which describe a collection of OBJ files which
  have not yet been linked into an executable image. The MSVC `/Zi` compiler
  switch enables this.

* Within COFF OBJs in `.debug$S` sections. The MSVC `/Z7` compiler switch
  enables this.

This directory contains documents that describe these parts of Code View:

* [Primitive Types](primitive_types.md) describes how primitive types, such as `int`,
  `char`, `void` are represented

* [Types](types/types.md) describes how composed (non-primitive) types are
  represented

* [Items](items/items.md) describes how "items" are represented. Items are
  metadata about programs, such as build environment info, compiler
  command-lines, etc.

* [Symbols](symbols/symbols.md) describes how procedures, global variables, and
  other program elements, collectively known as "symbols", are represented.

* [Line Data](./line_data.md) describes how source code line mappings work.

* [Number](./number.md) describes the `Number` type, which represents literal
  constant values in a variety of types.
