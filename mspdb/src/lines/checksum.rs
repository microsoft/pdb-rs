//! Code for the `FILE_CHECKSUMS` subsection.

use super::*;

/// The hash algorithm used for the checksum.
#[derive(
    Copy,
    Clone,
    Eq,
    PartialEq,
    Ord,
    PartialOrd,
    IntoBytes,
    FromBytes,
    Unaligned,
    KnownLayout,
    Immutable,
)]
#[repr(transparent)]
pub struct ChecksumKind(pub u8);

impl ChecksumKind {
    /// No checksum at all
    pub const NONE: ChecksumKind = ChecksumKind(0);
    /// MD-5 checksum. See `/ZH:MD5` for MSVC.
    pub const MD5: ChecksumKind = ChecksumKind(1);
    /// SHA-1 checksum. See `/ZH:SHA1` for MSVC
    pub const SHA_1: ChecksumKind = ChecksumKind(2);
    /// SHA-256 checksum.  See `/ZH:SHA_256` for MSVC.
    pub const SHA_256: ChecksumKind = ChecksumKind(3);
}

impl std::fmt::Debug for ChecksumKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        static NAMES: [&str; 4] = ["NONE", "MD5", "SHA_1", "SHA_256"];

        if let Some(name) = NAMES.get(self.0 as usize) {
            f.write_str(name)
        } else {
            write!(f, "??({})", self.0)
        }
    }
}

#[test]
fn checksum_kind_debug() {
    assert_eq!(format!("{:?}", ChecksumKind::SHA_256), "SHA_256");
    assert_eq!(format!("{:?}", ChecksumKind(42)), "??(42)");
}

/// The File Checksums Subection
///
/// The file checksums subsection contains records for the source files referenced by Line Data.
pub struct FileChecksumsSubsection<'a> {
    #[allow(missing_docs)]
    pub bytes: &'a [u8],
}

impl<'a> FileChecksumsSubsection<'a> {
    #[allow(missing_docs)]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// Iterates the `FileChecksum` records within this subsection.
    pub fn iter(&self) -> FileChecksumIter<'a> {
        FileChecksumIter { bytes: self.bytes }
    }

    /// Given a file index, which is a byte offset into the `FileChecksums` section, gets a
    /// `FileChecksum` value.
    pub fn get_file(&self, file_index: u32) -> anyhow::Result<FileChecksum<'a>> {
        if let Some(b) = self.bytes.get(file_index as usize..) {
            if let Some(c) = FileChecksumIter::new(b).next() {
                Ok(c)
            } else {
                bail!("failed to decode FileChecksum record");
            }
        } else {
            bail!("file index is out of range of file checksums subsection");
        }
    }
}

/// Like `FileChecksums`, but with mutable access
pub struct FileChecksumsSubsectionMut<'a> {
    #[allow(missing_docs)]
    pub bytes: &'a mut [u8],
}

impl<'a> HasRestLen for FileChecksumsSubsectionMut<'a> {
    fn rest_len(&self) -> usize {
        self.bytes.len()
    }
}

impl<'a> FileChecksumsSubsectionMut<'a> {
    #[allow(missing_docs)]
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes }
    }

    /// Iterates the `FileChecksumMut` records within this subsection.
    pub fn iter_mut(&mut self) -> FileChecksumMutIter<'_> {
        FileChecksumMutIter { bytes: self.bytes }
    }

    /// Given a file index, which is a byte offset into the `FileChecksums` section, gets a
    /// `FileChecksumMut` value.
    pub fn get_file_mut(&mut self, file_index: u32) -> anyhow::Result<FileChecksumMut<'_>> {
        if let Some(b) = self.bytes.get_mut(file_index as usize..) {
            if let Some(c) = FileChecksumMutIter::new(b).next() {
                Ok(c)
            } else {
                bail!("failed to decode FileChecksum record");
            }
        } else {
            bail!("file index is out of range of file checksums subsection");
        }
    }
}

/// Points to a single file checksum record.
pub struct FileChecksum<'a> {
    /// The fixed-size header.
    pub header: &'a FileChecksumHeader,
    /// The checksum bytes.
    pub checksum_data: &'a [u8],
}

