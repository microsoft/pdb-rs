use crate::IMAGE_FILE_MACHINE;

pub const IMAGE_REL_I386_ABSOLUTE: u16 = 0x0000; // Reference is absolute, no relocation is necessary
pub const IMAGE_REL_I386_DIR16: u16 = 0x0001; // Direct 16-bit reference to the symbols virtual address
pub const IMAGE_REL_I386_REL16: u16 = 0x0002; // PC-relative 16-bit reference to the symbols virtual address
pub const IMAGE_REL_I386_DIR32: u16 = 0x0006; // Direct 32-bit reference to the symbols virtual address
pub const IMAGE_REL_I386_DIR32NB: u16 = 0x0007; // Direct 32-bit reference to the symbols virtual address, base not included
pub const IMAGE_REL_I386_SEG12: u16 = 0x0009; // Direct 16-bit reference to the segment-selector bits of a 32-bit virtual address
pub const IMAGE_REL_I386_SECTION: u16 = 0x000A;
pub const IMAGE_REL_I386_SECREL: u16 = 0x000B;
pub const IMAGE_REL_I386_TOKEN: u16 = 0x000C; // clr token
pub const IMAGE_REL_I386_SECREL7: u16 = 0x000D; // 7 bit offset from base of section containing target
pub const IMAGE_REL_I386_REL32: u16 = 0x0014; // PC-relative 32-bit reference to the symbols virtual address

pub fn reloc_type_str_i386(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_I386_ABSOLUTE => Some("IMAGE_REL_I386_ABSOLUTE"),
        IMAGE_REL_I386_DIR16 => Some("IMAGE_REL_I386_DIR16"),
        IMAGE_REL_I386_REL16 => Some("IMAGE_REL_I386_REL16"),
        IMAGE_REL_I386_DIR32 => Some("IMAGE_REL_I386_DIR32"),
        IMAGE_REL_I386_DIR32NB => Some("IMAGE_REL_I386_DIR32NB"),
        IMAGE_REL_I386_SEG12 => Some("IMAGE_REL_I386_SEG12"),
        IMAGE_REL_I386_SECTION => Some("IMAGE_REL_I386_SECTION"),
        IMAGE_REL_I386_SECREL => Some("IMAGE_REL_I386_SECREL"),
        IMAGE_REL_I386_TOKEN => Some("IMAGE_REL_I386_TOKEN"),
        IMAGE_REL_I386_SECREL7 => Some("IMAGE_REL_I386_SECREL7"),
        IMAGE_REL_I386_REL32 => Some("IMAGE_REL_I386_REL32"),
        _ => None,
    }
}

pub fn reloc_type_str_short_i386(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_I386_ABSOLUTE => Some("ABSOLUTE"),
        IMAGE_REL_I386_DIR16 => Some("DIR16"),
        IMAGE_REL_I386_REL16 => Some("REL16"),
        IMAGE_REL_I386_DIR32 => Some("DIR32"),
        IMAGE_REL_I386_DIR32NB => Some("DIR32NB"),
        IMAGE_REL_I386_SEG12 => Some("SEG12"),
        IMAGE_REL_I386_SECTION => Some("SECTION"),
        IMAGE_REL_I386_SECREL => Some("SECREL"),
        IMAGE_REL_I386_TOKEN => Some("TOKEN"),
        IMAGE_REL_I386_SECREL7 => Some("SECREL7"),
        IMAGE_REL_I386_REL32 => Some("REL32"),
        _ => None,
    }
}

//
// ARM64 relocations types.
//

