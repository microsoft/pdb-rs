//! Section 4, numeric leaves

use super::Leaf;
use crate::parser::{Parse, Parser, ParserError};
use bstr::BStr;
use dump_utils::HexStr;
use std::fmt::{Debug, Display};
use std::num::TryFromIntError;

/// A numeric constant defined within a CodeView type or symbol record.
///
/// # References
/// * "Numeric Leaves" section of PDB specification.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Number<'a> {
    bytes: &'a [u8],
}

impl<'a> Number<'a> {
    /// Gets the raw bytes of this `Number`.
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Gets the kind (representation) of this value.
    /// If this is an immediate value (integer in `0..=0x7fff`), gets the actual value.
    pub fn kind(&self) -> Leaf {
        let mut p = Parser::new(self.bytes);
        Leaf(p.u16().unwrap())
    }
}

impl<'a> Parse<'a> for Number<'a> {
    fn from_parser(p: &mut Parser<'a>) -> Result<Self, ParserError> {
        let start = p.peek_rest();

        let more_len = match Leaf(p.u16()?) {
            lf if lf.is_immediate_numeric() => 0,
            Leaf::LF_CHAR => 1,
            Leaf::LF_SHORT => 2,
            Leaf::LF_USHORT => 2,
            Leaf::LF_LONG => 4,
            Leaf::LF_ULONG => 4,
            Leaf::LF_REAL32 => 4,
            Leaf::LF_REAL64 => 8,
            Leaf::LF_REAL80 => 10,
            Leaf::LF_REAL128 => 16,
            Leaf::LF_QUADWORD => 8,
            Leaf::LF_UQUADWORD => 8,
            Leaf::LF_REAL48 => 6,
            Leaf::LF_COMPLEX32 => 8,
            Leaf::LF_COMPLEX64 => 16,
            Leaf::LF_COMPLEX80 => 20,
            Leaf::LF_COMPLEX128 => 32,
            Leaf::LF_VARSTRING => p.u16()? as usize,
            Leaf::LF_OCTWORD => 16,
            Leaf::LF_UOCTWORD => 16,
            Leaf::LF_DECIMAL => 16,
            Leaf::LF_DATE => 8,
            Leaf::LF_UTF8STRING => {
                p.skip_strz()?;
                0
            }
            Leaf::LF_REAL16 => 2,
            lf => {
                log::warn!("unrecognized numeric leaf {lf:?}");
                // We don't know how many bytes to consume, so we can't keep parsing.
                return Err(ParserError::new());
            }
        };

        p.skip(more_len)?;
        Ok(Self {
            bytes: &start[..start.len() - p.len()],
        })
    }
}

impl<'a> Number<'a> {}

macro_rules! try_from_number {
    (
        $t:ty
    ) => {
        impl<'a> TryFrom<Number<'a>> for $t {
            type Error = TryFromIntError;

            #[inline(never)]
            fn try_from(value: Number<'a>) -> Result<Self, Self::Error> {
                use map_parser_error_to_int_error as e;

                let mut p = Parser::new(value.bytes);
                Ok(match Leaf(e(p.u16())?) {
                    lf if lf.is_immediate_numeric() => Self::try_from(lf.0)?,
                    Leaf::LF_USHORT => Self::try_from(e(p.u16())?)?,
                    Leaf::LF_ULONG => Self::try_from(e(p.u32())?)?,
                    Leaf::LF_UQUADWORD => Self::try_from(e(p.u64())?)?,
                    Leaf::LF_CHAR => Self::try_from(e(p.i8())?)?,
                    Leaf::LF_SHORT => Self::try_from(e(p.i16())?)?,
                    Leaf::LF_LONG => Self::try_from(e(p.i32())?)?,
                    Leaf::LF_QUADWORD => Self::try_from(e(p.i64())?)?,
                    Leaf::LF_OCTWORD => Self::try_from(e(p.i128())?)?,
                    Leaf::LF_UOCTWORD => Self::try_from(e(p.u128())?)?,
                    _ => return Err(try_from_int_error()),
                })
            }
        }
    };
}

try_from_number!(i8);
try_from_number!(i16);
try_from_number!(i32);
try_from_number!(i64);
try_from_number!(i128);

try_from_number!(u8);
try_from_number!(u16);
try_from_number!(u32);
try_from_number!(u64);
try_from_number!(u128);

fn map_parser_error_to_int_error<T>(r: Result<T, ParserError>) -> Result<T, TryFromIntError> {
    match r {
        Ok(x) => Ok(x),
        Err(ParserError) => Err(try_from_int_error()),
    }
}

/// Error type for conversions from `Number` to `f32`
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TryFromFloatError;

impl From<ParserError> for TryFromFloatError {
    fn from(_: ParserError) -> Self {
        Self
    }
}

