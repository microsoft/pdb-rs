#![allow(clippy::redundant_pattern_matching)]

use super::*;
use bstr::ByteSlice;
use std::borrow::Cow;
use zerocopy::FromZeroes;

#[test]
fn empty() {
    assert!(Parser::new(&[]).is_empty());
    assert!(!Parser::new(&[42]).is_empty());
}

#[test]
fn len() {
    assert_eq!(Parser::new(&[]).len(), 0);
    assert_eq!(Parser::new(&[42]).len(), 1);
}

#[test]
fn ints() {
    let bytes = &[
        0x12, 0x34, // u16
        0x56, 0x78, 0xaa, 0xee, // u32
        0x55, 0x33, // u16
    ];

    let mut p = Parser::new(bytes);
    assert_eq!(p.len(), 8);
    assert_eq!(p.u16().unwrap(), 0x3412);
    assert_eq!(p.len(), 6);
    assert_eq!(p.u32().unwrap(), 0xeeaa_7856);
    assert_eq!(p.len(), 2);
    assert_eq!(p.u8().unwrap(), 0x55);
    assert_eq!(p.len(), 1);
    assert_eq!(p.i8().unwrap(), 0x33);
    assert_eq!(p.len(), 0);
    assert!(p.is_empty());

    // Integers do not need to be aligned.  Read some misaligned stuff.
    let mut p = Parser::new(bytes);
    p.u8().unwrap();
    assert_eq!(p.u16().unwrap(), 0x5634); // at index 1
    assert_eq!(p.u32().unwrap(), 0x55_ee_aa_78); // at index 3

    let mut p = Parser::new(&[1, 2, 3, 4]);
    assert_eq!(p.u32().unwrap(), 0x04_03_02_01);
    assert!(p.is_empty());

    let mut p = Parser::new(&[1, 2, 3, 4]);
    assert_eq!(p.type_index().unwrap(), TypeIndex(0x04_03_02_01));
    assert!(p.is_empty());
}

#[test]
fn strz() {
    assert!(Parser::new(&[]).strz().is_err());
    assert!(Parser::new(&b"x").strz().is_err());

    assert_eq!(
        Parser::new(&[b'f', b'o', b'o', 0])
            .strz()
            .unwrap()
            .to_str()
            .unwrap(),
        "foo"
    );
}

#[test]
fn strz_bad_utf8() {
    // 0x80 is a continuation byte in UTF-8. It must be preceded by a leading byte.
    let buf: &[u8] = b"\x80 bad\0second string \xe2\x9c\x85\0";
    let mut p = Parser::new(buf);

    let bad_raw = p.strz().unwrap();
    assert!(bad_raw.to_str().is_err()); // is not valid UTF-8
    assert_eq!(bad_raw as &[u8], &[0x80, b' ', b'b', b'a', b'd']);
    let bad_lossy: Cow<str> = bad_raw.to_str_lossy();
    assert!(matches!(bad_lossy, Cow::Owned(_)));
    assert_eq!(bad_lossy, "\u{fffd} bad"); // U+FFFD is the replacement character

    let good = p.strz().unwrap();
    let good_str: &str = good.to_str().unwrap();
    assert_eq!(good_str, "second string ✅");

    assert!(p.is_empty());
}

#[test]
fn strt() {
    let buf = b"\x03abc123";
    let mut p = Parser::new(buf);
    let s = p.strt().unwrap();
    assert_eq!(s, "abc");
    assert_eq!(p.into_rest(), b"123");
}

#[test]
fn rest() {
    let mut p = Parser::new(&[1, 2, 3, 4, 5]);
    assert_eq!(p.u8().unwrap(), 1);

    assert_eq!(p.peek_rest(), &[2, 3, 4, 5]);

    let rest = p.take_rest();
    assert_eq!(rest, &[2, 3, 4, 5]);
    assert!(p.is_empty());
}

#[test]
fn into_rest() {
    let mut p = Parser::new(&[1, 2, 3, 4, 5]);
    assert_eq!(p.u8().unwrap(), 1);
    let rest = p.into_rest();
    assert_eq!(rest, &[2, 3, 4, 5]);
}

#[test]
fn needs() {
    let p = Parser::new(&[10, 20]);
    assert!(matches!(p.needs(0), Ok(_)));
    assert!(matches!(p.needs(2), Ok(_)));
    assert!(matches!(p.needs(3), Err(_)));
}

#[test]
fn skip() {
    let mut p = Parser::new(&[1, 2, 3, 4, 5]);
    p.skip(2).unwrap();
    assert_eq!(p.u16().unwrap(), 0x403);
}

#[derive(AsBytes, FromBytes, FromZeroes, Unaligned, PartialEq, Eq, Debug)]
#[repr(C)]
struct Bar {
    b: u8,
    a: u8,
    r: u8,
}

#[test]
fn get() {
    let buf = &[b'b', b'a', b'r', 4, 5];
    let mut p = Parser::new(buf);

    let bar: &Bar = p.get().unwrap();
    assert_eq!(bar.b, b'b');
    assert_eq!(bar.a, b'a');
    assert_eq!(bar.r, b'r');

    assert!(p.get::<Bar>().is_err());
}

#[test]
fn copy() {
    let buf = &[b'b', b'a', b'r', 4, 5];
    let mut p = Parser::new(buf);

    let bar: Bar = p.copy().unwrap();
    assert_eq!(bar.b, b'b');
    assert_eq!(bar.a, b'a');
    assert_eq!(bar.r, b'r');

    assert!(p.copy::<Bar>().is_err());
}

#[test]
fn slice() {
    let buf = b"ABC123()!zap";
    let mut p = Parser::new(buf);

    let bars: &[Bar] = p.slice(3).unwrap();
    assert_eq!(bars.len(), 3);
    assert_eq!(
        bars[0],
        Bar {
            b: b'A',
            a: b'B',
            r: b'C'
        }
    );
    assert_eq!(
        bars[1],
        Bar {
            b: b'1',
            a: b'2',
            r: b'3'
        }
    );
    assert_eq!(
        bars[2],
        Bar {
            b: b'(',
            a: b')',
            r: b'!'
        }
    );
    assert_eq!(p.into_rest(), b"zap");
}
