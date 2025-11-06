//! Image section
//!
//! # References
//! * <https://learn.microsoft.com/en-us/windows/win32/api/winnt/ns-winnt-image_section_header>

use bstr::BStr;
use core::fmt::Debug;
use core::mem::size_of;
use static_assertions::const_assert_eq;
use zerocopy_derive::*;

#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Default,
    Hash,
    Ord,
    PartialOrd,
    IntoBytes,
    FromBytes,
    Immutable,
    KnownLayout,
)]
#[repr(transparent)]
pub struct SectionCharacteristics(pub u32);

bitflags::bitflags! {
    impl SectionCharacteristics: u32 {
        //
        // Section characteristics.
        //
        //      IMAGE_SCN_TYPE_REG                   0x00000000  // Reserved.
        //      IMAGE_SCN_TYPE_DSECT                 0x00000001  // Reserved.
        //      IMAGE_SCN_TYPE_NOLOAD                0x00000002  // Reserved.
        //      IMAGE_SCN_TYPE_GROUP                 0x00000004  // Reserved.
        const IMAGE_SCN_TYPE_NO_PAD = 8; // obsolete
        //      IMAGE_SCN_TYPE_COPY                  0x00000010  // Reserved.

        const IMAGE_SCN_CNT_CODE                   = 0x00000020;  // Section contains code.
        const IMAGE_SCN_CNT_INITIALIZED_DATA       = 0x00000040;  // Section contains initialized data.
        const IMAGE_SCN_CNT_UNINITIALIZED_DATA     = 0x00000080;  // Section contains uninitialized data.

        const IMAGE_SCN_LNK_OTHER                  = 0x00000100;  // Reserved.
        const IMAGE_SCN_LNK_INFO                   = 0x00000200;  // Section contains comments or some other type of information.
        //      IMAGE_SCN_TYPE_OVER                  0x00000400  // Reserved.
        const IMAGE_SCN_LNK_REMOVE                 = 0x00000800;  // Section contents will not become part of image.
        const IMAGE_SCN_LNK_COMDAT                 = 0x00001000;  // Section contents comdat.
        //                                           0x00002000  // Reserved.
        //      IMAGE_SCN_MEM_PROTECTED - Obsolete   0x00004000
        const IMAGE_SCN_NO_DEFER_SPEC_EXC          = 0x00004000;  // Reset speculative exceptions handling bits in the TLB entries for this section.
        const IMAGE_SCN_GPREL                      = 0x00008000;  // Section content can be accessed relative to GP
        const IMAGE_SCN_MEM_FARDATA                = 0x00008000;
        //      IMAGE_SCN_MEM_SYSHEAP  - Obsolete    0x00010000
        const IMAGE_SCN_MEM_PURGEABLE              = 0x00020000;
        const IMAGE_SCN_MEM_16BIT                  = 0x00020000;
        const IMAGE_SCN_MEM_LOCKED                 = 0x00040000;
        const IMAGE_SCN_MEM_PRELOAD                = 0x00080000;

        const IMAGE_SCN_ALIGN_1BYTES               = 0x00100000;  //
        const IMAGE_SCN_ALIGN_2BYTES               = 0x00200000;  //
        const IMAGE_SCN_ALIGN_4BYTES               = 0x00300000;  //
        const IMAGE_SCN_ALIGN_8BYTES               = 0x00400000;  //
        const IMAGE_SCN_ALIGN_16BYTES              = 0x00500000;  // Default alignment if no others are specified.
        const IMAGE_SCN_ALIGN_32BYTES              = 0x00600000;  //
        const IMAGE_SCN_ALIGN_64BYTES              = 0x00700000;  //
        const IMAGE_SCN_ALIGN_128BYTES             = 0x00800000;  //
        const IMAGE_SCN_ALIGN_256BYTES             = 0x00900000;  //
        const IMAGE_SCN_ALIGN_512BYTES             = 0x00A00000;  //
        const IMAGE_SCN_ALIGN_1024BYTES            = 0x00B00000;  //
        const IMAGE_SCN_ALIGN_2048BYTES            = 0x00C00000;  //
        const IMAGE_SCN_ALIGN_4096BYTES            = 0x00D00000;  //
        const IMAGE_SCN_ALIGN_8192BYTES            = 0x00E00000;  //
        // Unused                                    0x00F00000
        const IMAGE_SCN_ALIGN_MASK                 = 0x00F00000;

        const IMAGE_SCN_LNK_NRELOC_OVFL            = 0x01000000;  // Section contains extended relocations.
        const IMAGE_SCN_MEM_DISCARDABLE            = 0x02000000;  // Section can be discarded.
        const IMAGE_SCN_MEM_NOT_CACHED             = 0x04000000;  // Section is not cachable.
        const IMAGE_SCN_MEM_NOT_PAGED              = 0x08000000;  // Section is not pageable.
        const IMAGE_SCN_MEM_SHARED                 = 0x10000000;  // Section is shareable.
        const IMAGE_SCN_MEM_EXECUTE                = 0x20000000;  // Section is executable.
        const IMAGE_SCN_MEM_READ                   = 0x40000000;  // Section is readable.
        const IMAGE_SCN_MEM_WRITE                  = 0x80000000;  // Section is writeable.
    }
}

impl SectionCharacteristics {
    /// Returns true if this contains `IMAGE_SCN_MEM_READ`
    pub fn is_read(self) -> bool {
        self.intersects(Self::IMAGE_SCN_MEM_READ)
    }

    /// Returns true if this contains `IMAGE_SCN_MEM_WRITE`
    pub fn is_write(self) -> bool {
        self.intersects(Self::IMAGE_SCN_MEM_WRITE)
    }

    /// Returns true if this contains `IMAGE_SCN_MEM_EXECUTE`
    pub fn is_exec(self) -> bool {
        self.intersects(Self::IMAGE_SCN_MEM_EXECUTE)
    }
}

impl Debug for SectionCharacteristics {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[{:08x}", self.0)?;

        write!(f, "]")
    }
}

pub const IMAGE_SIZEOF_SHORT_NAME: usize = 8;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(
    Clone,
    Default,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    Hash,
    FromBytes,
    IntoBytes,
    Immutable,
    KnownLayout,
)]
pub struct IMAGE_SECTION_HEADER {
    pub name: [u8; IMAGE_SIZEOF_SHORT_NAME],
    pub physical_address_or_virtual_size: u32,
    pub virtual_address: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
    pub pointer_to_relocations: u32,
    pub pointer_to_linenumbers: u32,
    pub number_of_relocations: u16,
    pub number_of_linenumbers: u16,
    pub characteristics: SectionCharacteristics,
}

impl IMAGE_SECTION_HEADER {
    pub fn name(&self) -> &BStr {
        BStr::new(if let Some(i) = self.name.iter().position(|&b| b == 0) {
            &self.name[..i]
        } else {
            &self.name
        })
    }
}

pub const IMAGE_SIZEOF_SECTION_HEADER: usize = 40;

const_assert_eq!(
    size_of::<IMAGE_SECTION_HEADER>(),
    IMAGE_SIZEOF_SECTION_HEADER
);