impl<'a> TryFrom<Number<'a>> for f32 {
    type Error = TryFromFloatError;

    fn try_from(value: Number<'a>) -> Result<Self, Self::Error> {
        let mut p = Parser::new(value.bytes);
        Ok(match Leaf(p.u16()?) {
            Leaf::LF_REAL32 => f32::from_le_bytes(p.array()?),
            _ => return Err(TryFromFloatError),
        })
    }
}

impl<'a> TryFrom<Number<'a>> for f64 {
    type Error = TryFromFloatError;

    fn try_from(value: Number<'a>) -> Result<Self, Self::Error> {
        let mut p = Parser::new(value.bytes);
        Ok(match Leaf(p.u16()?) {
            Leaf::LF_REAL32 => f32::from_le_bytes(p.array::<4>()?) as f64,
            Leaf::LF_REAL64 => f64::from_le_bytes(p.array::<8>()?),
            _ => return Err(TryFromFloatError),
        })
    }
}

/// Error type for conversions from `Number` to string
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct TryFromStrError;

impl From<ParserError> for TryFromStrError {
    fn from(_: ParserError) -> Self {
        Self
    }
}
impl<'a> TryFrom<Number<'a>> for &'a BStr {
    type Error = TryFromStrError;

    fn try_from(value: Number<'a>) -> Result<Self, Self::Error> {
        let mut p = Parser::new(value.bytes);
        Ok(match Leaf(p.u16()?) {
            Leaf::LF_UTF8STRING => p.strz()?,
            Leaf::LF_VARSTRING => {
                let len = p.u16()?;
                let bytes = p.bytes(len as usize)?;
                BStr::new(bytes)
            }
            _ => return Err(TryFromStrError),
        })
    }
}

impl<'a> Debug for Number<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl<'a> Display for Number<'a> {
    #[inline(never)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn e<T>(
            f: &mut std::fmt::Formatter<'_>,
            r: Result<T, ParserError>,
        ) -> Result<T, std::fmt::Error> {
            match r {
                Ok(x) => Ok(x),
                Err(ParserError) => {
                    f.write_str("??(parser error)")?;
                    Err(std::fmt::Error)
                }
            }
        }

        let mut p = Parser::new(self.bytes);

        match Leaf(p.u16().unwrap()) {
            lf if lf.is_immediate_numeric() => Display::fmt(&lf.0, f),
            Leaf::LF_CHAR => Display::fmt(&e(f, p.i8())?, f),
            Leaf::LF_SHORT => Display::fmt(&e(f, p.i16())?, f),
            Leaf::LF_USHORT => Display::fmt(&e(f, p.u16())?, f),
            Leaf::LF_LONG => Display::fmt(&e(f, p.i32())?, f),
            Leaf::LF_ULONG => Display::fmt(&e(f, p.u32())?, f),
            Leaf::LF_REAL32 => Display::fmt(&e(f, p.f32())?, f),
            Leaf::LF_REAL64 => Display::fmt(&e(f, p.f64())?, f),
            Leaf::LF_QUADWORD => Display::fmt(&e(f, p.i64())?, f),
            Leaf::LF_UQUADWORD => Display::fmt(&e(f, p.u64())?, f),
            Leaf::LF_VARSTRING => {
                // This uses a 2-byte length prefix, not 1-byte.
                let len = p.u16().unwrap();
                let s = BStr::new(p.bytes(len as usize).unwrap());
                <BStr as Display>::fmt(s, f)
            }
            Leaf::LF_OCTWORD => Display::fmt(&e(f, p.i128())?, f),
            Leaf::LF_UOCTWORD => Display::fmt(&e(f, p.u128())?, f),
            Leaf::LF_UTF8STRING => {
                let s = p.strz().unwrap();
                <BStr as Display>::fmt(s, f)
            }

            lf => {
                write!(f, "?? {lf:?} {:?}", HexStr::new(self.bytes))
            }
        }
    }
}

fn try_from_int_error() -> TryFromIntError {
    u32::try_from(-1i8).unwrap_err()
}

#[cfg(test)]
fn parse_number(bytes: &[u8]) -> Number {
    let mut p = Parser::new(bytes);
    let n = p.number().unwrap();
    assert!(p.is_empty());
    n
}

#[test]
fn number_error() {
    assert!(Number::parse(&[]).is_err()); // too short
    assert!(Number::parse(&[0]).is_err()); // also too short
    assert!(Number::parse(&[0xff, 0xff]).is_err()); // unrecognized kind
}

#[test]
fn number_immediate() {
    // Values below 0x8000 are literal uint16 constants.
    let n = parse_number(&[0xaa, 0x70]);
    assert_eq!(n.as_bytes(), &[0xaa, 0x70]);
    assert_eq!(u32::try_from(n).unwrap(), 0x70aa);
}