/// Points to a single file checksum record, with mutable access.
pub struct FileChecksumMut<'a> {
    /// The fixed-size header.
    pub header: &'a mut FileChecksumHeader,
    /// The checksum bytes.
    pub checksum_data: &'a mut [u8],
}

impl<'a> FileChecksum<'a> {
    /// Gets the `NameIndex` of the file name for this record. To dereference the `NameIndex value,
    /// use [`crate::names::NamesStream`].
    pub fn name(&self) -> NameIndex {
        NameIndex(self.header.name.get())
    }
}

/// The header at the start of a file checksum record.
///
/// Each file checksum record specifies the name of the file (using an offset into the Names Stream),
/// the kind of checksum (none, SHA1, SHA256, MD5, etc.), the size of the checksum, and the
/// checksum bytes.
///
/// The checksum record is variable-length.
#[derive(IntoBytes, FromBytes, KnownLayout, Immutable, Unaligned, Clone, Debug)]
#[repr(C)]
pub struct FileChecksumHeader {
    /// Offset into the global string table (the `/names` stream) of the PDB.
    pub name: U32<LE>,

    /// The size in bytes of the checksum. The checksum bytes immediately follow the `FileChecksumHeader`.
    pub checksum_size: u8,

    /// The hash algorithm used for the checksum.
    pub checksum_kind: ChecksumKind,
}

/// Iterates FileChecksum values from a byte stream.
pub struct FileChecksumIter<'a> {
    /// The unparsed data
    pub bytes: &'a [u8],
}

impl<'a> HasRestLen for FileChecksumIter<'a> {
    fn rest_len(&self) -> usize {
        self.bytes.len()
    }
}

/// Iterator state. Iterates `FileChecksumMut` values.
pub struct FileChecksumMutIter<'a> {
    /// The unparsed data
    pub bytes: &'a mut [u8],
}

impl<'a> HasRestLen for FileChecksumMutIter<'a> {
    fn rest_len(&self) -> usize {
        self.bytes.len()
    }
}

impl<'a> FileChecksumIter<'a> {
    /// Starts a new iterator
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }
}

impl<'a> Iterator for FileChecksumIter<'a> {
    type Item = FileChecksum<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.bytes);
        let len_before = p.len();
        let header: &FileChecksumHeader = p.get().ok()?;
        let checksum_data = p.bytes(header.checksum_size as usize).ok()?;

        // Align to 4-byte boundaries.
        let record_len = len_before - p.len();
        let _ = p.skip((4 - (record_len & 3)) & 3);

        self.bytes = p.into_rest();
        Some(FileChecksum {
            header,
            checksum_data,
        })
    }
}

impl<'a> FileChecksumMutIter<'a> {
    /// Starts a new iterator
    pub fn new(bytes: &'a mut [u8]) -> Self {
        Self { bytes }
    }
}

impl<'a> Iterator for FileChecksumMutIter<'a> {
    type Item = FileChecksumMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bytes.is_empty() {
            return None;
        }

        let mut p = ParserMut::new(take(&mut self.bytes));
        let len_before = p.len();
        let header: &mut FileChecksumHeader = p.get_mut().ok()?;
        let checksum_data = p.bytes_mut(header.checksum_size as usize).ok()?;

        // Align to 4-byte boundaries.
        let record_len = len_before - p.len();
        let _ = p.skip((4 - (record_len & 3)) & 3);

        self.bytes = p.into_rest();
        Some(FileChecksumMut {
            header,
            checksum_data,
        })
    }
}

