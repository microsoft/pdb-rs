use zerocopy_derive::*;

use crate::IMAGE_DLLCHARACTERISTICS;

#[repr(C)]
#[derive(
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    IntoBytes,
    FromBytes,
    Immutable,
    KnownLayout,
)]
pub struct IMAGE_FILE_HEADER {
    pub machine: u16,
    pub number_of_sections: u16,
    pub time_date_stamp: u32,
    pub pointer_to_symbol_table: u32,
    pub number_of_symbols: u32,
    pub size_of_optional_header: u16,
    pub characteristics: u16,
}

#[repr(C)]
#[derive(
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    IntoBytes,
    FromBytes,
    Immutable,
    KnownLayout,
)]
pub struct IMAGE_DATA_DIRECTORY {
    pub virtual_address: u32,
    pub size: u32,
}

pub const IMAGE_NUMBEROF_DIRECTORY_ENTRIES: usize = 16;

#[repr(C)]
#[derive(
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    IntoBytes,
    FromBytes,
    Immutable,
    KnownLayout,
)]
pub struct IMAGE_OPTIONAL_HEADER32 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,
    pub image_base: u32,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub check_sum: u32,
    pub subsystem: u16,
    pub dll_characteristics: IMAGE_DLLCHARACTERISTICS,
    pub size_of_stack_reserve: u32,
    pub size_of_stack_commit: u32,
    pub size_of_heap_reserve: u32,
    pub size_of_heap_commit: u32,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

#[repr(C)]
#[derive(
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    IntoBytes,
    FromBytes,
    Immutable,
    KnownLayout,
)]
pub struct IMAGE_OPTIONAL_HEADER64 {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub image_base: u64,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub major_operating_system_version: u16,
    pub minor_operating_system_version: u16,
    pub major_image_version: u16,
    pub minor_image_version: u16,
    pub major_subsystem_version: u16,
    pub minor_subsystem_version: u16,
    pub win32_version_value: u32,
    pub size_of_image: u32,
    pub size_of_headers: u32,
    pub check_sum: u32,
    pub subsystem: u16,
    pub dll_characteristics: IMAGE_DLLCHARACTERISTICS,
    pub size_of_stack_reserve: u64,
    pub size_of_stack_commit: u64,
    pub size_of_heap_reserve: u64,
    pub size_of_heap_commit: u64,
    pub loader_flags: u32,
    pub number_of_rva_and_sizes: u32,
    pub data_directory: [IMAGE_DATA_DIRECTORY; IMAGE_NUMBEROF_DIRECTORY_ENTRIES],
}

pub const IMAGE_NT_OPTIONAL_HDR32_MAGIC: u16 = 0x10b;
pub const IMAGE_NT_OPTIONAL_HDR64_MAGIC: u16 = 0x20b;
pub const IMAGE_ROM_OPTIONAL_HDR_MAGIC: u16 = 0x107;

pub struct IMAGE_NT_HEADERS64 {
    pub signature: u32,
    pub file_header: IMAGE_FILE_HEADER,
    pub optional_header: IMAGE_OPTIONAL_HEADER64,
}

pub struct IMAGE_NT_HEADERS32 {
    pub signature: u32,
    pub file_header: IMAGE_FILE_HEADER,
    pub optional_header: IMAGE_OPTIONAL_HEADER32,
}

pub struct IMAGE_ROM_OPTIONAL_HEADER {
    pub magic: u16,
    pub major_linker_version: u8,
    pub minor_linker_version: u8,
    pub size_of_code: u32,
    pub size_of_initialized_data: u32,
    pub size_of_uninitialized_data: u32,
    pub address_of_entry_point: u32,
    pub base_of_code: u32,
    pub base_of_data: u32,
    pub base_of_bss: u32,
    pub gpr_mask: u32,
    pub cpr_mask: [u32; 4],
    pub gp_value: u32,
}

pub struct IMAGE_ROM_HEADERS {
    pub file_header: IMAGE_FILE_HEADER,
    pub optional_header: IMAGE_ROM_OPTIONAL_HEADER,
}
