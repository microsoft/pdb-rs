#![allow(clippy::useless_vec)]

use super::*;
use anyhow::Result;
use dump_utils::{DumpRangesSucc, HexDump};
use std::io::{Cursor, Seek, SeekFrom, Write};
use std::sync::Mutex;

macro_rules! assert_bytes_eq {
    ($a:expr, $b:expr) => {
        match (&($a), &($b)) {
            (a, b) => {
                let a_bytes: &[u8] = a.as_ref();
                let b_bytes: &[u8] = b.as_ref();

                if a_bytes != b_bytes {
                    panic!(
                        "Bytes do not match:\n{:?}\n{:?}",
                        HexDump::new(a_bytes),
                        HexDump::new(b_bytes)
                    );
                }
            }
        }
    };

    ($a:expr, $b:expr, $($msg:tt)*) => {
        match (&($a), &($b)) {
            (a, b) => {
                let a_bytes: &[u8] = a.as_ref();
                let b_bytes: &[u8] = b.as_ref();

                if a_bytes != b_bytes {
                    let msg = format!($($msg)*);
                    panic!(
                        "Bytes do not match: {msg}\n{:?}\n{:?}",
                        HexDump::new(a_bytes),
                        HexDump::new(b_bytes)
                    );
                }
            }
        }
    };
}

struct WritePair<Test, Good> {
    test: Test,
    good: Good,
}

impl<Test: Write, Good: Write> std::io::Write for WritePair<Test, Good> {
    fn flush(&mut self) -> std::io::Result<()> {
        self.test.flush()?;
        self.good.flush()?;
        Ok(())
    }

    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let len_test = self.test.write(buf)?;
        let len_good = self.good.write(buf)?;
        assert_eq!(len_test, len_good);
        Ok(len_test)
    }

    fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.test.write_all(buf)?;
        self.good.write_all(buf)?;
        Ok(())
    }
}

impl<A: Seek, B: Seek> std::io::Seek for WritePair<A, B> {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        let pos_test = self.test.seek(pos)?;
        let pos_good = self.good.seek(pos)?;
        assert_eq!(pos_test, pos_good);
        Ok(pos_test)
    }
}

#[derive(Default)]
struct TestFile {
    data: Mutex<Vec<u8>>,
}

impl ReadAt for TestFile {
    fn read_exact_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<()> {
        debug!(
            "TestFile: read at 0x{:08x}, len 0x{:08x}",
            offset,
            buf.len()
        );
        let lock = self.data.lock().unwrap();
        lock.read_exact_at(buf, offset)?;
        debug!(
            "TestFile: read received:\n{:?}",
            HexDump::new(buf).at(offset as usize)
        );
        Ok(())
    }

    fn read_at(&self, buf: &mut [u8], offset: u64) -> std::io::Result<usize> {
        let lock = self.data.lock().unwrap();
        let n = lock.read_at(buf, offset)?;
        debug!(
            "TestFile: read received:\n{:?}",
            HexDump::new(&buf[0..n]).at(offset as usize)
        );
        Ok(n)
    }
}

impl WriteAt for TestFile {
    fn write_at(&self, buf: &[u8], offset: u64) -> std::io::Result<usize> {
        self.write_all_at(buf, offset)?;
        Ok(buf.len())
    }

    fn write_all_at(&self, buf: &[u8], offset: u64) -> std::io::Result<()> {
        debug!(
            "TestFile: write at 0x{:08x}, len 0x{:08x}\n{:?}",
            offset,
            buf.len(),
            HexDump::new(buf).at(offset as usize)
        );

        let mut lock = self.data.lock().unwrap();
        let vec: &mut Vec<u8> = &mut lock;

        let offset = offset as usize;

        if offset == vec.len() {
            vec.extend_from_slice(buf);
        } else {
            let new_len = buf.len() + offset;
            if new_len > vec.len() {
                vec.resize(new_len, 0);
            }
            vec[offset..offset + buf.len()].copy_from_slice(buf);
        }
        Ok(())
    }
}

struct Tester {
    msf: Msf<TestFile>,
}

fn tester() -> Tester {
    println!();

    let f = TestFile::default();
    let msf = Msf::create_for(f, CreateOptions::default()).unwrap();
    Tester { msf }
}

/// Contains enough state from an MSF file that we can fake up a StreamWriter for testing.
struct StreamTester {
    file: TestFile,
    stream_size: u32,
    page_allocator: PageAllocator,
    pages: Vec<Page>,
    expected_stream_data: Vec<u8>,
}