/// Test iteration of records with byte ranges.
#[test]
fn iter_ranges() {
    const PAD: u8 = 0xaa;

    #[rustfmt::skip]
    let data = &[
        200, 0, 0, 0,               // name
        0,                          // checksum_size
        0,                          // no checksum
        // <-- offset = 6
        PAD, PAD,
        // <-- offset = 8

        42, 0, 0, 0,                // name
        16,                         // checksum_size
        1,                          // MD5
        0xc0, 0xc1, 0xc2, 0xc3,     // checksum
        0xc4, 0xc5, 0xc6, 0xc7,
        0xc8, 0xc9, 0xca, 0xcb,
        0xcc, 0xcd, 0xce, 0xcf,
        // <-- offset = 30
        PAD, PAD,

        // <-- offset = 32
    ];

    let sums = FileChecksumsSubsection::new(data);
    let mut iter = sums.iter().with_ranges();

    let (sub0_range, _) = iter.next().unwrap();
    assert_eq!(sub0_range, 0..8);

    let (sub1_range, _) = iter.next().unwrap();
    assert_eq!(sub1_range, 8..32);

    assert!(iter.next().is_none());
}

/// Tests that FileChecksumMutIter allows us to modify checksum records.
#[test]
fn iter_mut() {
    const PAD: u8 = 0xaa;

    #[rustfmt::skip]
    let data = &[
        42, 0, 0, 0,                // name
        16,                         // checksum_size
        1,                          // MD5
        0xc0, 0xc1, 0xc2, 0xc3,     // checksum
        0xc4, 0xc5, 0xc6, 0xc7,
        0xc8, 0xc9, 0xca, 0xcb,
        0xcc, 0xcd, 0xce, 0xcf,
        // <-- offset = 22
        PAD, PAD,

        // <-- offset = 24
    ];

    let mut data_mut = data.to_vec();
    let mut sums = FileChecksumsSubsectionMut::new(&mut data_mut);
    let mut iter = sums.iter_mut();
    assert_eq!(iter.rest_len(), 24); // initial amount of data in iterator

    let sum0 = iter.next().unwrap();
    assert_eq!(iter.rest_len(), 0); // initial amount of data in iterator
    assert_eq!(sum0.header.name.get(), 42);
    assert_eq!(
        sum0.checksum_data,
        &[
            0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
            0xce, 0xcf
        ]
    );

    sum0.header.name = U32::new(0xcafef00d);
    sum0.checksum_data[4] = 0xff;

    #[rustfmt::skip]
    let expected_new_data = &[
        0x0d, 0xf0, 0xfe, 0xca,     // name (modified)
        16,                         // checksum_size
        1,                          // MD5
        0xc0, 0xc1, 0xc2, 0xc3,     // checksum
        0xff, 0xc5, 0xc6, 0xc7,     // <-- modified
        0xc8, 0xc9, 0xca, 0xcb,
        0xcc, 0xcd, 0xce, 0xcf,
        // <-- offset = 22
        PAD, PAD,

        // <-- offset = 24
    ];

    assert_eq!(data_mut.as_slice(), expected_new_data);
}

