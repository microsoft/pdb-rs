using System;
using System.Diagnostics;
using System.IO;

namespace Pdb;

public sealed class MsfReader : IMsfReader, IDisposable
{
    internal readonly System.IO.Stream _file;

    /// <summary>
    /// The size in bytes of each stream, or `NilStreamSize` if the stream is a nil stream.
    /// </summary>
    internal readonly uint[] _streamSizes;

    /// <summary>
    /// indexed by stream
    /// len = num_streams + 1
    /// points into allStreamPages, e.g. allStreamPages[streamPageStarts[s] .. streamPageStarts[s + 1]]
    /// _streamPageStarts[0] == 0
    /// </summary>
    internal readonly int[] _streamPageStarts;

    // _streamPageStarts[num_streams] == _allStreamPages.Length
    internal readonly uint[] _allStreamPages;

    /// <summary>
    /// Number of bits in the page size, e.g. 12 for 4096 bytes per page
    /// </summary>
    internal readonly int _pageSizeShift;

    public int NumStreams { get { return _streamSizes.Length; } }

    public long StreamSize(int stream) {
        if (stream <= 0 || stream >= _streamSizes.Length) {
            return 0;
        }

        uint streamSize = _streamSizes[stream];
        if (streamSize == MsfDefs.NilStreamSize) {
            return 0;
        }

        return streamSize;
    }

    public bool IsStreamValid(int stream)
    {
        if (stream <= 0 || stream >= _streamSizes.Length) {
            return false;
        }

        uint size = _streamSizes[stream];
        if (size == MsfDefs.NilStreamSize) {
            return false;
        }

        return true;
    }

    public IMsfStreamReader GetStreamReader(int stream)
    {
        return new MsfStreamReader(this, stream);
    }

    public void Dispose()
    {
        this._file.Dispose();
    }

    public static MsfReader Open(Stream file)
    {
        // Read the file header and check its signature.
        byte[] fileHeader = new byte[MsfDefs.FileHeaderSize];
        IOUtils.SeekRead(file, 0, fileHeader);

        // fh = "file header"
        var fh = new Bytes(new Span<byte>(fileHeader));
        Span<byte> signature = fh.ReadN(32);

        if (!MsfUtils.SpanEq(signature, new Span<byte>(MsfDefs.FileSignature))) {
            throw new Exception("File signature is wrong (is not an MSF file)");
        }

        uint pageSize = fh.ReadUInt32();
        uint activeFpm = fh.ReadUInt32(); // we don't use the FPM
        uint numPages = fh.ReadUInt32();
        uint streamDirSize = fh.ReadUInt32();
        // <-- fh is now positioned on stream dir page map; we will read it, below.

        // Find log2 of the page size. Page size is required to be a power of 2.
        int pageSizeShift = MsfUtils.GetLog2OfPageSize(pageSize);

        // This field is ignored
        uint smallStreamDirPage = fh.ReadUInt32();

        byte[] streamDirBytes = ReadStreamDirectory(file, pageSizeShift, fh._data, (int)streamDirSize);

        // Now that the entire Stream Directory has been read into streamDirBytes, parse it
        // and build the streamSizes, allStreamPages, and streamPageStarts tables.

        Bytes sd = new Bytes(streamDirBytes);

        uint numStreamsUInt32 = sd.ReadUInt32();
        if (numStreamsUInt32 < 1 || numStreamsUInt32 >= 0xfffe) {
            throw new Exception("The MSF file is invalid. The Stream Directory contains too many streams.");
        }
        int numStreams = (int)numStreamsUInt32;

        uint[] streamSizes = new uint[numStreams];
        int[] streamPageStarts = new int[numStreams + 1];
        streamPageStarts[0] = 0; // Redundant, but explicit.

        // Read the stream sizes and build the streamSizes table.
        // At the same time, build the streamPageStarts table.
        int nextPageStart = 0;
        for (int i = 0; i < numStreams; ++i) {
            uint streamSize = sd.ReadUInt32();
            streamSizes[i] = streamSize;

            // Find out how many pages this stream requires, taking care not
            // to assign pages to nil streams.
            if (streamSize != MsfDefs.NilStreamSize) {
                int streamNumPages = MsfUtils.NumPagesForBytes(streamSize, pageSizeShift);
                nextPageStart += streamNumPages;
            }
            streamPageStarts[i + 1] = nextPageStart;
        }

        // At this point, nextPageStart tells us how many total pages were assigned to all streams.
        // Now we can allocate allStreamPages and read it from the Stream Directory.
        uint[] allStreamPages = new uint[nextPageStart];
        for (int i = 0; i < allStreamPages.Length; ++i) {
            uint streamPage = sd.ReadUInt32();
            allStreamPages[i] = streamPage;
        }

        return new MsfReader(
            file: file,
            streamSizes: streamSizes,
            pageSizeShift: pageSizeShift,
            allStreamPages: allStreamPages,
            streamPageStarts: streamPageStarts
        );
    }

    MsfReader(System.IO.Stream file, int pageSizeShift, uint[] streamSizes, int[] streamPageStarts, uint[] allStreamPages) {
        _file = file;
        _pageSizeShift = pageSizeShift;
        _streamPageStarts = streamPageStarts;
        _streamSizes = streamSizes;
        _allStreamPages = allStreamPages;
    }

    static byte[] ReadStreamDirectory(System.IO.Stream file, int pageSizeShift, in Span<byte> pageMapBytes, int streamDirSize) {
        if (streamDirSize % 4 != 0) {
            throw new Exception("Stream directory size is invalid (is not a multiple of 4)");
        }
        int streamDirSizeUInts = (int)(streamDirSize / 4);

        byte[] streamDirBytes = new byte[streamDirSize];

        int numStreamDirPages = MsfUtils.NumPagesForBytes((uint)streamDirSize, pageSizeShift);
        int numStreamMapPages = MsfUtils.NumPagesForBytes((uint)numStreamDirPages * 4, pageSizeShift);

        byte[] pageMapBuffer = new byte[1 << pageSizeShift];

        // Read each map page and then use it to read each of the pages of the Stream Directory

        // offset in bytes of the next chunk of the Stream Directory to read

        int streamDirOffset = 0;

        Bytes rootReader = new Bytes(pageMapBytes);

        // one iteration per map page
        while (streamDirOffset < streamDirSize) {
            // Read the page number of the next map page from the file header
            uint pageMapPage = rootReader.ReadUInt32();
            IOUtils.SeekRead(file, ((long)pageMapPage) << pageSizeShift, pageMapBuffer);

            // pageMapBuffer now contains page numbers for stream directory

            // one iteration per dir page
            // TODO: find sequential pages and coalesce reads
            Bytes mapBytes = new Bytes(pageMapBuffer);
            while (streamDirOffset < streamDirSize) {
                if (!mapBytes.HasN(4)) {
                    break;
                }

                uint dirPage = mapBytes.ReadUInt32();

                int readSize = Math.Min(streamDirSize - streamDirOffset, 1 << pageSizeShift);

                IOUtils.SeekRead(file, ((long)dirPage) << pageSizeShift,
                    new Span<byte>(streamDirBytes, streamDirOffset, readSize));

                streamDirOffset += readSize;
            }
        }

        return streamDirBytes;
    }

}



