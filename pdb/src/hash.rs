//! MSVC hash algorithms

use zerocopy::{FromBytes, LE, U16, U32};

#[cfg(test)]
use pretty_hex::PrettyHex;

/// Computes a 32-bit hash. This produces the same results as the hash function used in the
/// MSVC PDB reader library.
///
/// This is a port of the `LHashPbCb` function.
///
/// # WARNING! WARNING! WARNING!
///
/// This is a **VERY POOR HASH FUNCTION** and it should not be used for *ANY* new code. This
/// function should only be used for compatibility with PDB data structures.
///
/// # References
///
/// * [`misc.h](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/PDB/include/misc.h#L15)
pub fn hash_mod_u32(pb: &[u8], m: u32) -> u32 {
    hash_u32(pb) % m
}

/// Computes a 32-bit hash, but does not compute a remainder (modulus).
#[inline(never)]
pub fn hash_u32(mut pb: &[u8]) -> u32 {
    let mut h: u32 = 0;
    if let Ok((u32s, tail)) = <[U32<LE>]>::ref_from_prefix(pb) {
        for u in u32s.iter() {
            h ^= u.get();
        }
        pb = tail;
    }

    // The tail is handled differently.
    if let Ok((tail_u16, rest)) = <U16<LE>>::read_from_prefix(pb) {
        h ^= tail_u16.get() as u32;
        pb = rest;
    }

    debug_assert!(pb.is_empty() || pb.len() == 1);

    if !pb.is_empty() {
        h ^= pb[0] as u32;
    }

    h |= 0x20202020;
    h ^= h >> 11;
    h ^ (h >> 16)
}

/// Computes a 16-bit hash
///
/// This is a port of the `HashPbCb` function.
pub fn hash_mod_u16(pb: &[u8], m: u32) -> u16 {
    hash_mod_u32(pb, m) as u16
}

