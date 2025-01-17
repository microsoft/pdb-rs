use anyhow::Result;
use std::io::{Cursor, Write};

pub fn show_comp_rate(description: &str, before: u64, after: u64) {
    if before == 0 {
        // We don't divide by zero around here.
        println!(
            "    {:-30} : {:8} -> {:8}",
            description,
            friendly::bytes(before),
            friendly::bytes(after)
        );
    } else {
        let percent = (before as f64 - after as f64) / (before as f64) * 100.0;
        println!(
            "    {:-30} : {:8} -> {:8} {:2.1} %",
            description,
            friendly::bytes(before),
            friendly::bytes(after),
            percent
        );
    }
}

#[allow(dead_code)] // useful for work in progress
pub fn zstd_compress(input: &[u8]) -> Result<Vec<u8>> {
    let mut output: Vec<u8> = Vec::new();

    let mut encoder = zstd::Encoder::new(Cursor::new(&mut output), 0)?;
    encoder.write_all(input)?;
    encoder.flush()?;
    drop(encoder);

    Ok(output)
}

#[allow(dead_code)] // useful for work in progress
pub fn zstd_compress_and_show(input: &[u8], description: &str) {
    let output = zstd_compress(input).unwrap();
    show_comp_rate(description, input.len() as u64, output.len() as u64);
}
