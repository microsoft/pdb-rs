//! Code for reading data from streams

use sync_file::ReadAt;
use tracing::{trace, trace_span};

use crate::pages::StreamPageMapper;
use crate::{Page, PageSize};

/// This reads data from a stream. It maps byte offsets within a stream to byte offsets within the
/// containing MSF file.
///
/// It will read as much data in a single `read()` call (to the underlying storage) as it can,
/// provided the pages within the stream are contiguous.
///
/// Returns `(bytes_transferred, new_pos)`, where `new_pos` is the position within the stream
/// after the last byte was read. If no bytes were transferred, then this is the same as `pos`.
/// Note that it is possible for `pos` (and thus `new_pos`) to be greater than `stream_size`.
pub(super) fn read_stream_core<F: ReadAt>(
    stream: u32,
    file: &F,
    page_size: PageSize,
    stream_size: u32,
    pages: &[Page],
    stream_pos: u64,
    dst: &mut [u8],
) -> std::io::Result<(usize, u64)> {
    let _span = trace_span!("read_stream_core").entered();

    // Early out for a read at the end. This also handles checking the 64-bit stream position
    // vs 32-bit, so we can safely cast to u32 after this check.
    if stream_pos >= stream_size as u64 {
        return Ok((0, stream_pos));
    }

    let mut stream_pos = stream_pos as u32;

    let original_len = dst.len();
    let mut remaining_dst = dst;

    let mapper = StreamPageMapper::new(pages, page_size, stream_size);

    while !remaining_dst.is_empty() && stream_pos < stream_size {
        let Some((file_offset, transfer_size)) = mapper.map(stream_pos, remaining_dst.len() as u32)
        else {
            break;
        };

        let (dst_this_transfer, dst_next) = remaining_dst.split_at_mut(transfer_size as usize);

        trace!(
            stream,
            stream_pos,
            transfer_size,
            file_offset,
            "reading stream data"
        );

        file.read_exact_at(dst_this_transfer, file_offset)?;

        stream_pos += transfer_size;
        remaining_dst = dst_next;
    }

    let total_bytes_transferred = original_len - remaining_dst.len();
    Ok((total_bytes_transferred, stream_pos as u64))
}
