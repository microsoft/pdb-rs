//! Iteration logic for subsections

use super::*;

/// Iterator state for subsections
pub struct SubsectionIter<'a> {
    rest: &'a [u8],
}

impl<'a> SubsectionIter<'a> {
    /// Start iteration
    pub fn new(rest: &'a [u8]) -> Self {
        Self { rest }
    }

    /// The remaining unparsed data.
    pub fn rest(&self) -> &'a [u8] {
        self.rest
    }
}

impl<'a> HasRestLen for SubsectionIter<'a> {
    fn rest_len(&self) -> usize {
        self.rest.len()
    }
}

/// Iterator state for subsections with mutable access
pub struct SubsectionIterMut<'a> {
    rest: &'a mut [u8],
}

impl<'a> SubsectionIterMut<'a> {
    /// Begins iteration
    pub fn new(rest: &'a mut [u8]) -> Self {
        Self { rest }
    }

    /// The remaining unparsed data.
    pub fn rest(&self) -> &[u8] {
        self.rest
    }
}

impl<'a> HasRestLen for SubsectionIterMut<'a> {
    fn rest_len(&self) -> usize {
        self.rest.len()
    }
}

/// A reference to one subsection
pub struct Subsection<'a> {
    /// The kind of data in this subsection.
    pub kind: SubsectionKind,
    /// The contents of the subsection.
    pub data: &'a [u8],
}

/// A reference to one subsection, with mutable access
pub struct SubsectionMut<'a> {
    /// The kind of data in this subsection.
    pub kind: SubsectionKind,
    /// The contents of the subsection.
    pub data: &'a mut [u8],
}

/// The header of a subsection.
#[derive(AsBytes, FromBytes, FromZeroes, Unaligned)]
#[repr(C)]
pub struct SubsectionHeader {
    /// The kind of data in this subsection.
    pub kind: U32<LE>,
    /// The size of the subsection, in bytes. This value does not count the size of the `kind` field.
    pub size: U32<LE>,
}

impl<'a> Iterator for SubsectionIter<'a> {
    type Item = Subsection<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let mut p = Parser::new(self.rest);
        let header: &SubsectionHeader = if let Ok(h) = p.get::<SubsectionHeader>() {
            h
        } else {
            warn!(
                "Failed to decode subsection data (incomplete header)!  rest_len = {}",
                self.rest.len()
            );
            return None;
        };
        let size = header.size.get() as usize;

        let data = if let Ok(d) = p.bytes(size) {
            d
        } else {
            warn!(
                "Failed to decode subsection data (incomplete payload)!  rest_len = {}",
                self.rest.len()
            );
            return None;
        };

        // If 'size' is not 4-byte aligned, then skip the alignment bytes.
        let alignment_len = (4 - (size & 3)) & 3;
        let _ = p.skip(alignment_len);

        self.rest = p.into_rest();

        Some(Subsection {
            kind: SubsectionKind(header.kind.get()),
            data,
        })
    }
}

impl<'a> Iterator for SubsectionIterMut<'a> {
    type Item = SubsectionMut<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let mut p = ParserMut::new(core::mem::take(&mut self.rest));
        let header: &SubsectionHeader = p.get::<SubsectionHeader>().ok()?;
        let size = header.size.get() as usize;
        let data = p.bytes_mut(size).ok()?;

        let alignment_len = (4 - (size & 3)) & 3;
        let _ = p.skip(alignment_len);

        self.rest = p.into_rest();

        Some(SubsectionMut {
            kind: SubsectionKind(header.kind.get()),
            data,
        })
    }
}

// Test that empty input or malformed line data (too little data) does not cause the iterator to fail.
// The iterator will return `None`.
#[test]
fn empty_or_malformed_input() {
    use dump_utils::HexDump;

    static CASES: &[(&str, &[u8])] = &[
        ("empty input", &[]),
        ("incomplete subsection_kind", &[0xf1, 0]),
        ("incomplete subsection_size", &[0xf1, 0, 0, 0, 0xff, 0xff]),
        (
            "incomplete subsection_data",
            &[
                0xf1, 0, 0, 0, // subsection_Kind
                0, 0, 1, 0, // subsection_size (0x10000)
            ],
        ),
    ];

    for &(case_name, case_data) in CASES.iter() {
        // Test the subsection iterator
        println!("case: {}\n{:?}", case_name, HexDump::new(case_data));
        let ld = LineData::new(case_data);
        assert_eq!(ld.subsections().count(), 0);
        assert!(ld.find_checksums().is_none());
        assert!(ld.find_checksums_bytes().is_none());
        ld.iter_name_index(|_name| panic!("should never be called"))
            .unwrap();

        // Do the same thing with a mutable iterator.
        let mut case_data_mut = case_data.to_vec();
        let mut ld = LineDataMut::new(&mut case_data_mut);
        assert_eq!(ld.subsections_mut().count(), 0);
    }
}