#[test]
fn number_char() {
    // LF_CHAR
    let n = parse_number(&[0x00, 0x80, (-33i8) as u8]);
    assert_eq!(i32::try_from(n).unwrap(), -33);

    assert!(f32::try_from(n).is_err());
    assert!(f64::try_from(n).is_err());
    assert!(<&BStr>::try_from(n).is_err());
}

#[test]
fn number_short() {
    // LF_SHORT
    let n = parse_number(&[0x01, 0x80, 0xaa, 0x55]);
    assert_eq!(i32::try_from(n).unwrap(), 0x55aa_i32);
    assert_eq!(u32::try_from(n).unwrap(), 0x55aa_u32);

    let n = parse_number(&[0x01, 0x80, 0x55, 0xaa]);
    assert_eq!(i32::try_from(n).unwrap(), -21931_i32);
    assert!(u32::try_from(n).is_err());

    assert!(f32::try_from(n).is_err());
    assert!(f64::try_from(n).is_err());
    assert!(<&BStr>::try_from(n).is_err());
}

#[test]
fn number_long() {
    // LF_LONG
    let n = parse_number(&[0x03, 0x80, 1, 2, 3, 4]);
    assert_eq!(u32::try_from(n).unwrap(), 0x04030201_u32);
    assert_eq!(i32::try_from(n).unwrap(), 0x04030201_i32);
    assert!(u16::try_from(n).is_err());
    assert!(i16::try_from(n).is_err());
    assert!(u8::try_from(n).is_err());
    assert!(i8::try_from(n).is_err());

    // unsigned cannot decode negative numbers
    let n = parse_number(&[0x03, 0x80, 0xfe, 0xff, 0xff, 0xff]);
    assert!(u8::try_from(n).is_err());
    assert!(u16::try_from(n).is_err());
    assert!(u32::try_from(n).is_err());
    assert!(u64::try_from(n).is_err());
    assert!(u128::try_from(n).is_err());
    assert_eq!(i8::try_from(n).unwrap(), -2);
    assert_eq!(i16::try_from(n).unwrap(), -2);
    assert_eq!(i32::try_from(n).unwrap(), -2);
    assert_eq!(i64::try_from(n).unwrap(), -2);
    assert_eq!(i128::try_from(n).unwrap(), -2);

    assert!(f32::try_from(n).is_err());
    assert!(f64::try_from(n).is_err());
    assert!(<&BStr>::try_from(n).is_err());
}

#[test]
fn number_real32() {
    use std::f32::consts::PI;

    let b: [u8; 4] = PI.to_le_bytes();
    assert_eq!(b, [0xdb, 0x0f, 0x49, 0x40]); // 0x400490fdb, pi in f32
    println!("f32 PI bytes: {:#x?}", b);

    // 8005 is LF_REAL32
    let n = parse_number(&[0x05, 0x80, 0xdb, 0x0f, 0x49, 0x40]);

    // LF_REAL32 is not convertible to any of the integer types
    assert!(u8::try_from(n).is_err());
    assert!(u16::try_from(n).is_err());
    assert!(u32::try_from(n).is_err());
    assert!(u64::try_from(n).is_err());
    assert!(u128::try_from(n).is_err());

    assert!(i8::try_from(n).is_err());
    assert!(i16::try_from(n).is_err());
    assert!(i32::try_from(n).is_err());
    assert!(i64::try_from(n).is_err());
    assert!(i128::try_from(n).is_err());

    // Floating-point exact equality can be weird.
    assert_eq!(f32::try_from(n).unwrap(), PI);

    // We convert to f64 but do not verify the value, because again, floating-point is weird.
    let _ = f64::try_from(n).unwrap();
}

#[test]
fn number_real64() {
    use std::f64::consts::PI;

    let b: [u8; 8] = PI.to_le_bytes();
    assert_eq!(b, [0x18, 0x2d, 0x44, 0x54, 0xfb, 0x21, 0x9, 0x40]);
    // assert_eq!(b, [0xdb, 0x0f, 0x49, 0x40]); // 0x400921fb54442d18, pi in f64
    println!("f64 PI bytes: {:#x?}", b);

    // 8006 is LF_REAL64
    let n = parse_number(&[0x06, 0x80, 0x18, 0x2d, 0x44, 0x54, 0xfb, 0x21, 0x9, 0x40]);

    // LF_REAL64 is not convertible to any of the integer types
    assert!(u8::try_from(n).is_err());
    assert!(u16::try_from(n).is_err());
    assert!(u32::try_from(n).is_err());
    assert!(u64::try_from(n).is_err());
    assert!(u128::try_from(n).is_err());

    assert!(i8::try_from(n).is_err());
    assert!(i16::try_from(n).is_err());
    assert!(i32::try_from(n).is_err());
    assert!(i64::try_from(n).is_err());
    assert!(i128::try_from(n).is_err());

    // Floating-point exact equality can be weird.
    assert_eq!(f64::try_from(n).unwrap(), PI);
}