impl StreamTester {
    fn new() -> Self {
        Self {
            file: Default::default(),
            stream_size: 0,
            page_allocator: PageAllocator::new(0x100, DEFAULT_PAGE_SIZE),
            pages: Vec::new(),
            expected_stream_data: Vec::new(),
        }
    }

    fn writer(&mut self) -> WritePair<StreamWriter<'_, TestFile>, Cursor<&mut Vec<u8>>> {
        let good = Cursor::new(&mut self.expected_stream_data);

        let test = StreamWriter {
            file: &mut self.file,
            size: &mut self.stream_size,
            page_allocator: &mut self.page_allocator,
            pages: &mut self.pages,
            pos: 0,
        };

        WritePair { good, test }
    }

    #[inline(never)]
    #[track_caller]
    fn write_at(&mut self, pos: u64, data: &[u8]) {
        debug!("----- Writing piece -----");
        debug!("current stream size: 0x{:x}", self.stream_size);
        debug!("piece contents:\n{:?}", HexDump::new(data).at(pos as usize));

        let mut w = self.writer();
        w.seek(SeekFrom::Start(pos)).unwrap();
        w.write_all(data).unwrap();
        self.check_data();
    }

    // Verifies that the data stored in the stream is consistent with the data that we also wrote
    // into expected_stream_data.
    #[track_caller]
    fn check_data(&self) {
        assert_eq!(
            self.stream_size as usize,
            self.expected_stream_data.len(),
            "stream sizes should be same"
        );

        let page_size = self.page_allocator.page_size;

        assert_eq!(
            num_pages_for_stream_size(self.stream_size, page_size) as usize,
            self.pages.len(),
            "number of pages should be consistent with stream size"
        );

        debug!("Stream pages: {:?}", DumpRangesSucc::new(&self.pages));

        let file = self.file.data.lock().unwrap();

        debug!("MSF contents:\n{:?}", HexDump::new(&file));

        for (spage, &page) in self.pages.iter().enumerate() {
            let whole_page_data =
                &file[page_to_offset(page, page_size) as usize..][..usize::from(page_size)];
            let page_start = (spage as u32) << page_size.exponent();
            let len_within_page = (self.stream_size - page_start).min(u32::from(page_size));

            // Page data from the MSF "file"
            let page_data = &whole_page_data[..len_within_page as usize];

            // Page data from our parallel file contents
            let expected_page_data = &self.expected_stream_data[(spage << page_size.exponent())..]
                [..len_within_page as usize];

            assert_bytes_eq!(expected_page_data, page_data, "Stream page {page}");
        }
    }
}

/// Create a stream but don't write anything to it.
#[test]
fn test_write_empty_stream() {
    let mut t = tester();

    let (si, _s) = t.msf.new_stream().unwrap();
    assert_eq!(si, 5);
    assert_eq!(t.msf.num_streams(), 6);

    t.msf.commit().unwrap();
}

/// Create a stream, do a single zero-length write to it.
#[test]
fn test_write_empty_buffer() {
    let mut t = tester();

    let (si, mut s) = t.msf.new_stream().unwrap();
    assert_eq!(si, 5);
    s.write_all(&[]).unwrap();

    assert_eq!(t.msf.num_streams(), 6);

    t.msf.commit().unwrap();
}

/// Create a stream, write a small amount of data into it
#[test]
fn test_write_hello_world() {
    let mut st = StreamTester::new();
    st.write_at(0, b"Hello, world!");
}

#[test]
fn test_write_simple() {
    let mut st = StreamTester::new();
    st.write_at(0, b"Alpha_");
    st.write_at(6, b"Bravo_");
    st.write_at(12, b"Charlie_");
    st.write_at(6, b"Delta_");
}

// Zero-extend with a small amount of data that does not cross the page boundary where zero-extend starts.
#[test]
fn test_zero_extend_unaligned_start_1() {
    let mut st = StreamTester::new();
    st.write_at(10, b"Hello!");
}

// Zero-extend with a small amount of data that DOES cross the page boundary where zero-extend starts.
// This also zero-extends several complete pages.
#[test]
fn test_zero_extend_unaligned_start_cross_page_many() {
    let mut st = StreamTester::new();
    st.write_at(0, b"Hello");
    st.write_at(0x2ffe, b"World!");
}

// unaligned start, finishes within a single page
#[test]
fn test_zero_extend_unaligned_start_single_page() {
    let mut st = StreamTester::new();
    st.write_at(0, b"old");
    // <-- zero extend 7 bytes
    st.write_at(10, b"new");
}

