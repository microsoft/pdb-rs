//! DBI Section Contribution Substream
//!
//! The Section Contributions Substream describes the COFF sections that contributed to a linked
//! binary. Section contributions come from object files that are submitted to the linker.
//!
//! The Section Contributions table is usually quite large, especially for large binaries.
//!
//! # References
//! * [`SC2` in `dbicommon.h`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/PDB/include/dbicommon.h#L107)

use super::*;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

/// Describes one section contribution.
#[allow(missing_docs)]
#[derive(Unaligned, IntoBytes, FromBytes, Immutable, KnownLayout, Clone, Debug)]
#[repr(C)]
pub struct SectionContribEntry {
    /// The section index
    pub section: U16<LE>,
    /// Alignment padding
    pub padding1: [u8; 2],
    pub offset: I32<LE>,
    pub size: I32<LE>,
    pub characteristics: U32<LE>,
    /// The zero-based module index of the module containing this section contribution.
    pub module_index: U16<LE>,
    /// Alignment padding
    pub padding2: [u8; 2],
    pub data_crc: U32<LE>,
    pub reloc_crc: U32<LE>,
}

/// Describes one section contribution.
#[allow(missing_docs)]
#[derive(Unaligned, IntoBytes, FromBytes, Immutable, KnownLayout, Clone, Debug)]
#[repr(C)]
pub struct SectionContribEntry2 {
    pub base: SectionContribEntry,
    pub coff_section: U32<LE>,
}

impl SectionContribEntry {
    /// Tests whether `offset` falls within this section contribution.
    pub fn contains_offset(&self, offset: i32) -> bool {
        let self_offset = self.offset.get();
        if offset < self_offset {
            return false;
        }

        let overshoot = offset - self_offset;
        if overshoot >= self.size.get() {
            return false;
        }

        true
    }
}

/// Decodes the Section Contribution Substream.
pub struct SectionContributionsSubstream<'a> {
    /// The array of section contributions.
    pub contribs: &'a [SectionContribEntry],
}

/// Version 6.0 of the Section Contributions Substream. This is the only supported version.
pub const SECTION_CONTRIBUTIONS_SUBSTREAM_VER60: u32 = 0xeffe0000 + 19970605;

impl<'a> SectionContributionsSubstream<'a> {
    /// Parses the header of the Section Contributions Substream.
    ///
    /// It is legal for a Section Contributions Substream to be entirely empty.
    pub fn parse(bytes: &'a [u8]) -> anyhow::Result<Self> {
        let mut p = Parser::new(bytes);
        if p.is_empty() {
            return Ok(Self { contribs: &[] });
        }

        let version = p.u32()?;

        match version {
            SECTION_CONTRIBUTIONS_SUBSTREAM_VER60 => {}
            _ => {
                bail!("The Section Contributions Substream has a version number that is not supported. Version: 0x{:08x}", version);
            }
        }

        let records_bytes = p.into_rest();
        let Ok(contribs) = <[SectionContribEntry]>::ref_from_bytes(records_bytes) else {
            bail!("The Section Contributions stream has an invalid size. It is not a multiple of the section contribution record size.  Size: 0x{:x}",
                bytes.len());
        };
        Ok(SectionContributionsSubstream { contribs })
    }

    /// Searches for a section contribution that contains the given offset.
    /// The `section` must match exactly. This uses binary search.
    pub fn find(&self, section: u16, offset: i32) -> Option<&SectionContribEntry> {
        let i = self.find_index(section, offset)?;
        Some(&self.contribs[i])
    }

    /// Searches for the index of a section contribution that contains the given offset.
    /// The `section` must match exactly. This uses binary search.
    pub fn find_index(&self, section: u16, offset: i32) -> Option<usize> {
        match self
            .contribs
            .binary_search_by_key(&(section, offset), |con| {
                (con.section.get(), con.offset.get())
            }) {
            Ok(i) => Some(i),
            Err(i) => {
                // We didn't find it, but i is close to it.
                if i > 0 {
                    let previous = &self.contribs[i - 1];
                    if previous.contains_offset(offset) {
                        return Some(i - 1);
                    }
                }

                if i + 1 < self.contribs.len() {
                    let next = &self.contribs[i + 1];
                    if next.contains_offset(offset) {
                        return Some(i + 1);
                    }
                }

                None
            }
        }
    }

    /// Searches for a section contribution that contains the given offset.
    /// The `section` must match exactly. This uses sequential scan (brute force).
    pub fn find_brute(&self, section: u16, offset: i32) -> Option<&SectionContribEntry> {
        let i = self.find_index_brute(section, offset)?;
        Some(&self.contribs[i])
    }

    /// Searches for the index of a section contribution that contains the given offset.
    /// The `section` must match exactly. This uses sequential scan (brute force).
    pub fn find_index_brute(&self, section: u16, offset: i32) -> Option<usize> {
        self.contribs
            .iter()
            .position(|c| c.section.get() == section && c.contains_offset(offset))
    }
}

/// Decodes the Section Contribution Substream.
pub struct SectionContributionsSubstreamMut<'a> {
    /// The array of section contributions.
    pub contribs: &'a mut [SectionContribEntry],
}

impl<'a> SectionContributionsSubstreamMut<'a> {
    /// Parses the header of the Section Contributions Substream.
    pub fn parse(bytes: &'a mut [u8]) -> anyhow::Result<Self> {
        let bytes_len = bytes.len();

        let mut p = ParserMut::new(bytes);
        if p.is_empty() {
            return Ok(Self { contribs: &mut [] });
        }

        let version = p.u32()?;

        match version {
            SECTION_CONTRIBUTIONS_SUBSTREAM_VER60 => {}
            _ => {
                bail!("The Section Contributions Substream has a version number that is not supported. Version: 0x{:08x}", version);
            }
        }

        let records_bytes = p.into_rest();

        let Ok(contribs) = <[SectionContribEntry]>::mut_from_bytes(records_bytes) else {
            bail!("The Section Contributions stream has an invalid size. It is not a multiple of the section contribution record size.  Size: 0x{:x}",
                bytes_len);
        };
        Ok(Self { contribs })
    }

    /// Given a lookup table that maps module indexes from old to new, this edits a
    /// Section Contributions table and converts module indexes.
    pub fn remap_module_indexes(&mut self, modules_old_to_new: &[u32]) -> anyhow::Result<()> {
        for (i, contrib) in self.contribs.iter_mut().enumerate() {
            let old = contrib.module_index.get();
            if let Some(&new) = modules_old_to_new.get(old as usize) {
                contrib.module_index.set(new as u16);
            } else {
                bail!("Section contribution record (at contribution index #{i} has module index {old}, \
                       which is out of range (num modules is {})",
                    modules_old_to_new.len());
            }

            // While we're at it, make sure that the padding fields are cleared.
            contrib.padding1 = [0; 2];
            contrib.padding2 = [0; 2];
        }
        Ok(())
    }
}
