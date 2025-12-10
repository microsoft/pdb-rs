use zerocopy_derive::*;

#[repr(transparent)]
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
pub struct IMAGE_DLLCHARACTERISTICS(pub u16);

//      IMAGE_LIBRARY_PROCESS_INIT            0x0001     // Reserved.
//      IMAGE_LIBRARY_PROCESS_TERM            0x0002     // Reserved.
//      IMAGE_LIBRARY_THREAD_INIT             0x0004     // Reserved.
//      IMAGE_LIBRARY_THREAD_TERM             0x0008     // Reserved.

/// Image can handle a high entropy 64-bit virtual address space.
pub const IMAGE_DLLCHARACTERISTICS_HIGH_ENTROPY_VA: u16 = 0x0020;
/// DLL can move.
pub const IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE: u16 = 0x0040;
/// Code Integrity Image
pub const IMAGE_DLLCHARACTERISTICS_FORCE_INTEGRITY: u16 = 0x0080;
/// Image is NX compatible
pub const IMAGE_DLLCHARACTERISTICS_NX_COMPAT: u16 = 0x0100;
/// Image understands isolation and doesn't want it
pub const IMAGE_DLLCHARACTERISTICS_NO_ISOLATION: u16 = 0x0200;
/// Image does not use SEH.  No SE handler may reside in this image
pub const IMAGE_DLLCHARACTERISTICS_NO_SEH: u16 = 0x0400;
/// Do not bind this image.
pub const IMAGE_DLLCHARACTERISTICS_NO_BIND: u16 = 0x0800;
/// Image should execute in an AppContainer
pub const IMAGE_DLLCHARACTERISTICS_APPCONTAINER: u16 = 0x1000;
/// Driver uses WDM model
pub const IMAGE_DLLCHARACTERISTICS_WDM_DRIVER: u16 = 0x2000;
/// Image supports Control Flow Guard.
pub const IMAGE_DLLCHARACTERISTICS_GUARD_CF: u16 = 0x4000;
pub const IMAGE_DLLCHARACTERISTICS_TERMINAL_SERVER_AWARE: u16 = 0x8000;
