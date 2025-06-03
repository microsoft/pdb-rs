#if TODO

namespace Pdb;

public sealed class DbiStreamInfo {
    public uint version;
    public uint age;
    public ushort GlobalSymbolIndexStream;
    public ushort BuildNumber;
    public ushort PublicSymbolIndexStream;
    public ushort PdbDllVersion;
    public ushort GlobalSymbolStream;
    public ushort PdbDllRbld;

    // Substreams
    public int mod_info_size;
    public int section_contribution_size;
    public int section_map_size;
    public int source_info_size;
    public int type_server_map_size;
    /// This field is _not_ a substream size. Not sure what it is.
    public uint mfc_type_server_index;
    public int optional_dbg_header_size;
    public int edit_and_continue_size;

    public ushort flags;
    public ushort machine;
    public uint padding;

    public const int DbiStreamHeaderSize = 64;

    public static DbiStreamInfo Read(MsfReader msf, uint stream) {

        var sr = msf.GetStreamReader(stream);

        byte[] buf = new byte[DbiStreamHeaderSize];

        sr.ReadAt(0, buf);

        var br = new System.IO.ByteReader(buf);

        uint version = br.ReadUInt32();
        uint age = br.ReadUInt32();

        ushort GlobalSymbolIndexStream = br.ReadUInt16();
        ushort BuildNumber = br.ReadUInt16();
        ushort PublicSymbolIndexStream = br.ReadUInt16();
        ushort PdbDllVersion = br.ReadUInt16();
        ushort GlobalSymbolStream = br.ReadUInt16();
        ushort PdbDllRbld = br.ReadUInt16();

    // Substreams
        int mod_info_size = br.ReadInt32();
        int section_contribution_size = br.ReadInt32();
        int section_map_size = br.ReadInt32();
        int source_info_size = br.ReadInt32();
        int type_server_map_size = br.ReadInt32();
        uint mfc_type_server_index = br.ReadUInt32();
        int optional_dbg_header_size = br.ReadInt32();
        int edit_and_continue_size = br.ReadInt32();

        ushort flags = br.ReadUInt16();
        ushort machine = br.ReadUInt16();
        uint padding = br.ReadUInt32();
    }

}

#endif
