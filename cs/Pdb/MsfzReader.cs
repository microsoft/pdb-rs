#if TODO

using System;
using System.IO;

namespace Pdb;

public sealed class MsfzReader : IMsfReader {
    uint NumStreams { get; }
    bool IsStreamValid(uint stream);
    ulong StreamSize(uint stream);

    IMsfStreamReader GetStreamReader(uint stream);
}

#endif
