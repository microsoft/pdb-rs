use std::cmp::Ordering;

use ms_codeview::syms::OffsetSegment;

/// Contains a list of COFF groups.
#[derive(Clone, Default, Debug)]
pub struct CoffGroups {
    /// The COFF groups
    pub vec: Vec<CoffGroup>,
}

/// Describes a single COFF group.
///
/// A COFF group is a contiguous region within a COFF section. For this reason, they are sometimes
/// called "subsections".
#[derive(Clone, Debug)]
pub struct CoffGroup {
    /// Name of the section
    pub name: String,
    /// Bit flags
    pub characteristics: u32,
    /// The location where this COFF group begins. The COFF group is contained entirely within
    /// a single COFF section.
    pub offset_segment: OffsetSegment,
    /// The size in bytes of the COFF group.
    pub size: u32,
}

impl CoffGroups {
    /// Find the COFF group which contains `offset_segment`.
    pub fn find_group_at(&self, offset_segment: OffsetSegment) -> Option<&CoffGroup> {
        let seg = offset_segment.segment.get();
        let off = offset_segment.offset.get();

        match self.vec.binary_search_by(|g| {
            let g_seg: u16 = g.offset_segment.segment();
            let c = g_seg.cmp(&seg);
            if c.is_ne() {
                return c;
            }

            let g_off = g.offset_segment.offset();
            if off < g_off {
                return Ordering::Greater;
            }

            let offset_within_group = off - g_off;
            if offset_within_group < g.size {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }) {
            Ok(i) => Some(&self.vec[i]),
            Err(_) => None,
        }
    }
}
