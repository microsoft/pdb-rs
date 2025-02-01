//! Hashing functions for Type Records
//!
//! # References
//!
//! * [`TPI1::hashPrec` in `tpi.cpp`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/PDB/dbi/tpi.cpp#L1296)

use crate::hash::hash_u32;
use crate::parser::ParserError;
use crate::types::{Leaf, TypeData, UdtProperties};
use bstr::BStr;
use zerocopy::IntoBytes;

/// Hash a type record, using the same rules as `TPI1::hashPrec`.
pub fn hash_type_record(
    kind: Leaf,
    record_bytes: &[u8],
    payload: &[u8],
) -> Result<u32, ParserError> {
    let tdata = TypeData::parse_bytes(kind, payload)?;

    match &tdata {
        TypeData::Alias(t) => Ok(hash_u32(t.name)),

        // This handles LF_CLASS, LF_STRUCTURE, and LF_INTERFACE.
        TypeData::Struct(t) => Ok(hash_udt_name(
            t.fixed.property.get(),
            record_bytes,
            t.name,
            t.unique_name,
        )),

        TypeData::Union(t) => Ok(hash_udt_name(
            t.fixed.property.get(),
            record_bytes,
            t.name,
            t.unique_name,
        )),

        TypeData::Enum(t) => Ok(hash_udt_name(
            t.fixed.property.get(),
            record_bytes,
            t.name,
            t.unique_name,
        )),

        TypeData::UdtSrcLine(t) => Ok(hash_u32(t.ty.as_bytes())),
        TypeData::UdtModSrcLine(t) => Ok(hash_u32(t.ty.as_bytes())),
        _ => Ok(crate::hash::hash_sig(record_bytes, 0)),
    }
}

fn hash_udt_name(
    prop: UdtProperties,
    record_bytes: &[u8],
    name: &BStr,
    unique_name: Option<&BStr>,
) -> u32 {
    if !prop.fwdref() && !is_udt_anon_name(name) {
        if prop.scoped() {
            if let Some(unique_name) = unique_name {
                return hash_u32(unique_name);
            }
        } else {
            // This branch is equivalent to the case handled by fIsGlobalDefUdt().
            return hash_u32(name);
        }
    }

    crate::hash::hash_sig(record_bytes, 0)
}

/// Tests if `name` is indicates that this UDT is an anonymous UDT.
pub fn is_udt_anon_name(name: &BStr) -> bool {
    name == "<unnamed-tag>"
        || name == "__unnamed"
        || name.ends_with(b"::<unnamed-tag>")
        || name.ends_with(b"::__unnamed")
}
