use std::io::{Read, Write};

use crate::Compression;

pub(crate) fn compress_to_vec(compression: Compression, input: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    compress_to_vec_mut(compression, input, &mut output)?;
    Ok(output)
}

pub(crate) fn compress_to_vec_mut(
    compression: Compression,
    input: &[u8],
    output: &mut Vec<u8>,
) -> std::io::Result<()> {
    match compression {
        Compression::Zstd => {
            let mut enc = zstd::Encoder::new(output, 0)?;
            enc.write_all(input)?;
            enc.finish()?;
        }

        Compression::Deflate => {
            let mut enc = flate2::write::DeflateEncoder::new(
                std::io::Cursor::new(output),
                flate2::Compression::default(),
            );

            enc.write_all(input)?;
            enc.finish()?;
        }
    }

    Ok(())
}

/// Decompresses a compressed buffer using the given compression algorithm.
///
/// `output.len()` specifies the expected size of the decoded stream. Returns `Err` if the
/// decompression algorithm returned the wrong number of bytes.
pub(crate) fn decompress_to_slice(
    compression: Compression,
    input: &[u8],
    output: &mut [u8],
) -> std::io::Result<()> {
    match compression {
        Compression::Zstd => {
            let mut dec = zstd::Decoder::new(input)?;
            dec.read_exact(output)?;
        }

        Compression::Deflate => {
            let mut dec = flate2::read::DeflateDecoder::new(input);
            dec.read_exact(output)?;
        }
    };

    Ok(())
}