/// Tests FileChecksumIter and FileChecksumMutIter.
#[test]
fn basic_iter() {
    const PAD: u8 = 0xaa;

    #[rustfmt::skip]
    let data = &[
        42, 0, 0, 0,                // name
        16,                         // checksum_size
        1,                          // MD5
        0xc0, 0xc1, 0xc2, 0xc3,     // checksum
        0xc4, 0xc5, 0xc6, 0xc7,
        0xc8, 0xc9, 0xca, 0xcb,
        0xcc, 0xcd, 0xce, 0xcf,
        // <-- offset = 22
        PAD, PAD,

        // <-- offset = 24
        0, 1, 0, 0,                 // name
        16,                         // checksum_size
        1,                          // MD5
        0xd0, 0xd1, 0xd2, 0xd3,     // checksum
        0xd4, 0xd5, 0xd6, 0xd7,
        0xd8, 0xd9, 0xda, 0xdb,
        0xdc, 0xdd, 0xde, 0xdf,
        // <-- offset = 46
        PAD, PAD,
        // <-- offset = 48
    ];

    // Test FileChecksumIter (immutable iterator)
    {
        let mut iter = FileChecksumIter::new(data);
        assert_eq!(iter.rest_len(), 48); // initial amount of data in iterator
        let sum0 = iter.next().unwrap();
        assert_eq!(sum0.name(), NameIndex(42));
        assert_eq!(
            sum0.checksum_data,
            &[
                0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
                0xce, 0xcf
            ]
        );

        assert_eq!(iter.rest_len(), 24); // record 0 is 24 bytes (including padding)

        let sum1 = iter.next().unwrap();
        assert_eq!(sum1.name(), NameIndex(0x100));
        assert_eq!(
            sum1.checksum_data,
            &[
                0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd,
                0xde, 0xdf,
            ]
        );

        assert_eq!(iter.rest_len(), 0); // record 1 is 24 bytes (including padding), leaving nothing in buffer
        assert!(iter.next().is_none());
    }

    // Test FileChecksumMutIter (mutable iterator)
    // We duplicate this because we can't do generics over mutability.
    {
        let mut data_mut = data.to_vec();
        let mut iter = FileChecksumMutIter::new(&mut data_mut);
        assert_eq!(iter.rest_len(), 48); // initial amount of data in iterator
        let sum0 = iter.next().unwrap();
        assert_eq!(sum0.header.name.get(), 42);
        assert_eq!(
            sum0.checksum_data,
            &[
                0xc0, 0xc1, 0xc2, 0xc3, 0xc4, 0xc5, 0xc6, 0xc7, 0xc8, 0xc9, 0xca, 0xcb, 0xcc, 0xcd,
                0xce, 0xcf
            ]
        );

        assert_eq!(iter.rest_len(), 24); // record 0 is 24 bytes (including padding)

        let sum1 = iter.next().unwrap();
        assert_eq!(sum1.header.name.get(), 0x100);
        assert_eq!(
            sum1.checksum_data,
            &[
                0xd0, 0xd1, 0xd2, 0xd3, 0xd4, 0xd5, 0xd6, 0xd7, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd,
                0xde, 0xdf,
            ]
        );

        assert_eq!(iter.rest_len(), 0); // record 1 is 24 bytes (including padding), leaving nothing in buffer
        assert!(iter.next().is_none());
    }
}

#[test]
fn test_get_file() {
    const PAD: u8 = 0xaa;

    #[rustfmt::skip]
    let data = &[
        42,   0,    0, 0, 0, 0, PAD, PAD,      // record 0 at 0
        0xee, 0,    0, 0, 0, 0, PAD, PAD,      // record 1 at 8
        0,    0xcc, 0, 0, 0, 0, PAD, PAD,      // record 2 at 0x10
        // len = 0x18
    ];

    // Test immutable access
    {
        let sums = FileChecksumsSubsection::new(data);

        let sum0 = sums.get_file(0).unwrap();
        assert_eq!(sum0.name(), NameIndex(42));

        let sum1 = sums.get_file(8).unwrap();
        assert_eq!(sum1.name(), NameIndex(0xee));

        let sum2 = sums.get_file(0x10).unwrap();
        assert_eq!(sum2.name(), NameIndex(0xcc00));

        // Test bad index (way outside of data)
        assert!(sums.get_file(0x1000).is_err());

        // Test bad index (invalid header)
        assert!(sums.get_file(0x16).is_err());
    }

    // Test mutable access
    {
        let mut data_mut = data.to_vec();
        let mut sums = FileChecksumsSubsectionMut::new(&mut data_mut);

        let sum0 = sums.get_file_mut(0).unwrap();
        assert_eq!(sum0.header.name.get(), 42);

        let sum1 = sums.get_file_mut(8).unwrap();
        assert_eq!(sum1.header.name.get(), 0xee);

        let sum2 = sums.get_file_mut(0x10).unwrap();
        assert_eq!(sum2.header.name.get(), 0xcc00);

        // Modify one of the records
        sum2.header.name = U32::new(0xcafe);

        // Test bad index (way outside of data)
        assert!(sums.get_file_mut(0x1000).is_err());

        // Test bad index (invalid header)
        assert!(sums.get_file_mut(0x16).is_err());

        #[rustfmt::skip]
        let expected_data = &[
            42,   0,    0, 0, 0, 0, PAD, PAD,      // record 0 at 0
            0xee, 0,    0, 0, 0, 0, PAD, PAD,      // record 1 at 8
            0xfe, 0xca, 0, 0, 0, 0, PAD, PAD,      // record 2 at 0x10
        ];

        assert_eq!(data_mut.as_slice(), expected_data);
    }
}