pub const IMAGE_REL_ARM64_ABSOLUTE: u16 = 0x0000; // No relocation required
pub const IMAGE_REL_ARM64_ADDR32: u16 = 0x0001; // 32 bit address. Review! do we need it?
pub const IMAGE_REL_ARM64_ADDR32NB: u16 = 0x0002; // 32 bit address w/o image base (RVA: for Data/PData/XData)
pub const IMAGE_REL_ARM64_BRANCH26: u16 = 0x0003; // 26 bit offset << 2 & sign ext. for B & BL
pub const IMAGE_REL_ARM64_PAGEBASE_REL21: u16 = 0x0004; // ADRP
pub const IMAGE_REL_ARM64_REL21: u16 = 0x0005; // ADR
pub const IMAGE_REL_ARM64_PAGEOFFSET_12A: u16 = 0x0006; // ADD/ADDS (immediate) with zero shift, for page offset
pub const IMAGE_REL_ARM64_PAGEOFFSET_12L: u16 = 0x0007; // LDR (indexed, unsigned immediate), for page offset
pub const IMAGE_REL_ARM64_SECREL: u16 = 0x0008; // Offset within section
pub const IMAGE_REL_ARM64_SECREL_LOW12A: u16 = 0x0009; // ADD/ADDS (immediate) with zero shift, for bit 0:11 of section offset
pub const IMAGE_REL_ARM64_SECREL_HIGH12A: u16 = 0x000A; // ADD/ADDS (immediate) with zero shift, for bit 12:23 of section offset
pub const IMAGE_REL_ARM64_SECREL_LOW12L: u16 = 0x000B; // LDR (indexed, unsigned immediate), for bit 0:11 of section offset
pub const IMAGE_REL_ARM64_TOKEN: u16 = 0x000C;
pub const IMAGE_REL_ARM64_SECTION: u16 = 0x000D; // Section table index
pub const IMAGE_REL_ARM64_ADDR64: u16 = 0x000E; // 64 bit address
pub const IMAGE_REL_ARM64_BRANCH19: u16 = 0x000F; // 19 bit offset << 2 & sign ext. for conditional B

pub fn reloc_type_str_arm64(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_ARM64_ABSOLUTE => Some("IMAGE_REL_ARM64_ABSOLUTE"),
        IMAGE_REL_ARM64_ADDR32 => Some("IMAGE_REL_ARM64_ADDR32"),
        IMAGE_REL_ARM64_ADDR32NB => Some("IMAGE_REL_ARM64_ADDR32NB"),
        IMAGE_REL_ARM64_BRANCH26 => Some("IMAGE_REL_ARM64_BRANCH26"),
        IMAGE_REL_ARM64_PAGEBASE_REL21 => Some("IMAGE_REL_ARM64_PAGEBASE_REL21"),
        IMAGE_REL_ARM64_REL21 => Some("IMAGE_REL_ARM64_REL21"),
        IMAGE_REL_ARM64_PAGEOFFSET_12A => Some("IMAGE_REL_ARM64_PAGEOFFSET_12A"),
        IMAGE_REL_ARM64_PAGEOFFSET_12L => Some("IMAGE_REL_ARM64_PAGEOFFSET_12L"),
        IMAGE_REL_ARM64_SECREL => Some("IMAGE_REL_ARM64_SECREL"),
        IMAGE_REL_ARM64_SECREL_LOW12A => Some("IMAGE_REL_ARM64_SECREL_LOW12A"),
        IMAGE_REL_ARM64_SECREL_HIGH12A => Some("IMAGE_REL_ARM64_SECREL_HIGH12A"),
        IMAGE_REL_ARM64_SECREL_LOW12L => Some("IMAGE_REL_ARM64_SECREL_LOW12L"),
        IMAGE_REL_ARM64_TOKEN => Some("IMAGE_REL_ARM64_TOKEN"),
        IMAGE_REL_ARM64_SECTION => Some("IMAGE_REL_ARM64_SECTION"),
        IMAGE_REL_ARM64_ADDR64 => Some("IMAGE_REL_ARM64_ADDR64"),
        IMAGE_REL_ARM64_BRANCH19 => Some("IMAGE_REL_ARM64_BRANCH19"),
        _ => None,
    }
}

