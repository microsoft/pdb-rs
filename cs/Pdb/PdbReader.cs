#if todo

using System;
using System.IO;

namespace Pdb;

public sealed class PdbReader : IDisposable {
    readonly MsfReader _reader;

    Modules _modules;

    DbiStreamInfo _dbiStreamInfo;

    public void Dispose() {
        _reader.Dispose();
    }

    public static PdbReader Open(string fileName) {
        FileStream f = FileStream.Open(fileName);
        return Open(f);
    }

    public static PdbReader Open(Stream stream) {
    }

    public MsfReader Msf {
        get { return self._reader; }
    }

    public DbiStreamInfo GetDbiStreamInfo() {
        if (_dbiStreamInfo != null) {
            return _dbiStreamInfo;
        }

        DbiStreamInfo dbiStreamInfo = ReadDbiStreamInfo();
        _dbiStreamInfo = dbiStreamInfo;

        return dbiStreamInfo;
    }

    public Modules GetModules() {
        if (_modules == null) {
            Modules modules = ReadModules();
            _modules = modules;
            return modules;
        }

        return _modules;
    }
}

public class Modules {
}

public class ModuleInfo {
    public string Name;
    public string OtherName;
    public uint Stream;
}

#endif
