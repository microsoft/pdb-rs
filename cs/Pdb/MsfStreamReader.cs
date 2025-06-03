using Pdb;
using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Threading.Tasks;

namespace Pdb;

public class MsfStreamReader : IMsfStreamReader {
    readonly MsfReader _msf;
    readonly int _stream;
    readonly int _firstPagePointer;
    readonly int _numPages;

    /// <summary>
    /// Size in bytes of the stream.  If the stream is a nil stream, this returns 0.
    /// </summary>
    readonly uint _streamSize;

    public ulong StreamSize {
        get {
            return _streamSize;
        }
    }

    internal MsfStreamReader(MsfReader msf, int stream) {
        _msf = msf;
        _stream = stream;

        uint streamSize = _msf._streamSizes[this._stream];
        if (streamSize == MsfDefs.NilStreamSize) {
            streamSize = 0;
        }
        _streamSize = streamSize;

        _firstPagePointer = _msf._streamPageStarts[stream];
        _numPages = _msf._streamPageStarts[stream + 1] - _firstPagePointer;
    }

    public int ReadAt(long streamOffset, in Span<byte> buffer) {
        int totalBytesRead = 0;

        int pageSizeShift = _msf._pageSizeShift;

        uint streamSize = _streamSize;
        if (streamSize == 0) {
            return 0;
        }

        Span<uint> streamPages = new Span<uint>(_msf._allStreamPages, _firstPagePointer, _numPages);

        while (buffer.Length != 0 && streamOffset < streamSize) {
            int maxReadSize = (int)(streamSize - streamOffset);

            // Find the page where the transfer starts
            uint firstStreamPage = streamPages[(int)(streamOffset >> pageSizeShift)];

            // Handle the range of bytes on the first page. The first page is different because
            // the transfer may start at a boundary that is not page-aligned and may even end on
            // a boundary that is not page-aligned.

            throw null;
        }

        return totalBytesRead;
    }
}
