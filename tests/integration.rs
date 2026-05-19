use std::io::Write;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc;
use std::sync::Arc;

use rift::reader::index::{spawn_indexer, IndexMessage, IndexPhase};
use rift::reader::mmap::MmapReader;

fn write_temp(content: &[u8]) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::new().unwrap();
    f.write_all(content).unwrap();
    f.flush().unwrap();
    f
}

#[test]
fn mmap_reader_opens_utf8_fixture() {
    let path = std::path::Path::new("tests/fixtures/utf8.txt");
    let reader = MmapReader::open(path).unwrap();
    assert!(reader.file_size > 0);
    assert_eq!(reader.encoding, rift::reader::mmap::Encoding::Utf8);
}

#[test]
fn mmap_reader_opens_crlf_fixture() {
    let path = std::path::Path::new("tests/fixtures/crlf.txt");
    let reader = MmapReader::open(path).unwrap();
    assert!(reader.file_size > 0);
}

#[test]
fn mmap_line_bytes_at_reads_first_line() {
    let file = write_temp(b"hello world\nsecond line\n");
    let reader = MmapReader::open(file.path()).unwrap();
    let (line, next) = reader.line_bytes_at(0, 4096);
    assert_eq!(line, b"hello world");
    let (line2, _) = reader.line_bytes_at(next, 4096);
    assert_eq!(line2, b"second line");
}

#[test]
fn mmap_line_bytes_at_handles_no_trailing_newline() {
    let file = write_temp(b"only line");
    let reader = MmapReader::open(file.path()).unwrap();
    let (line, _) = reader.line_bytes_at(0, 4096);
    assert_eq!(line, b"only line");
}

#[test]
fn line_index_completes_for_small_file() {
    let file = write_temp(b"line1\nline2\nline3\n");
    let reader = MmapReader::open(file.path()).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let index = spawn_indexer(Arc::clone(&reader.mmap), reader.file_size, 64, cancel, tx);

    // Drain messages until Complete
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
            IndexMessage::Complete => break,
            IndexMessage::Error(e) => panic!("indexer error: {e}"),
            IndexMessage::Progress(_) => {}
        }
    }

    let idx = index.read().unwrap();
    assert_eq!(idx.phase(), IndexPhase::Complete);
    assert_eq!(idx.line_count(), 3);
}

#[test]
fn line_index_offset_for_line_zero_is_zero() {
    let file = write_temp(b"abc\ndef\n");
    let reader = MmapReader::open(file.path()).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let index = spawn_indexer(Arc::clone(&reader.mmap), reader.file_size, 64, cancel, tx);
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
            IndexMessage::Complete => break,
            IndexMessage::Error(e) => panic!("{e}"),
            IndexMessage::Progress(_) => {}
        }
    }
    let idx = index.read().unwrap();
    let pos = idx.offset_for_line(0);
    let offset = match pos {
        rift::reader::index::LinePosition::Exact { byte_offset, .. } => byte_offset,
        rift::reader::index::LinePosition::Estimated { byte_offset, .. } => byte_offset,
    };
    assert_eq!(offset, 0);
}

#[test]
fn line_index_line_at_offset_roundtrip() {
    let file = write_temp(b"aaa\nbbb\nccc\n");
    let reader = MmapReader::open(file.path()).unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let (tx, rx) = mpsc::channel();
    let index = spawn_indexer(Arc::clone(&reader.mmap), reader.file_size, 64, cancel, tx);
    loop {
        match rx.recv_timeout(std::time::Duration::from_secs(5)).unwrap() {
            IndexMessage::Complete => break,
            IndexMessage::Error(e) => panic!("{e}"),
            IndexMessage::Progress(_) => {}
        }
    }
    let idx = index.read().unwrap();
    // line 1 starts at byte offset 4 ("bbb\n")
    assert_eq!(idx.line_at_offset(4), 1);
    assert_eq!(idx.line_at_offset(8), 2);
}

#[test]
fn format_detector_identifies_plain_text() {
    use rift::highlight::FileFormat;
    use rift::highlight::FormatDetector;
    let bytes = b"Hello world\nThis is plain text\n";
    let fmt = FormatDetector::detect(bytes);
    assert!(matches!(fmt, FileFormat::PlainText));
}

#[test]
fn format_detector_identifies_json_lines() {
    use rift::highlight::FileFormat;
    use rift::highlight::FormatDetector;
    let bytes = b"{\"key\": \"value\"}\n{\"key\": \"value2\"}\n";
    let fmt = FormatDetector::detect(bytes);
    assert!(matches!(fmt, FileFormat::JsonLines));
}

#[test]
fn format_detector_identifies_log() {
    use rift::highlight::FileFormat;
    use rift::highlight::FormatDetector;
    let bytes = b"2026-01-01 ERROR something failed\n2026-01-01 INFO ok\n";
    let fmt = FormatDetector::detect(bytes);
    assert!(matches!(fmt, FileFormat::AnsiLog));
}

#[test]
fn format_detector_identifies_csv() {
    use rift::highlight::FileFormat;
    use rift::highlight::FormatDetector;
    let bytes = b"name,age,city\nalice,30,paris\nbob,25,lyon\n";
    let fmt = FormatDetector::detect(bytes);
    assert!(matches!(fmt, FileFormat::Csv { .. }));
}
