//! Architecture-specific definitions

macro_rules! register_set {
    (
        $( #[$a:meta] )*
        $v:vis enum $ty_name:ident;
        $( $reg_name:ident = $reg_value:expr, )*
    ) => {
        $( #[$a] )*
        #[allow(missing_docs)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        $v struct $ty_name(pub u16);

        #[allow(missing_docs)]
        impl $ty_name {
            $(
                pub const $reg_name: $ty_name = $ty_name($reg_value);
            )*

            #[inline(never)]
            pub fn get_name(self) -> Option<&'static str> {
                match self {
                    $(
                        Self::$reg_name => Some(stringify!($reg_name)),
                    )*
                    _ => None,
                }
            }

            #[inline(never)]
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $(
                        stringify!($reg_name) => Some(Self::$reg_name),
                    )*
                    _ => None,
                }
            }
        }

        impl core::fmt::Display for $ty_name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                if let Some(s) = self.get_name() {
                    f.write_str(s)
                } else {
                    write!(f, "??(0x{:x})", self.0)
                }
            }
        }

        impl core::fmt::Debug for $ty_name {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                <Self as core::fmt::Display>::fmt(self, f)
            }
        }
    }
}

/// Identifies COFF CPU architectures.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum Arch {
    /// AMD64
    AMD64,
    /// ARM64, including ARM64EC, ARM64X
    ARM64,
    /// X86
    X86,
}

pub mod amd64;
pub mod arm64;
pub mod x86;

/// Identifies a register in a specific architecture
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct ArchReg {
    /// Target architecture
    pub arch: Arch,
    /// The untyped register index
    pub reg: u16,
}

impl ArchReg {
    /// Ties an arch and a reg
    pub fn new(arch: Arch, reg: u16) -> Self {
        Self { arch, reg }
    }
}

use core::fmt::{Debug, Display};

impl Debug for ArchReg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Display>::fmt(self, f)
    }
}

impl Display for ArchReg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.arch {
            Arch::AMD64 => Display::fmt(&amd64::Amd64Reg(self.reg), f),
            Arch::X86 => Display::fmt(&x86::X86Reg(self.reg), f),
            Arch::ARM64 => Display::fmt(&arm64::Arm64Reg(self.reg), f),
        }
    }
}
