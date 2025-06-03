
namespace Pdb;

internal ref struct Bytes {
    internal Bytes(Span<byte> data) {
        _data = data;
    }

    internal Span<byte> _data;

    internal bool HasN(int n) {
        return _data.Length >= n;
    }

    internal Span<byte> ReadN(int n) {
        if (_data.Length < n) {
            throw new Exception("Not enough data");
        }
        Span<byte> s = _data.Slice(0, n);
        _data = _data.Slice(n);
        return s;
    }

    internal byte ReadByte() {
        var data = ReadN(1);
        return data[0];
    }

    internal ushort ReadUInt16() {
        var data = ReadN(2);
        uint b0 = data[0];
        uint b1 = data[1];
        return (ushort)(b0 | (b1 << 8));
    }

    internal short ReadInt16() {
        return (short)ReadUInt16();
    }

    internal uint ReadUInt32() {
        var data = ReadN(4);
        uint b0 = data[0];
        uint b1 = data[1];
        uint b2 = data[2];
        uint b3 = data[3];
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }

    internal int ReadInt32() {
        return (int)ReadUInt32();
    }

    internal ulong ReadUInt64() {
        var data = ReadN(8);
        ulong b0 = data[0];
        ulong b1 = data[1];
        ulong b2 = data[2];
        ulong b3 = data[3];
        ulong b4 = data[4];
        ulong b5 = data[5];
        ulong b6 = data[6];
        ulong b7 = data[7];
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            | (b4 << 32) | (b5 << 40) | (b6 << 48) | (b7 << 56);
    }

    internal int ReadInt64() {
        return (int)ReadUInt64();
    }

    // Get* methods read at an absolute index and do not move the read cursor

    void NeedsBytes(int n) {
        if (_data.Length < n) {
            throw new Exception("Not enough data");
        }
    }

    internal Span<byte> GetN(int offset, int n) {
        if (offset > _data.Length) {
            throw new Exception("Not enough data");
        }

        int avail = _data.Length - offset;
        if (avail < n) {
            throw new Exception("Not enough data");
        }

        return _data.Slice(offset, n);
    }

    internal byte GetByte(int offset) {
        var data = GetN(offset, 1);
        return data[0];
    }


    internal ushort GetUInt16(int start) {
        var data = GetN(start, 2);
        uint b0 = data[0];
        uint b1 = data[1];
        return (ushort)(b0 | (b1 << 8));
    }

    internal short GetInt16(int start) {
        return (short)GetUInt16(start);
    }

    internal uint GetUInt32(int start) {
        var data = GetN(start, 4);
        uint b0 = data[0];
        uint b1 = data[1];
        uint b2 = data[2];
        uint b3 = data[3];
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24);
    }

    internal int GetInt32(int start) {
        return (int)GetUInt32(start);
    }

    internal uint GetUInt64(int start) {
        var data = GetN(start, 8);
        uint b0 = data[0];
        uint b1 = data[1];
        uint b2 = data[2];
        uint b3 = data[3];
        uint b4 = data[4];
        uint b5 = data[5];
        uint b6 = data[6];
        uint b7 = data[7];
        return b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
            | (b4 << 32) | (b5 << 40) | (b6 << 48) | (b7 << 56);
    }

    internal int GetInt64(int start) {
        return (int)GetUInt64(start);
    }
}
