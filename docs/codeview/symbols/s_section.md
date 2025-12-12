# `S_SECTION` (0x1136) - COFF Section

```c
struct Section {
    uint16_t section;          // Section number
    uint8_t  align;            // Alignment of this section (power of 2)  
    uint8_t  reserved;         // Reserved, must be zero
    uint32_t rva;              // RVA of this section base
    uint32_t cb;               // Size in bytes of this section
    uint32_t characteristics;  // Section characteristics (bit flags)
    strz name;                 // NUL-terminated section name
};
```

The `S_SECTION` symbol describes a COFF section in a PE executable. COFF
sections are contiguous regions of memory that contain code, data, or other
information in an executable file.

## Fields

`section` is the 1-based section number within the PE executable. This number is
used to reference the section from other parts of the debug information.

`align` specifies the alignment of this section as a power of 2. For example, a
value of 12 means the section is aligned to 2^12 = 4096 bytes.

`reserved` is a reserved field that must be zero.

`rva` is the Relative Virtual Address (RVA) of the base of this section. This is
the address where the section will be loaded in memory, relative to the base
address of the executable.

`cb` is the size of this section in bytes.

`characteristics` contains the section characteristics as bit flags. These flags
are the same as the COFF section characteristics defined by the Windows PE
format. See [IMAGE_SECTION_HEADER in the Windows documentation](https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_section_header)
for details on the specific bit flags.

`name` is the NUL-terminated name of the section, such as `.text`, `.data`,
`.rdata`, etc.

## Usage

The `S_SECTION` symbol provides mapping information between section numbers used
in other debug symbols and the actual sections in the PE executable. This
information is essential for debuggers and other tools to correctly resolve
addresses and understand the layout of the executable.

Section symbols are typically found in module symbol streams and provide the
foundation for interpreting other symbols that reference sections by number,
such as procedure symbols that specify which section contains their code.

## Examples

Common section names include:
- `.text` - executable code
- `.data` - initialized data
- `.rdata` - read-only data
- `.bss` - uninitialized data
- `.pdata` - exception handling information
- `.idata` - import table

This symbol appears only in the special `* Linker *` module symbol stream, not
in the global symbol stream.