pub fn reloc_type_str_short_arm64(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_ARM64_ABSOLUTE => Some("ABSOLUTE"),
        IMAGE_REL_ARM64_ADDR32 => Some("ADDR32"),
        IMAGE_REL_ARM64_ADDR32NB => Some("ADDR32NB"),
        IMAGE_REL_ARM64_BRANCH26 => Some("BRANCH26"),
        IMAGE_REL_ARM64_PAGEBASE_REL21 => Some("PAGEBASE_REL21"),
        IMAGE_REL_ARM64_REL21 => Some("REL21"),
        IMAGE_REL_ARM64_PAGEOFFSET_12A => Some("PAGEOFFSET_12A"),
        IMAGE_REL_ARM64_PAGEOFFSET_12L => Some("PAGEOFFSET_12L"),
        IMAGE_REL_ARM64_SECREL => Some("SECREL"),
        IMAGE_REL_ARM64_SECREL_LOW12A => Some("SECREL_LOW12A"),
        IMAGE_REL_ARM64_SECREL_HIGH12A => Some("SECREL_HIGH12A"),
        IMAGE_REL_ARM64_SECREL_LOW12L => Some("SECREL_LOW12L"),
        IMAGE_REL_ARM64_TOKEN => Some("TOKEN"),
        IMAGE_REL_ARM64_SECTION => Some("SECTION"),
        IMAGE_REL_ARM64_ADDR64 => Some("ADDR64"),
        IMAGE_REL_ARM64_BRANCH19 => Some("BRANCH19"),
        _ => None,
    }
}

//
// x64 relocations
//
pub const IMAGE_REL_AMD64_ABSOLUTE: u16 = 0x0000; // Reference is absolute, no relocation is necessary
pub const IMAGE_REL_AMD64_ADDR64: u16 = 0x0001; // 64-bit address (VA).
pub const IMAGE_REL_AMD64_ADDR32: u16 = 0x0002; // 32-bit address (VA).
pub const IMAGE_REL_AMD64_ADDR32NB: u16 = 0x0003; // 32-bit address w/o image base (RVA).
pub const IMAGE_REL_AMD64_REL32: u16 = 0x0004; // 32-bit relative address from byte following reloc
pub const IMAGE_REL_AMD64_REL32_1: u16 = 0x0005; // 32-bit relative address from byte distance 1 from reloc
pub const IMAGE_REL_AMD64_REL32_2: u16 = 0x0006; // 32-bit relative address from byte distance 2 from reloc
pub const IMAGE_REL_AMD64_REL32_3: u16 = 0x0007; // 32-bit relative address from byte distance 3 from reloc
pub const IMAGE_REL_AMD64_REL32_4: u16 = 0x0008; // 32-bit relative address from byte distance 4 from reloc
pub const IMAGE_REL_AMD64_REL32_5: u16 = 0x0009; // 32-bit relative address from byte distance 5 from reloc
pub const IMAGE_REL_AMD64_SECTION: u16 = 0x000A; // Section index
pub const IMAGE_REL_AMD64_SECREL: u16 = 0x000B; // 32 bit offset from base of section containing target
pub const IMAGE_REL_AMD64_SECREL7: u16 = 0x000C; // 7 bit unsigned offset from base of section containing target
pub const IMAGE_REL_AMD64_TOKEN: u16 = 0x000D; // 32 bit metadata token
pub const IMAGE_REL_AMD64_SREL32: u16 = 0x000E; // 32 bit signed span-dependent value emitted into object
pub const IMAGE_REL_AMD64_PAIR: u16 = 0x000F;
pub const IMAGE_REL_AMD64_SSPAN32: u16 = 0x0010; // 32 bit signed span-dependent value applied at link time
pub const IMAGE_REL_AMD64_EHANDLER: u16 = 0x0011;
pub const IMAGE_REL_AMD64_IMPORT_BR: u16 = 0x0012; // Indirect branch to an import
pub const IMAGE_REL_AMD64_IMPORT_CALL: u16 = 0x0013; // Indirect call to an import
pub const IMAGE_REL_AMD64_CFG_BR: u16 = 0x0014; // Indirect branch to a CFG check
pub const IMAGE_REL_AMD64_CFG_BR_REX: u16 = 0x0015; // Indirect branch to a CFG check, with REX.W prefix
pub const IMAGE_REL_AMD64_CFG_CALL: u16 = 0x0016; // Indirect call to a CFG check
pub const IMAGE_REL_AMD64_INDIR_BR: u16 = 0x0017; // Indirect branch to a target in RAX (no CFG)
pub const IMAGE_REL_AMD64_INDIR_BR_REX: u16 = 0x0018; // Indirect branch to a target in RAX, with REX.W prefix (no CFG)
pub const IMAGE_REL_AMD64_INDIR_CALL: u16 = 0x0019; // Indirect call to a target in RAX (no CFG)
pub const IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_FIRST: u16 = 0x0020; // Indirect branch for a switch table using Reg 0 (RAX)
pub const IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_LAST: u16 = 0x002F; // Indirect branch for a switch table using Reg 15 (R15)

