#![allow(clippy::format_collect)]

use super::*;
use bstr::BStr;
use pow2::Pow2;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use sync_file::ReadAt;
use tracing::{debug_span, info, info_span, instrument};

#[static_init::dynamic(drop)]
static mut INIT_LOGGER: Option<tracy_client::Client> = {
    use tracing_subscriber::fmt::format::FmtSpan;
    use tracing_subscriber::layer::SubscriberExt;

    if let Ok(s) = std::env::var("ENABLE_TRACY") {
        if s == "1" {
            let client = tracy_client::Client::start();

            eprintln!("Enabling Tracy");
            tracing::subscriber::set_global_default(
                tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
            )
            .expect("setup tracy layer");
            return Some(client);
        }
    }

    tracing_subscriber::fmt::fmt()
        .compact()
        .with_max_level(tracing_subscriber::filter::LevelFilter::TRACE)
        .with_level(false)
        .with_file(true)
        .with_line_number(true)
        .with_span_events(FmtSpan::ENTER | FmtSpan::EXIT)
        .with_test_writer()
        .without_time()
        .with_ansi(false)
        .init();

    None
};

#[track_caller]
fn make_msfz<F>(f: F) -> Msfz<Vec<u8>>
where
    F: FnOnce(&mut MsfzWriter<Cursor<Vec<u8>>>),
{
    let _span = info_span!("make_msfz").entered();

    let cursor: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    let mut w = MsfzWriter::new(cursor).unwrap();

    // Make debugging easier
    w.set_stream_dir_compression(None);

    f(&mut w);

    info!("Streams:");
    for (i, stream_opt) in w.streams.iter().enumerate() {
        if let Some(stream) = stream_opt {
            info!(stream_index = i, fragments = ?stream.fragments);
        } else {
            info!(stream_index = i, "nil stream");
        }
    }

    info!("Encoded stream directory:");
    let stream_dir_bytes = encode_stream_dir(&w.streams);
    info!(stream_dir_bytes = stream_dir_bytes.as_slice());

    let (summary, returned_file) = w.finish().unwrap();
    info!(%summary);
    let msfz_data = returned_file.into_inner();

    info!(msfz_data = msfz_data.as_slice());

    match Msfz::from_file(msfz_data) {
        Ok(msfz) => msfz,
        Err(e) => {
            panic!("Failed to decode MSFZ file during test: {e:?}");
        }
    }
}

#[test]
#[instrument]
fn basic_compressed() {
    let r = make_msfz(|w| {
        w.reserve_num_streams(10);

        // write streams out of order
        w.stream_writer(4)
            .unwrap()
            .write_all(b"Hello, world!")
            .unwrap();
        w.stream_writer(1)
            .unwrap()
            .write_all(b"Friends, Romans, yadda yadda")
            .unwrap();

        // Write a "large" stream to stream 2
        let mut big_stream: Vec<u8> = vec![0; 0x1_0000];
        let mut big = Cursor::new(&mut big_stream);
        big.seek(SeekFrom::Start(0x1000)).unwrap();
        big.write_all(b"way out in the hinterlands").unwrap();
        big.into_inner();
        w.stream_writer(2).unwrap().write_all(&big_stream).unwrap();
    });

    // Now read it back.

    assert_eq!(r.num_streams(), 10);

    assert!(r.is_stream_valid(0)); // stream directory; reserved
    assert!(r.is_stream_valid(1)); // we wrote to it
    assert!(r.is_stream_valid(2)); // we wrote to it
    assert!(!r.is_stream_valid(3));
    assert!(r.is_stream_valid(4)); // we wrote to it
    assert!(!r.is_stream_valid(5));
    assert!(!r.is_stream_valid(6));
    assert!(!r.is_stream_valid(7));
    assert!(!r.is_stream_valid(8));
    assert!(!r.is_stream_valid(9));

    assert_eq!(r.stream_size(0), 0); // stream dir stream should always be zero-length

    {
        let stream1 = r.read_stream_to_cow(1).unwrap();
        assert_eq!(
            BStr::new(&stream1),
            BStr::new(b"Friends, Romans, yadda yadda")
        );
    }

    {
        let stream4 = r.read_stream_to_cow(4).unwrap();
        assert_eq!(BStr::new(&stream4), BStr::new(b"Hello, world!"));
    }

    {
        // test Seek + Read
        let msg = b"way out in the hinterlands";
        let mut buf: Vec<u8> = vec![0; msg.len()];
        let mut s = r.get_stream_reader(2).unwrap();
        s.seek(SeekFrom::Start(0x1000)).unwrap();
        s.read_exact(buf.as_mut_slice()).unwrap();
        assert_eq!(BStr::new(&buf), BStr::new(msg));

        // test ReadAt
        buf.clear();
        buf.resize(msg.len(), 0);
        s.read_at(buf.as_mut_slice(), 0x1000).unwrap();
        assert_eq!(BStr::new(&buf), BStr::new(msg));
    }

    // Verify that reading a nil stream works.
    assert_eq!(r.stream_size(3), 0); // check that nil stream is zero-length
    assert!(r.read_stream_to_cow(3).unwrap().is_empty());
    let mut sr = r.get_stream_reader(3).unwrap();
    assert_eq!(seek_read_span(&mut sr, 0, 0).unwrap(), &[]);
    assert_eq!(read_span_at(&sr, 0, 0).unwrap(), &[]);
}

