using System;
using System.Diagnostics;

namespace Pdb;


ref struct StreamPageMapper
{
    readonly Span<uint> _pages;
    readonly int _pageSizeShift;
    readonly uint _streamSize;

    internal StreamPageMapper(
        Span<uint> pages,
        int pageSizeShift,
        uint streamSize)
    {
        Debug.Assert(MsfUtils.NumPagesForBytes(streamSize, pageSizeShift) == pages.Length);

        this._pages = pages;
        this._pageSizeShift = pageSizeShift;
        this._streamSize = streamSize;
    }

    /// Maps a byte offset and a length within a stream to a contiguous run of bytes within the MSF file.
    ///
    /// Repeated calls to this function (with increasing values of `pos`) can be used to read/write
    /// the contents of a stream using the smallest number of read/write calls to the underlying
    /// MSF file.
    ///
    /// Returns `(file_offset, transfer_len)` where `file_offset` is the byte offset within the MSF
    /// file and `transfer_len` is the length of the longest contiguous sub-range of the requested
    /// range.
    ///
    /// If this returns `None` then no bytes can be mapped. This occurs when `pos >= stream_size`.
    ///
    /// Invariants:
    ///
    /// * if returned `Some`, then `transfer_len <= bytes_wanted`
    /// * if returned `Some`, then `transfer_len > 0`
    public (long fileOffset, uint transferLen) Map(uint pos, uint bytesWanted) {
        Debug.Assert(bytesWanted > 0);

        if (this._streamSize == MsfDefs.NilStreamSize) {
            return (-1, 0);
        }

        if (pos >= this._streamSize) {
            return (-1, 0);
        }

        uint bytes_available = this._streamSize - pos;
        uint max_transfer_size = Math.Min(bytes_available, bytesWanted);

        if (max_transfer_size == 0) {
            return (0, 0);
        }

        // We will reduce transfer_size as needed.
        uint transfer_size;

        // Find the position within the file where the read will start.
        int first_page_index = (int)(pos >> this._pageSizeShift);
        uint first_page_pointer = this._pages[first_page_index];
        long first_page_file_offset = ((long)first_page_pointer) << this._pageSizeShift;

        uint offset_within_first_page = pos - MsfUtils.AlignDownToPageSize(pos, this._pageSizeShift);
        long file_offset = first_page_file_offset + (long)offset_within_first_page;

        // Find the longest read we can execute in a single underlying read call.
        // If pages are numbered consecutively, then cover as many pages as we can.

        // Does the beginning of the read cross a page boundary?
        uint bytes_available_first_page = (1u << this._pageSizeShift) - offset_within_first_page;
        if (max_transfer_size > bytes_available_first_page) {
            // Yes, this read crosses a page boundary.
            // Set transfer_size to just the bytes in the first page.
            // Then, keep advancing through the page list as long as pages are sequential.
            uint p = pos + bytes_available_first_page;
            // Debug.Assert(this.page_size.is_aligned(p));

            uint last_page_ptr = first_page_pointer;

            while (true) {
                Debug.Assert(
                    p - pos <= max_transfer_size,
                    "p = {p}, max_transfer_size = {max_transfer_size}"
                );
                uint want_bytes = max_transfer_size - (p - pos);

                if (p - pos == max_transfer_size) {
                    // Reached max transfer size.
                    break;
                }

                int p_page = (int)(p >> this._pageSizeShift);

                uint p_ptr = this._pages[p_page];
                Debug.Assert(p_page > first_page_index);

                if (p_ptr != last_page_ptr + 1) {
                    // The pages are not contiguous, so we stop here.
                    break;
                }

                // Advance over this page.
                p += Math.Min(want_bytes, 1u << this._pageSizeShift);
                last_page_ptr += 1;
            }

            transfer_size = p - pos;
        } else {
            // This range does not cross a page boundary; it fits within a single page.
            transfer_size = max_transfer_size;
        }

        Debug.Assert(transfer_size > 0);

        Debug.Assert(
            transfer_size <= bytesWanted
            // "transfer_size = {}, bytes_wanted = {}",
            // transfer_size,
            // bytes_wanted
        );

        return (file_offset, transfer_size);
    }


}