#[test]
fn test_hash() {
    static INPUTS: &[(u32, &[u8])] = &[
        (0x00000c09, b""),
        (0x00000c09, b" "),
        (0x00000c09, b"  "),
        (0x00000c09, b"   "),
        (0x00000c09, b"    "),
        (0x00019fe2, b"hello"),
        (0x00019fe2, b"HELLO"),
        (0x0003c00b, b"Hello, World"),
        (0x0003c00b, b"hello, world"),
        (0x000068e2, b"hello_world::main"),
        (0x0000b441, b"std::vector<std::basic_string<wchar_t>>"),
        (0x000372ae, b"__chkstk"),
        (0x0001143b, b"WelsEmms"),
        (0x00000c0a, &[1]),
        (0x00000e0a, &[1, 2]),
        (0x00000e0b, &[1, 2, 3]),
        (0x00038b6b, &[1, 2, 3, 4]),
        (0x00038b70, &[1, 2, 3, 4, 5]),
        (0x00038d70, &[1, 2, 3, 4, 5, 6]),
        (0x00038d69, &[1, 2, 3, 4, 5, 6, 7]),
        (0x00019789, &[1, 2, 3, 4, 5, 6, 7, 8]),
        (0x00019790, &[1, 2, 3, 4, 5, 6, 7, 8, 9]),
        (0x00019191, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
        (0x0001918a, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
        (0x000313ed, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        (0x000313f8, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        (0x000214eb, &[5, 6, 7, 8]),
    ];

    for &(expected_output, input) in INPUTS.iter() {
        let m = 0x3_ffff;
        let actual_output = hash_mod_u32(input, m);
        assert_eq!(
            expected_output, actual_output,
            "expected: 0x{expected_output:08x}, actual: 0x{actual_output:08x}, input: {input:02x?}"
        );
    }
}

/// Computes a hash code using an algorithm equivalent to the `SigForPbCb` function.
/// This is a CRC-32 checksum with an initial value of `sig`.
///
/// See `SigForPbCb` in <https://github.com/microsoft/microsoft-pdb/blob/master/langapi/shared/crc32.h>
pub fn hash_sig(pb: &[u8], mut sig: u32) -> u32 {
    static RGCRC: [u32; 0x100] = [
        0x00000000, 0x77073096, 0xEE0E612C, 0x990951BA, 0x076DC419, 0x706AF48F, 0xE963A535,
        0x9E6495A3, 0x0EDB8832, 0x79DCB8A4, 0xE0D5E91E, 0x97D2D988, 0x09B64C2B, 0x7EB17CBD,
        0xE7B82D07, 0x90BF1D91, 0x1DB71064, 0x6AB020F2, 0xF3B97148, 0x84BE41DE, 0x1ADAD47D,
        0x6DDDE4EB, 0xF4D4B551, 0x83D385C7, 0x136C9856, 0x646BA8C0, 0xFD62F97A, 0x8A65C9EC,
        0x14015C4F, 0x63066CD9, 0xFA0F3D63, 0x8D080DF5, 0x3B6E20C8, 0x4C69105E, 0xD56041E4,
        0xA2677172, 0x3C03E4D1, 0x4B04D447, 0xD20D85FD, 0xA50AB56B, 0x35B5A8FA, 0x42B2986C,
        0xDBBBC9D6, 0xACBCF940, 0x32D86CE3, 0x45DF5C75, 0xDCD60DCF, 0xABD13D59, 0x26D930AC,
        0x51DE003A, 0xC8D75180, 0xBFD06116, 0x21B4F4B5, 0x56B3C423, 0xCFBA9599, 0xB8BDA50F,
        0x2802B89E, 0x5F058808, 0xC60CD9B2, 0xB10BE924, 0x2F6F7C87, 0x58684C11, 0xC1611DAB,
        0xB6662D3D, 0x76DC4190, 0x01DB7106, 0x98D220BC, 0xEFD5102A, 0x71B18589, 0x06B6B51F,
        0x9FBFE4A5, 0xE8B8D433, 0x7807C9A2, 0x0F00F934, 0x9609A88E, 0xE10E9818, 0x7F6A0DBB,
        0x086D3D2D, 0x91646C97, 0xE6635C01, 0x6B6B51F4, 0x1C6C6162, 0x856530D8, 0xF262004E,
        0x6C0695ED, 0x1B01A57B, 0x8208F4C1, 0xF50FC457, 0x65B0D9C6, 0x12B7E950, 0x8BBEB8EA,
        0xFCB9887C, 0x62DD1DDF, 0x15DA2D49, 0x8CD37CF3, 0xFBD44C65, 0x4DB26158, 0x3AB551CE,
        0xA3BC0074, 0xD4BB30E2, 0x4ADFA541, 0x3DD895D7, 0xA4D1C46D, 0xD3D6F4FB, 0x4369E96A,
        0x346ED9FC, 0xAD678846, 0xDA60B8D0, 0x44042D73, 0x33031DE5, 0xAA0A4C5F, 0xDD0D7CC9,
        0x5005713C, 0x270241AA, 0xBE0B1010, 0xC90C2086, 0x5768B525, 0x206F85B3, 0xB966D409,
        0xCE61E49F, 0x5EDEF90E, 0x29D9C998, 0xB0D09822, 0xC7D7A8B4, 0x59B33D17, 0x2EB40D81,
        0xB7BD5C3B, 0xC0BA6CAD, 0xEDB88320, 0x9ABFB3B6, 0x03B6E20C, 0x74B1D29A, 0xEAD54739,
        0x9DD277AF, 0x04DB2615, 0x73DC1683, 0xE3630B12, 0x94643B84, 0x0D6D6A3E, 0x7A6A5AA8,
        0xE40ECF0B, 0x9309FF9D, 0x0A00AE27, 0x7D079EB1, 0xF00F9344, 0x8708A3D2, 0x1E01F268,
        0x6906C2FE, 0xF762575D, 0x806567CB, 0x196C3671, 0x6E6B06E7, 0xFED41B76, 0x89D32BE0,
        0x10DA7A5A, 0x67DD4ACC, 0xF9B9DF6F, 0x8EBEEFF9, 0x17B7BE43, 0x60B08ED5, 0xD6D6A3E8,
        0xA1D1937E, 0x38D8C2C4, 0x4FDFF252, 0xD1BB67F1, 0xA6BC5767, 0x3FB506DD, 0x48B2364B,
        0xD80D2BDA, 0xAF0A1B4C, 0x36034AF6, 0x41047A60, 0xDF60EFC3, 0xA867DF55, 0x316E8EEF,
        0x4669BE79, 0xCB61B38C, 0xBC66831A, 0x256FD2A0, 0x5268E236, 0xCC0C7795, 0xBB0B4703,
        0x220216B9, 0x5505262F, 0xC5BA3BBE, 0xB2BD0B28, 0x2BB45A92, 0x5CB36A04, 0xC2D7FFA7,
        0xB5D0CF31, 0x2CD99E8B, 0x5BDEAE1D, 0x9B64C2B0, 0xEC63F226, 0x756AA39C, 0x026D930A,
        0x9C0906A9, 0xEB0E363F, 0x72076785, 0x05005713, 0x95BF4A82, 0xE2B87A14, 0x7BB12BAE,
        0x0CB61B38, 0x92D28E9B, 0xE5D5BE0D, 0x7CDCEFB7, 0x0BDBDF21, 0x86D3D2D4, 0xF1D4E242,
        0x68DDB3F8, 0x1FDA836E, 0x81BE16CD, 0xF6B9265B, 0x6FB077E1, 0x18B74777, 0x88085AE6,
        0xFF0F6A70, 0x66063BCA, 0x11010B5C, 0x8F659EFF, 0xF862AE69, 0x616BFFD3, 0x166CCF45,
        0xA00AE278, 0xD70DD2EE, 0x4E048354, 0x3903B3C2, 0xA7672661, 0xD06016F7, 0x4969474D,
        0x3E6E77DB, 0xAED16A4A, 0xD9D65ADC, 0x40DF0B66, 0x37D83BF0, 0xA9BCAE53, 0xDEBB9EC5,
        0x47B2CF7F, 0x30B5FFE9, 0xBDBDF21C, 0xCABAC28A, 0x53B39330, 0x24B4A3A6, 0xBAD03605,
        0xCDD70693, 0x54DE5729, 0x23D967BF, 0xB3667A2E, 0xC4614AB8, 0x5D681B02, 0x2A6F2B94,
        0xB40BBE37, 0xC30C8EA1, 0x5A05DF1B, 0x2D02EF8D,
    ];

    for &b in pb.iter() {
        sig = (sig >> 8) ^ RGCRC[((sig & 0xff) ^ (b as u32)) as usize];
    }
    sig
}

#[test]
fn test_hash_sig() {
    static CASES: &[(u32, u32, &[u8])] = &[
        // expected_hash, input_sig, input_bytes
        (0x00000000, 0x00000000, &[]),
        (0x01234567, 0x01234567, &[]),
        (0x57eccb91, 0x00000000, b"hello, world!"),
        (0x29b1c6ec, 0xabababab, b"hello, world!"),
        (0x2b4468c3, 0x00000000, b"hello!"),
        (0x102f0bec, 0xabababab, b"hello!"),
    ];

    for &(expected_hash, input_sig, input_bytes) in CASES.iter() {
        let actual_hash = hash_sig(input_bytes, input_sig);
        assert_eq!(
            actual_hash,
            expected_hash,
            "actual: 0x{actual_hash:08x}, expected: 0x{expected_hash:08x}, input_sig: 0x{input_sig:08x}, bytes: {:?}",
            input_bytes.hex_dump()
        );
    }
}

/// Computes a CRC-32 with an initializer value, then computes the modulus of it.
pub fn hash_sig_mod(pb: &[u8], sig: u32, modulus: u32) -> u32 {
    let h = hash_sig(pb, sig);
    h % modulus
}
