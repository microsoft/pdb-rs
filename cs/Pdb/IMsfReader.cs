
namespace Pdb;

public interface IMsfReader {
    int NumStreams { get; }
    bool IsStreamValid(int stream);
    long StreamSize(int stream);

    IMsfStreamReader GetStreamReader(int stream);
}

public interface IMsfStreamReader {
    ulong StreamSize { get; }

    int ReadAt(long streamOffset, in Span<byte> buffer);
}
