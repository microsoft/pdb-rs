use anyhow::Result;
use bstr::BStr;
use ms_pdb::codeview::parser::Parse;
use ms_pdb::codeview::IteratorWithRangesExt;
use ms_pdb::types::{Enum, Leaf, Struct, TypeIndex};
use ms_pdb::Pdb;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::Path;
use tracing::{info, warn};

#[derive(clap::Parser)]
pub struct TpiDiffOptions {
    pub base_pdb: String,
    pub diff_pdb: String,
}

pub fn tpi_diff(options: &TpiDiffOptions) -> Result<()> {
    let base_pdb = Pdb::open(Path::new(&options.base_pdb))?;
    let diff_pdb = Pdb::open(Path::new(&options.diff_pdb))?;

    let base_tpi = base_pdb.read_type_stream()?;

    // Read base_tpi and build an index of the types.

    // (name, properties) -> base TypeIndex
    let mut struct_map: HashMap<(&BStr, u16), TypeIndex> = HashMap::new();

    let mut enums: HashMap<&BStr, TypeIndex> = HashMap::new();

    let base_type_index_begin: TypeIndex = base_tpi.type_index_begin();

    for (i, (_range, ty)) in base_tpi.iter_type_records().with_ranges().enumerate() {
        let base_type_index = TypeIndex(base_type_index_begin.0 + i as u32);

        match ty.kind {
            Leaf::LF_CLASS | Leaf::LF_STRUCTURE => {
                let st = Struct::parse(ty.data)?;

                match struct_map.entry((st.name, st.fixed.property.0.get())) {
                    Entry::Occupied(_occ) => {
                        warn!("found duplicate entry for {:?} : {}", ty.kind, st.name);
                        continue;
                    }

                    Entry::Vacant(vac) => {
                        vac.insert(base_type_index);
                    }
                }
            }

            Leaf::LF_ENUM => {
                let en = Enum::parse(ty.data)?;

                match enums.entry(en.name) {
                    Entry::Occupied(_) => {
                        warn!("found duplicate LF_ENUM : {}", en.name);
                        continue;
                    }

                    Entry::Vacant(vac) => {
                        vac.insert(base_type_index);
                    }
                }
            }

            _ => {}
        }
    }

    info!("Number of LF_STRUCTURE records found: {}", struct_map.len());

    // Scan the diff TPI and see which entries can be remapped.

    let diff_tpi = diff_pdb.read_type_stream()?;
    // let diff_type_index_begin = diff_tpi.type_index_begin();

    let mut num_records_remapped: u32 = 0;
    let mut len_new_tpi: usize = 0;

    for (_i, (range, ty)) in diff_tpi.iter_type_records().with_ranges().enumerate() {
        // let diff_type_index = TypeIndex(diff_type_index_begin.0 + i as u32);

        match ty.kind {
            Leaf::LF_CLASS | Leaf::LF_STRUCTURE => {
                let st = Struct::parse(ty.data)?;

                if struct_map.contains_key(&(st.name, st.fixed.property.0.get())) {
                    // TODO: do a deep structure equivalence test
                    // for now, assume name means the same struct
                    num_records_remapped += 1;
                    continue;
                }
            }

            Leaf::LF_ENUM => {
                let en = Enum::parse(ty.data)?;

                if enums.contains_key(&en.name) {
                    // TODO: do a deep structure equivalence test
                    // for now, assume name means the same struct
                    num_records_remapped += 1;
                    continue;
                }
            }
            _ => {}
        }

        len_new_tpi += range.end - range.start;
    }

    info!("Finished scanning diff TPI.");
    info!(
        "Number of LF_STRUCTURE records remapped: {}",
        num_records_remapped
    );
    info!(
        "Length of base TPI:   {0:10}  {0:10x}",
        base_tpi.type_records_bytes().len()
    );
    info!(
        "Length of diff TPI:   {0:10}  {0:10x}",
        diff_tpi.type_records_bytes().len()
    );
    info!(
        "Length of new TPI:    {0:10}  {0:10x}   {1:2.3} %",
        len_new_tpi,
        (diff_tpi.type_records_bytes().len() as f64 - len_new_tpi as f64) * 100.0
            / diff_tpi.type_records_bytes().len() as f64
    );

    Ok(())
}
