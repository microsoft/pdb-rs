use std::str::FromStr;

use bitvec::prelude::BitSlice;

#[allow(dead_code)] // useful
pub fn dump_bitvec<T, O, W: std::fmt::Write + ?Sized>(
    b: &BitSlice<T, O>,
    out: &mut W,
) -> std::fmt::Result
where
    T: bitvec::prelude::BitStore,
    O: bitvec::prelude::BitOrder,
{
    let mut prev = None;
    for i in b.iter_ones() {
        if let Some((start, end)) = &mut prev {
            if *end + 1 == i {
                *end = i;
                continue;
            }
            if *start != *end {
                write!(out, "{}-{} ", *start, *end)?;
            } else {
                write!(out, "{} ", *start)?;
            }
            prev = None;
        } else {
            prev = Some((i, i));
        }
    }

    if let Some((start, end)) = prev {
        if start != end {
            write!(out, "{start}-{end} ")?;
        } else {
            write!(out, "{start} ")?;
        }
    }

    Ok(())
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct HexU64(pub u64);

impl FromStr for HexU64 {
    type Err = <u64 as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let value: u64 = if let Some(suffix) = s.strip_prefix("0x") {
            u64::from_str_radix(suffix, 0x10)?
        } else if let Some(suffix) = s.strip_prefix("0X") {
            u64::from_str_radix(suffix, 0x10)?
        } else {
            u64::from_str(s)?
        };
        Ok(Self(value))
    }
}
