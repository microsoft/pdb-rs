/*
use sha2::{Digest, Sha256};

#[derive(Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Sha256Hash(pub [u8; 32]);

impl Sha256Hash {
    pub fn digest(bytes: &[u8]) -> Sha256Hash {
        Self(Sha256::digest(bytes).into())
    }
}

impl std::fmt::Display for Sha256Hash {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        const HEX_DIGITS: [u8; 16] = [
            b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd',
            b'e', b'f',
        ];

        let mut chars: [u8; 64] = [0; 64];
        for i in 0..32 {
            let b = self.0[i];
            chars[i * 2] = HEX_DIGITS[(b >> 4) as usize];
            chars[i * 2 + 1] = HEX_DIGITS[(b & 0xf) as usize];
        }

        fmt.write_str(std::str::from_utf8(&chars).unwrap())
    }
}
*/