pub fn reloc_type_str_amd64(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_AMD64_ABSOLUTE => Some("IMAGE_REL_AMD64_ABSOLUTE"),
        IMAGE_REL_AMD64_ADDR64 => Some("IMAGE_REL_AMD64_ADDR64"),
        IMAGE_REL_AMD64_ADDR32 => Some("IMAGE_REL_AMD64_ADDR32"),
        IMAGE_REL_AMD64_ADDR32NB => Some("IMAGE_REL_AMD64_ADDR32NB"),
        IMAGE_REL_AMD64_REL32 => Some("IMAGE_REL_AMD64_REL32"),
        IMAGE_REL_AMD64_REL32_1 => Some("IMAGE_REL_AMD64_REL32_1"),
        IMAGE_REL_AMD64_REL32_2 => Some("IMAGE_REL_AMD64_REL32_2"),
        IMAGE_REL_AMD64_REL32_3 => Some("IMAGE_REL_AMD64_REL32_3"),
        IMAGE_REL_AMD64_REL32_4 => Some("IMAGE_REL_AMD64_REL32_4"),
        IMAGE_REL_AMD64_REL32_5 => Some("IMAGE_REL_AMD64_REL32_5"),
        IMAGE_REL_AMD64_SECTION => Some("IMAGE_REL_AMD64_SECTION"),
        IMAGE_REL_AMD64_SECREL => Some("IMAGE_REL_AMD64_SECREL"),
        IMAGE_REL_AMD64_SECREL7 => Some("IMAGE_REL_AMD64_SECREL7"),
        IMAGE_REL_AMD64_TOKEN => Some("IMAGE_REL_AMD64_TOKEN"),
        IMAGE_REL_AMD64_SREL32 => Some("IMAGE_REL_AMD64_SREL32"),
        IMAGE_REL_AMD64_PAIR => Some("IMAGE_REL_AMD64_PAIR"),
        IMAGE_REL_AMD64_SSPAN32 => Some("IMAGE_REL_AMD64_SSPAN32"),
        IMAGE_REL_AMD64_EHANDLER => Some("IMAGE_REL_AMD64_EHANDLER"),
        IMAGE_REL_AMD64_IMPORT_BR => Some("IMAGE_REL_AMD64_IMPORT_BR"),
        IMAGE_REL_AMD64_IMPORT_CALL => Some("IMAGE_REL_AMD64_IMPORT_CALL"),
        IMAGE_REL_AMD64_CFG_BR => Some("IMAGE_REL_AMD64_CFG_BR"),
        IMAGE_REL_AMD64_CFG_BR_REX => Some("IMAGE_REL_AMD64_CFG_BR_REX"),
        IMAGE_REL_AMD64_CFG_CALL => Some("IMAGE_REL_AMD64_CFG_CALL"),
        IMAGE_REL_AMD64_INDIR_BR => Some("IMAGE_REL_AMD64_INDIR_BR"),
        IMAGE_REL_AMD64_INDIR_BR_REX => Some("IMAGE_REL_AMD64_INDIR_BR_REX"),
        IMAGE_REL_AMD64_INDIR_CALL => Some("IMAGE_REL_AMD64_INDIR_CALL"),
        IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_FIRST => {
            Some("IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_FIRST")
        }
        IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_LAST => {
            Some("IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_LAST")
        }
        _ => None,
    }
}

