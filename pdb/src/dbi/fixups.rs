//! Definitions relating to the `FIXUP_DATA` Optional Debug Substream.

use super::*;
use crate::Pdb;

/// Describes a fixup record, stored in the `FIXUP_DATA` Optional Debug Substream.
///
/// This _does not_ describe the structure of a relocation record (stored in the PE binary itself).
/// Instead, this structure describes "fixups" (which are essentially relocation records) that are
/// stored in the `FIXUP_DATA` Optional Debug Substream. These records allow binary analysis tools
/// to discover relationships between code (and between code and data) that are not otherwise
/// represented in the PE binary itself.
#[allow(missing_docs)]
#[repr(C)]
#[derive(Clone, Default, Debug, FromBytes, Immutable, IntoBytes, KnownLayout)]
pub struct Fixup {
    /// Relocation type
    ///
    /// This is one of the `IMAGE_REL_*` constants. See the documentation for PE relocations.
    /// These values are architecture-dependent; the same numeric values may have different
    /// meanings on different architectures.
    pub fixup_type: u16,
    pub extra: u16,
    pub rva: u32,
    pub rva_target: u32,
}

impl<F: ReadAt> Pdb<F> {
    /// Reads (uncached) the `FIXUP_DATA` optional debug stream.
    pub fn read_fixups(&self) -> anyhow::Result<Option<Vec<Fixup>>> {
        let Some(fixup_stream_index) = self.fixup_stream()? else {
            return Ok(None);
        };

        let sr = self.get_stream_reader(fixup_stream_index)?;
        let num_records = sr.stream_size() as usize / size_of::<Fixup>();
        let mut fixups: Vec<Fixup> = Fixup::new_vec_zeroed(num_records).unwrap();
        sr.read_exact_at(fixups.as_mut_bytes(), 0)?;
        Ok(Some(fixups))
    }
}
