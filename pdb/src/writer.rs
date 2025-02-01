//! Utilties for copying data between streams.

use std::io::{Read, Write};

/// Copies all data from `Src` to `Dst`
pub fn copy_stream_with_buffer<Dst, Src>(
    mut dst: Dst,
    mut src: Src,
    buffer: &mut [u8],
) -> std::io::Result<()>
where
    Dst: Write,
    Src: Read,
{
    loop {
        let n = src.read(buffer)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buffer[..n])?;
    }

    Ok(())
}

/// Copies all data from `Src` to `Dst`
pub fn copy_stream<Dst, Src>(dst: Dst, src: Src) -> std::io::Result<()>
where
    Dst: Write,
    Src: Read,
{
    const BUFFER_LEN: usize = 16 << 20; // 16 MiB

    let mut buffer = vec![0; BUFFER_LEN];
    copy_stream_with_buffer(dst, src, &mut buffer)
}
