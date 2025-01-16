//! Logic for finding duplicated type definitions and removing duplicates
#![allow(missing_docs)]

use crate::parser::{Parse, Parser};
use crate::types::{Leaf, Struct, TypesIter};
use anyhow::Result;

pub fn find_dup_types(records: &[u8]) -> Result<()> {
    for ty in TypesIter::new(records) {
        let mut p = Parser::new(ty.data);

        match ty.kind {
            Leaf::LF_STRUCTURE | Leaf::LF_CLASS => {
                let _type_data = Struct::from_parser(&mut p)?;
                // type_data.name
            }
            _ => {}
        }
    }

    Ok(())
}