/// Test the code for crossing chunk boundaries.
#[test]
#[instrument]
fn multi_chunks() {
    let r = make_msfz(|w| {
        w.reserve_num_streams(2);

        let mut sw = w.stream_writer(1).unwrap();
        sw.write_all(b"alpha ").unwrap(); // 0..6
        sw.end_chunk().unwrap();
        sw.write_all(b"bravo ").unwrap(); // 6..12
        sw.end_chunk().unwrap();
        sw.write_all(b"charlie").unwrap(); // 12..19
    });

    assert_eq!(r.num_streams(), 2);
    assert!(r.is_stream_valid(0)); // stream directory; reserved
    assert!(r.is_stream_valid(1)); // we wrote to it
    assert_eq!(r.stream_size(1), 19);

    // Verify that reading the entire stream works correctly.
    assert_eq!(
        BStr::new(&r.read_stream_to_cow(1).unwrap()),
        BStr::new(b"alpha bravo charlie")
    );

    // Verify that Read works correctly at various offsets.

    let mut sr = r.get_stream_reader(1).unwrap();

    let cases: &[(u64, usize, &[u8])] = &[
        (0, 4, b"alph"),          // within a chunk
        (0, 6, b"alpha "),        // complete chunk
        (0, 8, b"alpha br"),      // spans 2 chunks
        (1, 3, b"lph"),           // within a single chunk
        (1, 6, b"lpha b"),        // spans 2 chunks
        (0, 12, b"alpha bravo "), // exactly 2 chunks
    ];

    for &(offset, len, expected) in cases.iter() {
        let data = seek_read_span(&mut sr, offset, len).unwrap();
        assert_eq!(
            BStr::new(&data),
            BStr::new(expected),
            "offset: {offset}, len: {len}"
        );

        let data_at = read_span_at(&sr, offset, len).unwrap();
        assert_eq!(
            BStr::new(&data_at),
            BStr::new(expected),
            "offset: {offset}, len: {len}"
        );
    }
}

#[test]
#[instrument]
fn basic_uncompressed() {
    let big_text: String = (0..100)
        .map(|i| format!("This should compress quite well #{i}\n"))
        .collect();

    let r = make_msfz(|w| {
        w.reserve_num_streams(10);

        // Write a compressed stream.
        let mut sw = w.stream_writer(1).unwrap();
        sw.set_compression_enabled(false);
        sw.write_all(big_text.as_bytes()).unwrap();

        // Write an uncompressed stream.
        let mut sw = w.stream_writer(2).unwrap();
        sw.set_compression_enabled(false);
        sw.write_all(b"This text should not be compressed.")
            .unwrap();
    });

    let mut sr = r.get_stream_reader(1).unwrap();
    check_read_ranges(
        &mut sr,
        big_text.as_bytes(),
        &[
            (0, 0),
            (0, 50),
            (0, 100),
            (100, 50),
            (100, 50),
            (1000, 10),
            // keep on multiple lines
        ],
    );
}

#[test]
#[instrument]
fn uncompressed_stream_alignment() {
    let r = make_msfz(|w| {
        w.reserve_num_streams(5);

        assert_eq!(w.file.out.stream_position().unwrap() & 0xf, 0);

        // Write 3 bytes with no alignment requirement.
        let mut sw = w.stream_writer(1).unwrap();
        sw.set_compression_enabled(false);
        sw.set_alignment(Pow2::from_exponent(0));
        sw.write_all(b"alpha").unwrap();

        assert_eq!(w.file.out.stream_position().unwrap() & 0xf, 5);

        let mut sw = w.stream_writer(2).unwrap();
        sw.set_compression_enabled(false);
        sw.set_alignment(Pow2::from_exponent(16));
        sw.write_all(b"zzzz").unwrap();

        assert_eq!(w.file.out.stream_position().unwrap() & 0xf, 4);
    });

    drop(r);
}