pub fn reloc_type_str_short_amd64(reloc: u16) -> Option<&'static str> {
    match reloc {
        IMAGE_REL_AMD64_ABSOLUTE => Some("ABSOLUTE"),
        IMAGE_REL_AMD64_ADDR64 => Some("ADDR64"),
        IMAGE_REL_AMD64_ADDR32 => Some("ADDR32"),
        IMAGE_REL_AMD64_ADDR32NB => Some("ADDR32NB"),
        IMAGE_REL_AMD64_REL32 => Some("REL32"),
        IMAGE_REL_AMD64_REL32_1 => Some("REL32_1"),
        IMAGE_REL_AMD64_REL32_2 => Some("REL32_2"),
        IMAGE_REL_AMD64_REL32_3 => Some("REL32_3"),
        IMAGE_REL_AMD64_REL32_4 => Some("REL32_4"),
        IMAGE_REL_AMD64_REL32_5 => Some("REL32_5"),
        IMAGE_REL_AMD64_SECTION => Some("SECTION"),
        IMAGE_REL_AMD64_SECREL => Some("SECREL"),
        IMAGE_REL_AMD64_SECREL7 => Some("SECREL7"),
        IMAGE_REL_AMD64_TOKEN => Some("TOKEN"),
        IMAGE_REL_AMD64_SREL32 => Some("SREL32"),
        IMAGE_REL_AMD64_PAIR => Some("PAIR"),
        IMAGE_REL_AMD64_SSPAN32 => Some("SSPAN32"),
        IMAGE_REL_AMD64_EHANDLER => Some("EHANDLER"),
        IMAGE_REL_AMD64_IMPORT_BR => Some("IMPORT_BR"),
        IMAGE_REL_AMD64_IMPORT_CALL => Some("IMPORT_CALL"),
        IMAGE_REL_AMD64_CFG_BR => Some("CFG_BR"),
        IMAGE_REL_AMD64_CFG_BR_REX => Some("CFG_BR_REX"),
        IMAGE_REL_AMD64_CFG_CALL => Some("CFG_CALL"),
        IMAGE_REL_AMD64_INDIR_BR => Some("INDIR_BR"),
        IMAGE_REL_AMD64_INDIR_BR_REX => Some("INDIR_BR_REX"),
        IMAGE_REL_AMD64_INDIR_CALL => Some("INDIR_CALL"),
        IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_FIRST => Some("INDIR_BR_SWITCHTABLE_FIRST"),
        IMAGE_REL_AMD64_INDIR_BR_SWITCHTABLE_LAST => Some("INDIR_BR_SWITCHTABLE_LAST"),
        _ => None,
    }
}

pub fn reloc_type_str(machine: IMAGE_FILE_MACHINE, reloc: u16) -> Option<&'static str> {
    match machine {
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_I386 => reloc_type_str_i386(reloc),
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_AMD64 => reloc_type_str_amd64(reloc),
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_ARM64 => reloc_type_str_arm64(reloc),
        _ => None,
    }
}

pub fn reloc_type_str_short(machine: IMAGE_FILE_MACHINE, reloc: u16) -> Option<&'static str> {
    match machine {
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_I386 => reloc_type_str_short_i386(reloc),
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_AMD64 => reloc_type_str_short_amd64(reloc),
        IMAGE_FILE_MACHINE::IMAGE_FILE_MACHINE_ARM64 => reloc_type_str_short_arm64(reloc),
        _ => None,
    }
}
