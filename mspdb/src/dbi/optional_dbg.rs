//! Decodes the Optional Debug Header Substream.
//!
//! This substream contains an array of stream indexes. The order of the array is significant;
//! each has a specific purpose. They are enumerated by the [`OptionalDebugHeaderStream`] type.
//!
//! # References
//! * <https://llvm.org/docs/PDB/DbiStream.html#id10>
//! * [`DBGTYPE` in `pdb.h`](https://github.com/microsoft/microsoft-pdb/blob/805655a28bd8198004be2ac27e6e0290121a5e89/langapi/include/pdb.h#L438)

use super::*;

/// Provides access to the Optional Debug Header.
pub struct OptionalDebugHeader<'a> {
    /// Raw access to the stream indexes
    pub stream_indexes: &'a [StreamIndexU16],
}

impl<'a> OptionalDebugHeader<'a> {
    /// Parses the Optional Debug Header Substream.
    pub fn parse(bytes: &'a [u8]) -> anyhow::Result<Self> {
        let Ok(stream_indexes) = <[StreamIndexU16]>::ref_from_bytes(bytes) else {
            bail!("The OptionalDebugHeader has an invalid size. The size is required to be a multiple of 2. Size: {}",
                bytes.len());
        };

        Ok(Self { stream_indexes })
    }

    /// Gets a stream index, given an index into the Optional Debug Header.
    pub fn stream_by_index(&self, i: usize) -> Option<u32> {
        self.stream_indexes.get(i)?.get()
    }

    /// Gets a stream index, given an identifier for a stream within the Optional Debug Header.
    pub fn stream(&self, s: OptionalDebugHeaderStream) -> Option<u32> {
        self.stream_by_index(s as usize)
    }

    /// The number of stream indexes in the Optional Debug Header Substream.
    pub fn num_streams(&self) -> usize {
        self.stream_indexes.len()
    }

    /// Iterates the streams within the Optional Debug Header. The iterated values are
    /// `(i, stream)` where `i` is an index into the Optional Debug Header.
    /// `OptionalDebugHeaderStream::try_from(i)`.
    pub fn iter_streams(&self) -> IterStreams<'_> {
        IterStreams {
            stream_indexes: self.stream_indexes,
            next: 0,
        }
    }
}

/// Iterates streams
pub struct IterStreams<'a> {
    stream_indexes: &'a [StreamIndexU16],
    next: usize,
}

impl<'a> Iterator for IterStreams<'a> {
    type Item = (usize, u32);

    fn next(&mut self) -> Option<Self::Item> {
        while self.next < self.stream_indexes.len() {
            let i = self.next;
            let stream_index_or_nil = self.stream_indexes[i].get();
            self.next += 1;

            if let Some(stream_index) = stream_index_or_nil {
                return Some((i, stream_index));
            }
        }
        None
    }
}

macro_rules! optional_debug_header_streams {
    (
        $(
            $( #[$a:meta] )*
            $index:literal, $name:ident, $description:expr;
        )*
    ) => {
        /// Identifies the stream indexes stored in the Optional Debug Header.
        #[derive(Copy, Clone, Eq, PartialEq, Debug)]
        #[repr(u8)]
        #[allow(non_camel_case_types)]
        #[allow(missing_docs)]
        pub enum OptionalDebugHeaderStream {
            $(
                $( #[$a] )*
                $name = $index,
            )*
        }

        /// The short name (identifier) for each of the names in `OptionalDebugHeaderStream`.
        pub static OPTIONAL_DEBUG_HEADER_STREAM_NAME: [&str; 11] = [
            $(
                stringify!($name),
            )*
        ];

        /// The for each of the names in `OptionalDebugHeaderStream`.
        pub static OPTIONAL_DEBUG_HEADER_STREAM_DESCRIPTION: [&str; 11] = [
            $(
                $description,
            )*
        ];

        impl TryFrom<usize> for OptionalDebugHeaderStream {
            type Error = ();

            fn try_from(i: usize) -> std::result::Result<Self, Self::Error> {
                match i {
                    $( $index => Ok(Self::$name), )*
                    _ => Err(()),
                }
            }
        }
    }
}

optional_debug_header_streams! {
    /// Stream contains an array of `FPO_DATA` structures. This contains the relocated contents of
    /// any `.debug$F` section from any of the linker inputs.
    0, fpo_data, "";
    /// Stream contains a debug data directory of type `IMAGE_DEBUG_TYPE_EXCEPTION`.
    1, exception_data, "";
    /// Stream contains a debug data directory of type `IMAGE_DEBUG_TYPE_FIXUP`.
    2, fixup_data, "";
    /// Stream contains a debug data directory of type `IMAGE_DEBUG_TYPE_OMAP_TO_SRC`.
    /// This is used for mapping addresses from instrumented code to uninstrumented code.
    3, omap_to_src_data, "";
    /// Stream contains a debug data directory of type `IMAGE_DEBUG_TYPE_OMAP_FROM_SRC`.
    /// This is used for mapping addresses from uninstrumented code to instrumented code.
    4, omap_from_src_data, "";
    /// A dump of all section headers from the original executable.
    5, section_header_data, "";
    6, token_to_record_id_map, "";
    /// Exception handler data
    7, xdata, "";
    /// Procedure data
    8, pdata, "";
    9, new_fpo_data, "";
    10, original_section_header_data, "";
}