#[test]
fn test_zero_extend_unaligned_start_cross_pages_aligned_end() {
    let mut st = StreamTester::new();
    st.write_at(0, b"old");
    st.write_at(10, &vec![0xaa; 0x1ff6]); // ends at page-aligned boundary
    assert_eq!(st.stream_size, 0x2000);
}

#[test]
fn test_zero_extend_unaligned_start_cross_pages_unaligned_end() {
    let mut st = StreamTester::new();
    st.write_at(0, b"old");
    st.write_at(10, &vec![0xaa; 0x2000]);
    assert_eq!(st.stream_size, 0x200a);
}

#[test]
fn test_zero_extend_aligned_start_unaligned_end() {
    let mut st = StreamTester::new();
    st.write_at(0x2000, b"alpha");
}

#[test]
fn test_zero_extend_aligned_start_pages_unaligned_end() {
    let mut st = StreamTester::new();
    st.write_at(0x0000, &vec![0xaa; 0x1000]);
    st.write_at(0x2010, b"alpha");
}

// aligned start, does not extend stream, existing stream page is unaligned
#[test]
fn test_overwrite_aligned_start_single_page() {
    let mut st = StreamTester::new();
    st.write_at(0, b"alpha bravo charlie delta");
    st.write_at(0, b"TANGO");
}

// unaligned start, does not extend stream, existing stream page is unaligned
#[test]
fn test_overwrite_unaligned_start_single_page() {
    let mut st = StreamTester::new();
    st.write_at(0, b"alpha bravo charlie delta");
    st.write_at(6, b"TANGO");
}

// unaligned start, extends stream, existing stream page is unaligned
#[test]
fn test_overwrite_case_unaligned_extend_within_page() {
    let mut st = StreamTester::new();
    st.write_at(0, b"alpha bravo");
    st.write_at(12, b"TANGO");
}

// unaligned start, does not extend stream, existing stream page is unaligned
#[test]
fn test_overwrite_case_unaligned_extend_across_pages() {
    let mut st = StreamTester::new();
    st.write_at(0, b"alpha bravo");
    let big = FRIENDS_ROMANS.repeat(10);
    println!("big length = 0x{:x}", big.len());
    st.write_at(12, big.as_bytes());
}

#[test]
fn test_overwrite_case_many_pages() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0x10_000]); // write lots of data
    st.write_at(0x0_f00, FRIENDS_ROMANS.as_bytes()); // get some shakespeare
    st.write_at(0x1_f00, FRIENDS_ROMANS.repeat(10).as_bytes());
}

// This tests the case in write_overwrite_aligned_pages() where we overwrite an unaligned portion
// of a page. buf.len() is too small to cover the page, but the existing stream does have enough
// pages assigned to it to cover it.
#[test]
fn test_overwrite_case_unaligned_end() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0x2_000]);
    st.write_at(0xffe, b"abcd");
}

#[test]
fn test_overwrite_case_zzz_1() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0x1_005]);
    st.write_at(0xffe, b"__abc");
}

#[test]
fn test_overwrite_case_zzz_2() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0x1_005]);
    st.write_at(0xffe, b"__abcde");
}

#[test]
fn test_overwrite_case_zzz_3() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0x1_005]);
    st.write_at(0xffe, b"__abcdefgh");
}

#[test]
fn test_overwrite_case_c() {
    let mut st = StreamTester::new();
    st.write_at(0, &vec![0xcc; 0xc]);
    st.write_at(0, &vec![0xaa; 0xaaaa]);
}

const FRIENDS_ROMANS: &str = r#"
Friends, Romans, countrymen, lend me your ears;
I come to bury Caesar, not to praise him.
The evil that men do lives after them;
The good is oft interred with their bones;
So let it be with Caesar. The noble Brutus
Hath told you Caesar was ambitious:
If it were so, it was a grievous fault,
And grievously hath Caesar answer'd it.
Here, under leave of Brutus and the rest-
For Brutus is an honourable man;
So are they all, all honourable men-
Come I to speak in Caesar's funeral.
He was my friend, faithful and just to me:
But Brutus says he was ambitious;
And Brutus is an honourable man.
He hath brought many captives home to Rome
Whose ransoms did the general coffers fill:
Did this in Caesar seem ambitious?
When that the poor have cried, Caesar hath wept:
Ambition should be made of sterner stuff:
Yet Brutus says he was ambitious;
And Brutus is an honourable man.
You all did see that on the Lupercal
I thrice presented him a kingly crown,
Which he did thrice refuse: was this ambition?
Yet Brutus says he was ambitious;
And, sure, he is an honourable man.
I speak not to disprove what Brutus spoke,
But here I am to speak what I do know.
You all did love him once, not without cause:
What cause withholds you then, to mourn for him?
O judgment! thou art fled to brutish beasts,
And men have lost their reason. Bear with me;
My heart is in the coffin there with Caesar,
And I must pause till it come back to me.
"#;