/// Test the alignment padding code in the subsection iterator.
#[test]
fn test_subsection_alignment() {
    const PAD: u8 = 0xaa;

    #[rustfmt::skip]
    static DATA: &[u8] = &[
                                // -----subsection 0 -----
        0xf4, 0, 0, 0,          // subsection_kind: DEBUG_S_FILECHKSMS
        0x2, 0, 0, 0,           // subsection_size (unaligned len = 2)
        0xab, 0xcd,             // subsection_data
        PAD, PAD,               // 2 padding bytes
                                // ----- subsection 1 -----
        0xf5, 0, 0, 0,          // subsection_kind: FRAMEDATA
        7, 0, 0, 0,             // subsection_size: 7 (unaligned len = 3)
        1, 2, 3, 4, 5, 6, 7,    // subsection_data
        PAD,                    // 1 padding byte
                                // ----- subsection 2 -----
        0xf6, 0, 0, 0,          // subsection_kind: INLINEELINES
        8, 0, 0, 0,             // subsection_size: 8 (unaligned len = 0)
        8, 7, 6, 5, 4, 3, 2, 1, // subsection_data
                                // no padding bytes
                                // ----- subsection 3 -----
        0xf7, 0, 0, 0,          // subsection_kind: CROSSSCOPEIMPORTS
        5, 0, 0, 0,             // subsection_size: 5 (unaligned len = 1)
        10, 11, 12, 13, 14,     // subsection_data
        PAD, PAD, PAD,          // 3 padding bytes


    ];

    // Test SubsectionsIter
    {
        let mut iter = LineData::new(DATA).subsections();

        let sub0 = iter.next().unwrap();
        assert_eq!(sub0.kind, SubsectionKind::FILE_CHECKSUMS);
        assert_eq!(sub0.data, &[0xab, 0xcd]);

        let sub1 = iter.next().unwrap();
        assert_eq!(sub1.kind, SubsectionKind::FRAMEDATA);
        assert_eq!(sub1.data, &[1, 2, 3, 4, 5, 6, 7]);

        let sub2 = iter.next().unwrap();
        assert_eq!(sub2.kind, SubsectionKind::INLINEELINES);
        assert_eq!(sub2.data, &[8, 7, 6, 5, 4, 3, 2, 1]);

        let sub3 = iter.next().unwrap();
        assert_eq!(sub3.kind, SubsectionKind::CROSSSCOPEIMPORTS);
        assert_eq!(sub3.data, &[10, 11, 12, 13, 14]);

        assert!(iter.rest().is_empty());
    }

    // Test SubsectionIterMut
    // We repeat the tests because we can't do generics over mutability, and the implementations of
    // SubsectionIter and SubsectionIterMut
    {
        let mut data_mut = DATA.to_vec();
        let mut iter = SubsectionIterMut::new(&mut data_mut);

        let sub0 = iter.next().unwrap();
        assert_eq!(sub0.kind, SubsectionKind::FILE_CHECKSUMS);
        assert_eq!(sub0.data, &[0xab, 0xcd]);

        let sub1 = iter.next().unwrap();
        assert_eq!(sub1.kind, SubsectionKind::FRAMEDATA);
        assert_eq!(sub1.data, &[1, 2, 3, 4, 5, 6, 7]);

        let sub2 = iter.next().unwrap();
        assert_eq!(sub2.kind, SubsectionKind::INLINEELINES);
        assert_eq!(sub2.data, &[8, 7, 6, 5, 4, 3, 2, 1]);

        let sub3 = iter.next().unwrap();
        assert_eq!(sub3.kind, SubsectionKind::CROSSSCOPEIMPORTS);
        assert_eq!(sub3.data, &[10, 11, 12, 13, 14]);

        assert!(iter.rest().is_empty());
    }
}
