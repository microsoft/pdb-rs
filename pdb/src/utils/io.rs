#![allow(missing_docs)]

use std::io::{Read, Seek, SeekFrom, Write};
use sync_file::ReadAt;
use zerocopy::{FromBytes, FromZeros, IntoBytes};

pub fn read_struct_at<T: FromBytes + IntoBytes, R: ReadAt>(
    r: &R,
    offset: u64,
) -> std::io::Result<T> {
    let mut value = T::new_zeroed();
    r.read_exact_at(value.as_mut_bytes(), offset)?;
    Ok(value)
}

pub fn read_struct<T: FromBytes + IntoBytes, R: Read>(r: &mut R) -> std::io::Result<T> {
    let mut value: T = T::new_zeroed();
    let value_bytes = value.as_mut_bytes();
    r.read_exact(value_bytes)?;
    Ok(value)
}

pub fn read_boxed_slice<T: FromBytes + IntoBytes, R: Read>(
    r: &mut R,
    n: usize,
) -> std::io::Result<Box<[T]>> {
    let mut value = <[T]>::new_box_zeroed_with_elems(n).unwrap();
    r.read_exact(value.as_mut_bytes())?;
    Ok(value)
}

pub fn read_boxed_slice_at<T: FromBytes + IntoBytes, R: ReadAt>(
    r: &mut R,
    offset: u64,
    n: usize,
) -> std::io::Result<Box<[T]>> {
    let mut value = <[T]>::new_box_zeroed_with_elems(n).unwrap();
    r.read_exact_at(value.as_mut_bytes(), offset)?;
    Ok(value)
}

pub fn write_at<W: Write + Seek>(w: &mut W, pos: u64, data: &[u8]) -> std::io::Result<()> {
    w.seek(SeekFrom::Start(pos))?;
    w.write_all(data)
}
