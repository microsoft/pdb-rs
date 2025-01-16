//! Id Stream (IPI)
//!
//! The ID Stream (IPI) uses the same stream header and record format as the TPI, but the
//! records stored in it have different kinds and different semantics.
//!
//! Some records in the IPI contain `TypeIndex` values that point into the TPI. This makes the IPI
//! stream dependent on the TPI stream.  The reverse is not true; the TPI does not contain pointers
//! into the IPI.

use crate::names::NameIndexLe;
use crate::names::NameIndexMapping;
use crate::parser::ParserError;
use crate::parser::ParserMut;
use crate::types;
use crate::types::visitor::visit_type_indexes_in_record_slice_mut;
use crate::types::visitor::IndexVisitorMut;
use crate::types::Leaf;
use crate::types::TypeIndex;
use crate::types::TypeIndexLe;
use crate::types::TypesIterMut;
use crate::types::UdtModSrcLine;
use crate::utils::iter::IteratorWithRangesExt;
use log::debug;
use zerocopy::U16;
use zerocopy::U32;

// TODO: This module defines several different passes through the IPI, which remap different
// fields. It may be beneficial (for performance) to combine those into a single pass.

/// Remaps `TypeIndex` values found within the IPI stream.
pub fn remap_type_indexes_in_ipi_records(
    item_records: &mut [u8],
    type_index_begin: TypeIndex,
    type_index_map: &[TypeIndex],
) -> anyhow::Result<()> {
    debug!("Remapping TypeIndex values in IPI Stream");

    struct Viz<'a> {
        type_index_begin: TypeIndex,
        type_index_map: &'a [TypeIndex],
    }

    impl<'a> IndexVisitorMut for Viz<'a> {
        fn item_id(
            &mut self,
            _offset: usize,
            _value: &mut types::ItemIdLe,
        ) -> Result<(), ParserError> {
            Ok(())
        }

        fn name_index(
            &mut self,
            _offset: usize,
            _value: &mut NameIndexLe,
        ) -> Result<(), ParserError> {
            Ok(())
        }

        fn type_index(&mut self, _offset: usize, ti: &mut TypeIndexLe) -> Result<(), ParserError> {
            let old_ti = ti.get();

            // Ignore primitives.
            if old_ti.0 < self.type_index_begin.0 {
                return Ok(());
            }

            let Some(&new) = self
                .type_index_map
                .get((old_ti.0 - self.type_index_begin.0) as usize)
            else {
                log::warn!(
                    "Found invalid TypeIndex in IPI: T#{:08x}.  max range: T#{:08x}",
                    old_ti.0,
                    self.type_index_begin.0 + self.type_index_map.len() as u32
                );
                return Ok(());
            };

            ti.0 = U32::new(new.0);
            Ok(())
        }
    }

    for (_ty_range, ty) in TypesIterMut::new(item_records).with_ranges() {
        visit_type_indexes_in_record_slice_mut(
            ty.kind,
            ty.data,
            Viz {
                type_index_begin,
                type_index_map,
            },
        )?;
    }

    Ok(())
}

/// Remaps `NameIndex` values found within the IPI stream.
pub fn remap_name_indexes_in_ipi_records(
    item_records: &mut [u8],
    name_remapping: &NameIndexMapping,
) -> anyhow::Result<()> {
    debug!("Remapping NameIndex values in IPI Stream");

    struct Viz<'a> {
        name_remapping: &'a NameIndexMapping,
    }

    impl<'a> IndexVisitorMut for Viz<'a> {
        fn item_id(
            &mut self,
            _offset: usize,
            _value: &mut types::ItemIdLe,
        ) -> Result<(), ParserError> {
            Ok(())
        }

        fn name_index(
            &mut self,
            _offset: usize,
            value: &mut NameIndexLe,
        ) -> Result<(), ParserError> {
            let new = self
                .name_remapping
                .map_old_to_new(value.get())
                .map_err(|_| ParserError::new())?;
            value.0 = U32::new(new.0);
            Ok(())
        }

        fn type_index(&mut self, _offset: usize, _ti: &mut TypeIndexLe) -> Result<(), ParserError> {
            Ok(())
        }
    }

    for (_ty_range, ty) in TypesIterMut::new(item_records).with_ranges() {
        // let record_stream_offset = stream_offset + ty_range.start as u32;
        visit_type_indexes_in_record_slice_mut(ty.kind, ty.data, Viz { name_remapping })?;
    }

    Ok(())
}

/// Scans through an IPI type record stream and searches for records that contain module indexes.
/// Remaps those module indexes.
///
/// The only record kind that contains module indexes is `LF_UDT_MOD_SRC_LINE`.
pub fn remap_module_indexes_in_ipi_records(
    stream_offset: u32,
    type_records: &mut [u8],
    module_old_to_new: &[u32],
) -> anyhow::Result<()> {
    for (ty_range, ty) in TypesIterMut::new(type_records).with_ranges() {
        let record_stream_offset = stream_offset + ty_range.start as u32;
        let mut p = ParserMut::new(ty.data);
        match ty.kind {
            Leaf::LF_UDT_MOD_SRC_LINE => {
                let rec: &mut UdtModSrcLine = p.get_mut()?;

                let old_module_index = rec.imod.get();
                if let Some(&new_module_index) = module_old_to_new.get(old_module_index as usize) {
                    rec.imod = U16::new(new_module_index as u16);
                } else {
                    anyhow::bail!(
                        "Found LF_UDT_MOD_SRC_LINE record at stream offset 0x{record_stream_offset:x}
                         that has a module index ({old_module_index}) that is outside of the valid
                         range of module indexes ({num_modules}).",
                         num_modules = module_old_to_new.len()
                    );
                }

                // TODO: just trash these values for now
                rec.imod = U16::new(0);
                // rec.line = U32::new(1);
            }

            _ => {}
        }
    }

    Ok(())
}
