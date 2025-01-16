//! TMCache Stream
//!
//! Code for reading the TMCache stream

#![allow(missing_docs)]

use crate::utils::io::{read_boxed_slice_at, read_struct_at};
use crate::utils::swizzle::Swizzle;
use anyhow::{bail, Result};
use std::mem::size_of;
use sync_file::ReadAt;
use zerocopy::{AsBytes, FromBytes, FromZeroes, Unaligned, LE, U32};

/// The name of the stream that contains the TMCache.
pub const TMCACHE_STREAM_NAME: &str = "/TMCache";

#[derive(AsBytes, FromBytes, FromZeroes, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct TMCacheHeader {
    pub version: U32<LE>,
    pub tm_table_offset: U32<LE>,
    pub tm_table_size: U32<LE>,
    pub module_table_offset: U32<LE>,
    pub module_table_size: U32<LE>,
}

#[derive(AsBytes, FromBytes, FromZeroes, Clone, Debug)]
#[repr(C)]
pub struct TMCacheModule {
    pub tm_index: u32,
    pub reserved: u32,
    pub checksum: u64,
}

pub struct TMCacheTable {
    pub module_table: Box<[TMCacheModule]>,
    /// Contains stream indexes.
    pub tm_table: Box<[u16]>,
}

pub const TM_CACHE_HEADER_VERSION_V1: u32 = 0x20200229;

impl TMCacheTable {
    pub fn read<R: ReadAt>(reader: &mut R) -> Result<Self> {
        let mut header = TMCacheHeader::new_zeroed();
        reader.read_exact_at(header.as_bytes_mut(), 0)?;

        if header.version.get() != TM_CACHE_HEADER_VERSION_V1 {
            bail!("The TMCache stream is using a version number that is not recognized. Version: 0x{:08x}",
            header.version.get());
        }

        // Validate module table location
        let module_table = {
            let module_table_offset = header.module_table_offset.get() as usize;
            let module_table_size_bytes = header.module_table_size.get() as usize;

            if module_table_size_bytes % size_of::<TMCacheModule>() != 0 {
                bail!(
                "The TMCache stream is invalid.  The module_table_size field ({module_table_size_bytes}) \
                 is not a multiple of the element size ({}).",
                 size_of::<TMCacheModule>()
            );
            }

            let module_table_len = module_table_size_bytes / size_of::<TMCacheModule>();
            let mut module_table: Box<[TMCacheModule]> =
                TMCacheModule::new_box_slice_zeroed(module_table_len);

            reader.read_exact_at(module_table.as_bytes_mut(), module_table_offset as u64)?;

            if cfg!(target_endian = "big") {
                // swizzle values
                for m in module_table.iter_mut() {
                    m.reserved = u32::from_le(m.reserved);
                    m.tm_index = u32::from_le(m.tm_index);
                    m.checksum = u64::from_le(m.checksum);
                }
            }
            module_table
        };

        // tm_table
        let tm_table = {
            let tm_table_offset = header.tm_table_offset.get();
            let tm_table_size_bytes = header.tm_table_size.get() as usize;
            if tm_table_size_bytes % size_of::<u16>() != 0 {
                bail!(
                    "The TMCache stream is invalid. The tm_table_size field ({tm_table_size_bytes})
                is not a multiple of the element size ({}).",
                    size_of::<u16>()
                );
            }

            let tm_cache_len = tm_table_size_bytes / size_of::<u16>();
            let mut tm_table: Box<[u16]> = u16::new_box_slice_zeroed(tm_cache_len);

            reader.read_exact_at(tm_table.as_bytes_mut(), tm_table_offset as u64)?;

            if cfg!(target_endian = "big") {
                // swizzle values
                for i in tm_table.iter_mut() {
                    *i = u16::from_le(*i);
                }
            }
            tm_table
        };

        Ok(Self {
            tm_table,
            module_table,
        })
    }
}

pub enum TMCache {
    Tmts(Tmts),
    Tmr(Tmr),
    Tmpct(Tmpct),
    TmpctOwner(Tmpct),
}

