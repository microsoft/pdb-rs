use anyhow::Result;
use dump_utils::HexDump;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use structopt::StructOpt;

use crate::util::HexU64;

/// Dumps the contents of a file as a hex dump.
#[derive(StructOpt)]
pub struct HexdumpOptions {
    /// The file to dump
    pub file: String,
    /// Offset within the file. Defaults to 0.
    pub offset: Option<HexU64>,
    /// Max length of the data to dump. Defaults to 0x1000.
    pub len: Option<HexU64>,
}

pub fn command(options: HexdumpOptions) -> Result<()> {
    let mut f = File::open(&options.file)?;

    let offset: u64 = if let Some(offset) = options.offset {
        offset.0
    } else {
        0
    };

    f.seek(SeekFrom::Start(offset))?;

    let len: usize = if let Some(len) = options.len {
        len.0 as usize
    } else {
        0x1000
    };

    let mut buffer: Vec<u8> = vec![0; len];

    let n = f.read(buffer.as_mut_slice())?;
    let bytes = &buffer[..n];

    print!("{}", HexDump::new(bytes).at(offset as usize));

    Ok(())
}