#[test]
fn test_write_many_pieces() {
    let mut st = StreamTester::new();
    st.write_at(0, b"Alpha_");
    st.write_at(6, b"Bravo_");
    st.write_at(12, b"Charlie_");
    st.write_at(6, b"Delta_");
    st.write_at(50, b"Zulu");
    st.write_at(0, b"__Wiffleball__");
    st.write_at(5, b"__Garrus__");
}

#[test]
fn test_write_x() {
    let mut st = StreamTester::new();
    st.write_at(0x35, b"!");
    st.write_at(0, b"zzz");
}

#[test]
fn msf_write_multi_streams() {
    let mut t = tester();

    {
        let (_si1, mut sw1) = t.msf.new_stream().unwrap();
        sw1.write_all(b"Hello, world!").unwrap();
    }

    {
        let (_si2, mut sw2) = t.msf.new_stream().unwrap();
        sw2.write_all(b"Hallo Welt!").unwrap();
    }

    {
        let (_si2, mut sw2) = t.msf.new_stream().unwrap();
        sw2.write_all(b"Salut tout le monde!").unwrap();
    }
}

fn writer() -> Msf<TestFile> {
    let f = TestFile::default();
    Msf::create_for(f, Default::default()).unwrap()
}

fn finish_and_dump(mut w: Msf<TestFile>) {
    match w.commit() {
        Err(e) => {
            panic!("PdbWriter::commit failed: {}", e);
        }
        Ok(_wrote_any) => {
            let data_guard = w.file.data.lock().unwrap();
            let data: &[u8] = &data_guard;

            println!(
                "Finished PDB.  Size = 0x{:x} {}:\n{:#?}",
                data.len(),
                data.len(),
                HexDump::new(data)
            );
        }
    }
}

#[test]
fn empty_pdb() {
    let w = writer();
    finish_and_dump(w);
}

#[test]
fn one_stream_hello_world() -> anyhow::Result<()> {
    let mut w = writer();

    let (_, mut s) = w.new_stream()?;
    s.write_all("Hello, world!".as_bytes())?;

    finish_and_dump(w);
    Ok(())
}

#[test]
fn simple_multiple_streams() -> anyhow::Result<()> {
    let mut w = writer();

    let (si, mut s) = w.new_stream()?;
    assert_eq!(si, 5);
    s.write_all("Friends, Romans, countrymen, lend me your ears.".as_bytes())?;

    let (si, mut s) = w.new_stream()?;
    assert_eq!(si, 6);
    s.write_all("I come to bury Caesar, not to praise him.".as_bytes())?;

    let (si, mut s) = w.new_stream()?;
    assert_eq!(si, 7);
    s.write_all("The evil that men do lives after them.".as_bytes())?;

    let (si, mut s) = w.new_stream()?;
    assert_eq!(si, 8);
    s.write_all("I come to bury Caesar, not to praise him.".as_bytes())?;

    let (si, mut s) = w.new_stream()?;
    assert_eq!(si, 9);
    s.write_all("So let it be with Caesar.".as_bytes())?;

    finish_and_dump(w);
    Ok(())
}

#[test]
fn mix_and_match() -> Result<()> {
    let mut w = writer();

    let (si0, _s0) = w.new_stream()?;
    let (si1, _s1) = w.new_stream()?;
    let (si2, _s2) = w.new_stream()?;

    w.write_stream(si0)?.write_all("Sponge Bob!".as_bytes())?;
    w.write_stream(si1)?.write_all("Squidward!".as_bytes())?;
    w.write_stream(si2)?.write_all("Mr Crabs!".as_bytes())?;
    w.write_stream(si0)?.write_all("Square Pants!".as_bytes())?; // should land on same page

    let mut w1 = w.write_stream(si1)?;
    w1.seek(SeekFrom::Start(0x2000))?;
    w1.write_all("Peace and Quiet...".as_bytes())?; // new page

    assert_eq!(w.write_stream(si0)?.pages, &[3]);
    assert_eq!(w.write_stream(si1)?.pages, &[4, 6, 7]);
    assert_eq!(w.write_stream(si2)?.pages, &[5]);

    finish_and_dump(w);

    Ok(())
}
