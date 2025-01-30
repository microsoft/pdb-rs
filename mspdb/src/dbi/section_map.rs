//! DBI Section Map Substream
#![allow(missing_docs)]

use super::*;
use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(IntoBytes, KnownLayout, Immutable, FromBytes, Unaligned)]
#[repr(C)]
pub struct SectionMapHeader {
    /// Total number of segment descriptors
    pub num_segments: U16<LE>,
    /// Number of logical segment descriptors
    pub num_logical_segments: U16<LE>,
}

#[derive(IntoBytes, KnownLayout, Immutable, FromBytes, Unaligned)]
#[repr(C)]
pub struct SectionMapEntry {
    /// Descriptor flags bit field. See `SectionMapEntryFlags`.
    pub flags: U16<LE>,
    /// The logical overlay number
    pub overlay: U16<LE>,
    /// Group index into the descriptor array
    pub group: U16<LE>,
    /// Logical segment index, interpreted via flags
    pub frame: U16<LE>,
    /// Byte index of segment / group name in string table, or 0xFFFF.
    pub section_name: U16<LE>,
    /// Byte index of class in string table, or 0xFFFF.
    pub class_name: U16<LE>,
    /// Byte offset of the logical segment within physical segment.
    /// If group is set in flags, this is the offset of the group.
    pub offset: U32<LE>,
    /// Byte count of the segment or group.
    pub section_length: U32<LE>,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
    pub struct SectionMapEntryFlags: u16 {
        /// Segment is readable.
        const READ = 1 << 0;
        /// Segment is writable.
        const WRITE = 1 << 1;
        /// Segment is executable.
        const EXECUTE = 1 << 2;
        /// Descriptor describes a 32-bit linear address.
        const ADDRESS_IS32_BIT = 1 << 3;
        /// Frame represents a selector.
        const IS_SELECTOR = 1 << 8;
        /// Frame represents an absolute address.
        const IS_ABSOLUTE_ADDRESS = 1 << 9;
        /// If set, descriptor represents a group. (obsolete)
        const IS_GROUP = 1 << 10;
    }
}

pub struct SectionMap<'a> {
    pub header: SectionMapHeader,
    pub entries: &'a [SectionMapEntry],
}

impl<'a> SectionMap<'a> {
    pub fn parse(bytes: &'a [u8]) -> anyhow::Result<Self> {
        let mut p = Parser::new(bytes);
        if p.is_empty() {
            return Ok(Self {
                entries: &[],
                header: SectionMapHeader {
                    num_logical_segments: U16::ZERO,
                    num_segments: U16::ZERO,
                },
            });
        }

        let header: SectionMapHeader = p.copy()?;

        let Ok(entries) = <[SectionMapEntry]>::ref_from_bytes(p.take_rest()) else {
            bail!("Section map has invalid length (is not a multiple of SectionMapEntry size). Length (including 4-byte header): 0x{:x}", bytes.len());
        };
        Ok(Self { header, entries })
    }
}