pub struct Tmts {
    pub ti_mapped_to: Box<[u32]>,
    pub id_mapped_to: Box<[u32]>,
}

pub struct Tmr {
    pub ti_mapped_to: Box<[u32]>,
}

// If the flag says it is TMR or TMPCT, then we have
//
//   DWORD          sn of TMPCT
//   DWORD          signature
//   DWORD          tiStart
//   DWORD          ctiFrom
//   vector<DWORD>  tiMappedTo
//   vector<DWORD>  offLeafRecord
//   DWORD          cbTypes
//   pair<ID, TI>   funcID, funcTI
#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
pub struct TmrHeader {
    /// Stream that contains TMPCT
    pub tmpct_stream: U32<LE>,
    pub signature: U32<LE>,
    pub ti_start: U32<LE>,
    pub cti_from: U32<LE>,
}

pub struct Tmpct {}

pub const TMCACHE_KIND_TMTS: u32 = 1;
pub const TMCACHE_KIND_TMR: u32 = 2;
pub const TMCACHE_KIND_TMPCT: u32 = 4;
pub const TMCACHE_KIND_TMPCT_OWNER: u32 = 8;

#[repr(C)]
#[derive(AsBytes, FromBytes, FromZeroes)]
struct TmtsHeader {
    cti_from: U32<LE>,
    cid_from: U32<LE>,
}

impl TMCache {
    pub fn read<R: ReadAt>(r: &mut R) -> anyhow::Result<Self> {
        let mut flags: U32<LE> = FromZeroes::new_zeroed();
        r.read_exact_at(flags.as_bytes_mut(), 0)?;
        let mut pos: u64 = 4;

        match flags.get() {
            TMCACHE_KIND_TMTS => {
                // If the flag says it is TMTS, then we have
                //
                //   DWORD          ctiFrom
                //   DWORD          cidFrom
                //   vector<DWORD>  tiMappedTo
                //   vector<DWORD>  idMappedTo
                //   pair<ID, TI>   funcID, funcTI

                let tmts: TmtsHeader = read_struct_at(r, pos)?;
                pos += size_of::<TmtsHeader>() as u64;

                let num_cti_from = tmts.cti_from.get() as usize;
                let num_cid_from = tmts.cid_from.get() as usize;
                let mut ti_mapped_to: Box<[u32]> = read_boxed_slice_at(r, pos, num_cti_from)?;
                pos += ti_mapped_to.as_bytes().len() as u64;

                ti_mapped_to.le_to_host();
                let mut id_mapped_to: Box<[u32]> = read_boxed_slice_at(r, pos, num_cid_from)?;
                pos += id_mapped_to.as_bytes().len() as u64;
                id_mapped_to.le_to_host();

                let _ = pos;

                Ok(Self::Tmts(Tmts {
                    id_mapped_to,
                    ti_mapped_to,
                }))
            }

            TMCACHE_KIND_TMR => {
                let tmr_header: TmrHeader = read_struct_at(r, pos)?;
                pos += size_of::<TmrHeader>() as u64;

                let cti_from = tmr_header.cti_from.get() as usize;

                let mut ti_mapped_to: Box<[u32]> = read_boxed_slice_at(r, pos, cti_from)?;
                pos += ti_mapped_to.as_bytes().len() as u64;
                ti_mapped_to.le_to_host();

                let mut off_leaf_record: Box<[u32]> = read_boxed_slice_at(r, pos, cti_from)?;
                pos += off_leaf_record.as_bytes().len() as u64;
                off_leaf_record.le_to_host();

                let _cb_types: u32 = u32::from_le(read_struct_at(r, pos)?);

                Ok(Self::Tmr(Tmr { ti_mapped_to }))
            }

            TMCACHE_KIND_TMPCT => {
                todo!("TMCACHE_KIND_TMPCT");
            }

            TMCACHE_KIND_TMPCT_OWNER => {
                todo!("TMCACHE_KIND_TMPCT_OWNER");
            }

            _ => {
                bail!("TMCache stream kind 0x{:08x} is unrecognized", flags);
            }
        }
    }
}
