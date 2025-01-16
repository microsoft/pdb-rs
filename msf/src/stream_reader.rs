use super::*;

/// Allows reading a stream using the [`Read`], [`Seek`], and [`ReadAt`] traits.
pub struct StreamReader<'a, F> {
    /// Size in bytes of the stream. This value _is never_ equal to [`NIL_STREAM_SIZE`].
    stream_size: u32,
    is_nil: bool,
    /// Page size of the MSF file.
    page_size: PageSize,
    /// Maps page indices within the stream to page indices within the MSF file.
    page_map: &'a [u32],
    /// Provides access to the MSF file contents.
    file: &'a F,
    /// The seek position of the stream reader.
    pos: u64,
}

impl<'a, F: ReadAt> StreamReader<'a, F> {
    pub(crate) fn new(pdb: &'a Msf<F>, stream_size: u32, page_map: &'a [u32], pos: u64) -> Self {
        Self {
            stream_size: if stream_size == NIL_STREAM_SIZE {
                0
            } else {
                stream_size
            },
            is_nil: stream_size == NIL_STREAM_SIZE,
            page_size: pdb.pages.page_size,
            page_map,
            file: &pdb.file,
            pos,
        }
    }

    /// Size in bytes of the stream.
    ///
    /// This will be zero for nil streams.
    pub fn len(&self) -> u32 {
        self.stream_size
    }

    /// Tests whether this stream is empty (zero-length)
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if this is a nil stream.
    pub fn is_nil(&self) -> bool {
        self.is_nil
    }
}

impl<'a, F: ReadAt> Seek for StreamReader<'a, F> {
    fn seek(&mut self, from: SeekFrom) -> std::io::Result<u64> {
        let new_pos: i64 = match from {
            SeekFrom::Start(offset) => offset as i64,
            SeekFrom::End(signed_offset) => signed_offset + self.stream_size as i64,
            SeekFrom::Current(signed_offset) => self.pos as i64 + signed_offset,
        };

        if new_pos < 0 {
            return Err(std::io::ErrorKind::InvalidInput.into());
        }
        self.pos = new_pos as u64;
        Ok(self.pos)
    }
}

impl<'a, F: ReadAt> Read for StreamReader<'a, F> {
    fn read(&mut self, dst: &mut [u8]) -> std::io::Result<usize> {
        let (n, new_pos) = super::read::read_stream_core(
            self.file,
            self.page_size,
            self.stream_size,
            self.page_map,
            self.pos,
            dst,
        )?;

        self.pos = new_pos;
        Ok(n)
    }
}

impl<'a, F: ReadAt> ReadAt for StreamReader<'a, F> {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        let (n, _new_pos) = super::read::read_stream_core(
            self.file,
            self.page_size,
            self.stream_size,
            self.page_map,
            offset,
            buf,
        )?;
        if n != buf.len() {
            return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
        }
        Ok(())
    }

    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        let (n, _new_pos) = super::read::read_stream_core(
            self.file,
            self.page_size,
            self.stream_size,
            self.page_map,
            offset,
            buf,
        )?;
        Ok(n)
    }
}
