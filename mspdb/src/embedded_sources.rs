use super::*;
use anyhow::Result;
use std::io::Write;

impl<F: ReadAt> Pdb<F> {
    /// Embeds the contents of a source file into the PDB.
    pub fn add_embedded_source(&mut self, file_path: &str, file_contents: &[u8]) -> Result<bool>
    where
        F: WriteAt,
    {
        let stream_name = format!("/src/{}", file_path);
        self.add_or_replace_named_stream(&stream_name, file_contents)
    }

    /// Sets the contents of a named stream to the given value.
    ///
    /// If there is already a named stream with the given name, then the stream's contents
    /// are replaced with `stream_contents`.  First, though, this function reads the contents of
    /// the existing stream and compares them to `stream_contents`. If they are identical, then
    /// the stream is not modified and this function will return `Ok(true)`.  If the contents are
    /// not identical, then this function returns `Ok(true)`.
    ///
    /// If there is not already a named stream with given name, then a new stream is created
    /// and an entry is added to the Named Streams Map. In this case, the function returns
    /// `Ok(false)`.
    pub fn add_or_replace_named_stream(
        &mut self,
        stream_name: &str,
        stream_contents: &[u8],
    ) -> Result<bool>
    where
        F: WriteAt,
    {
        if let Some(existing_stream) = self.named_streams().get(stream_name) {
            // No need to update the named stream directory.

            // Are the stream contents identical?
            let existing_len = self.stream_len(existing_stream);
            if existing_len == stream_contents.len() as u64 {
                let existing_contents = self.read_stream_to_vec(existing_stream)?;
                if existing_contents == stream_contents {
                    return Ok(false);
                }
            }

            let mut w = self.msf_mut_err()?.write_stream(existing_stream)?;
            w.set_len(0)?;
            w.write_all(stream_contents)?;
            Ok(true)
        } else {
            let (new_stream, mut w) = self.msf_mut_err()?.new_stream()?;
            w.write_all(stream_contents)?;
            self.named_streams_mut().insert(stream_name, new_stream);
            Ok(true)
        }
    }
}
