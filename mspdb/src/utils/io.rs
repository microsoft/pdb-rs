#![allow(missing_docs)]

use std::io::{Read, Seek, SeekFrom, Write};
use sync_file::ReadAt;
use zerocopy::{AsBytes, FromBytes};

pub fn read_struct_at<T: FromBytes + AsBytes, R: ReadAt>(r: &R, offset: u64) -> std::io::Result<T> {
    let mut value = T::new_zeroed();
    r.read_exact_at(value.as_bytes_mut(), offset)?;
    Ok(value)
}

pub fn read_struct<T: FromBytes + AsBytes, R: Read>(r: &mut R) -> std::io::Result<T> {
    let mut value: T = T::new_zeroed();
    let value_bytes = value.as_bytes_mut();
    r.read_exact(value_bytes)?;
    Ok(value)
}

pub fn read_boxed_slice<T: FromBytes + AsBytes, R: Read>(
    r: &mut R,
    n: usize,
) -> std::io::Result<Box<[T]>> {
    let mut value: Box<[T]> = T::new_box_slice_zeroed(n);
    r.read_exact(value.as_bytes_mut())?;
    Ok(value)
}

pub fn read_boxed_slice_at<T: FromBytes + AsBytes, R: ReadAt>(
    r: &mut R,
    offset: u64,
    n: usize,
) -> std::io::Result<Box<[T]>> {
    let mut value: Box<[T]> = T::new_box_slice_zeroed(n);
    r.read_exact_at(value.as_bytes_mut(), offset)?;
    Ok(value)
}

pub fn write_at<W: Write + Seek>(w: &mut W, pos: u64, data: &[u8]) -> std::io::Result<()> {
    w.seek(SeekFrom::Start(pos))?;
    w.write_all(data)
}