#[test]
#[instrument]
fn interleaving() {
    // variant specifies bits for whether compression is enabled for various pieces.
    for variant in 0u64..16u64 {
        let vbit = |i: u32| variant & (1u64 << i) != 0;

        let r = make_msfz(|w| {
            w.reserve_num_streams(5);

            let mut sw = w.stream_writer(1).unwrap();
            sw.set_compression_enabled(vbit(0));
            sw.write_all(b"Hello, world!\n").unwrap();

            let mut sw = w.stream_writer(2).unwrap();
            sw.set_compression_enabled(vbit(1));
            sw.write_all(
                b"
The universe (which others call the Library) is composed of an indefinite,
perhaps infinite number of hexagonal galleries. In the center of each gallery is a ventillation
shaft, bounded by a low railing.
",
            )
            .unwrap();

            let mut sw = w.stream_writer(1).unwrap();
            sw.set_compression_enabled(vbit(2));
            sw.write_all(b"Goodbye, world!\n").unwrap();

            let mut sw = w.stream_writer(2).unwrap();
            sw.set_compression_enabled(vbit(3));
            sw.write_all(
                b"
From any hexagon one can see the floors above and below -- one after another, endlessly.
The arrangement of the galleries is always the same: Twenty bookshelves, five to each side,
line four of the hexagon's six sides; the height of the bookshelves, floor to ceiling, is
hardly greater than the height of a normal librarian.
",
            )
            .unwrap();

            let mut sw = w.stream_writer(1).unwrap();
            sw.write_all(b"Hello, again!\n").unwrap();

            let mut sw = w.stream_writer(2).unwrap();
            sw.write_all(
                b"
One of the hexagon's free sides opens onto a narrow sort of vestibule, which in turn opens onto
another gallery, identical to the first -- identical in fact to all.",
            )
            .unwrap();
        });

        let s1 = r.read_stream_to_cow(1).unwrap();
        assert_eq!(
            std::str::from_utf8(&s1).unwrap(),
            "Hello, world!\n\
             Goodbye, world!\n\
             Hello, again!\n"
        );

        let s2 = r.read_stream_to_cow(2).unwrap();
        assert_eq!(
            std::str::from_utf8(&s2).unwrap(),
            "
The universe (which others call the Library) is composed of an indefinite,
perhaps infinite number of hexagonal galleries. In the center of each gallery is a ventillation
shaft, bounded by a low railing.

From any hexagon one can see the floors above and below -- one after another, endlessly.
The arrangement of the galleries is always the same: Twenty bookshelves, five to each side,
line four of the hexagon's six sides; the height of the bookshelves, floor to ceiling, is
hardly greater than the height of a normal librarian.

One of the hexagon's free sides opens onto a narrow sort of vestibule, which in turn opens onto
another gallery, identical to the first -- identical in fact to all."
        );
    }
}

fn check_read_ranges<F: ReadAt>(
    sr: &mut StreamReader<'_, F>,
    known_good_data: &[u8],
    ranges: &[(u64, usize)],
) {
    let _span = debug_span!("check_read_ranges").entered();

    for &(offset, len) in ranges.iter() {
        let expected = &known_good_data[offset as usize..offset as usize + len];

        let data = seek_read_span(sr, offset, len).unwrap();
        assert_eq!(
            BStr::new(&data),
            BStr::new(expected),
            "offset: {offset}, len: {len}"
        );

        let data_at = read_span_at(sr, offset, len).unwrap();
        assert_eq!(
            BStr::new(&data_at),
            BStr::new(expected),
            "offset: {offset}, len: {len}"
        );
    }
}

fn seek_read_span<R: Read + Seek>(r: &mut R, offset: u64, len: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0; len];
    r.seek(SeekFrom::Start(offset))?;
    r.read_exact(buf.as_mut_slice())?;
    Ok(buf)
}

fn read_span_at<R: ReadAt>(r: &R, offset: u64, len: usize) -> Result<Vec<u8>> {
    let mut buf = vec![0; len];
    r.read_at(buf.as_mut_slice(), offset)?;
    Ok(buf)
}