#[test]
fn number_strz() {
    let n = parse_number(b"\x1b\x80Hello, world\0");
    assert_eq!(n.kind(), Leaf::LF_UTF8STRING);
    assert_eq!(<&BStr>::try_from(n).unwrap(), "Hello, world");

    let n = parse_number(&[0x00, 0x80, (-33i8) as u8]);
    assert!(<&BStr>::try_from(n).is_err());
}

#[test]
fn number_varstring() {
    let s = parse_number(b"\x10\x80\x0c\x00Hello, world");
    assert_eq!(s.kind(), Leaf::LF_VARSTRING);
    assert_eq!(<&BStr>::try_from(s).unwrap(), "Hello, world");
}

#[test]
fn number_unsupported_types() {
    // We can test decoding the prefix for these types, even if we can't currently display them
    // or convert them to something useful.
    let cases: &[(Leaf, usize)] = &[
        (Leaf::LF_REAL80, 10),
        (Leaf::LF_REAL128, 16),
        (Leaf::LF_REAL48, 6),
        (Leaf::LF_COMPLEX32, 8),
        (Leaf::LF_COMPLEX64, 16),
        (Leaf::LF_COMPLEX80, 20),
        (Leaf::LF_COMPLEX128, 32),
        (Leaf::LF_DECIMAL, 16),
        (Leaf::LF_DATE, 8),
        (Leaf::LF_REAL16, 2),
    ];

    for &(kind, num_zeroes) in cases.iter() {
        let mut input = vec![0; 2 + num_zeroes];
        input[0] = kind.0 as u8;
        input[1] = (kind.0 >> 8) as u8;
        let n = parse_number(&input);
        assert_eq!(kind, n.kind());
    }
}

#[test]
fn display() {
    let cases: &[(&[u8], &str)] = &[
        (&[0x01, 0x04], "immediate 1025"),
        (&[0x00, 0x80, 0xff], "LF_CHAR -1"),
        (&[0x01, 0x80, 0xfe, 0xff], "LF_SHORT -2"),
        (&[0x02, 0x80, 0xfd, 0xff], "LF_USHORT 65533"),
        (&[0x03, 0x80, 0xfc, 0xff, 0xff, 0xff], "LF_LONG -4"),
        (&[0x04, 0x80, 0x00, 0x00, 0x02, 0x00], "LF_ULONG 131072"),
        (&[0x05, 0x80, 0xdb, 0x0f, 0x49, 0x40], "LF_REAL32 3.1415927"),
        (
            &[0x06, 0x80, 0x18, 0x2d, 0x44, 0x54, 0xfb, 0x21, 0x9, 0x40],
            "LF_REAL64 3.141592653589793",
        ),
        (
            &[0x09, 0x80, 0xfb, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            "LF_QUADWORD -5",
        ),
        (
            &[0x0a, 0x80, 0x00, 0xe4, 0x0b, 0x54, 0x02, 0x00, 0x00, 0x00],
            "LF_UQUADWORD 10000000000",
        ),
        (
            &[
                0x17, 0x80, 0xfa, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                0xff, 0xff, 0xff, 0xff,
            ],
            "LF_OCTWORD -6",
        ),
        (
            &[
                0x18, 0x80, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00,
            ],
            "LF_UOCTWORD 1",
        ),
    ];

    for &(input, expected_output) in cases.iter() {
        let mut p = Parser::new(input);
        let leaf = Leaf(p.u16().unwrap());

        let n = parse_number(input);

        let actual_output = if leaf.is_immediate_numeric() {
            format!("immediate {n}")
        } else {
            format!("{leaf:?} {n}")
        };

        assert_eq!(actual_output, expected_output, "bytes: {:#x?}", input);

        // Cover Debug::fmt. It just trivially defers to Display.
        let _ = format!("{:?}", n);
    }
}

#[test]
fn display_bogus() {
    // Because the byte slice within Number is private and Number::parse() does not construct
    // a Number for kinds it does not recognize, it is impossible (outside of this module)
    // to construct a Number over an invalid, non-immediate Leaf value.  But the Display code
    // has to have a case for that, so we construct a bogus Number just so we can display it.
    let bogus_num = Number {
        bytes: &[0xff, 0xff, 0xaa, 0xaa, 0xaa],
    };
    println!("bogus_num = {bogus_num}");
}
