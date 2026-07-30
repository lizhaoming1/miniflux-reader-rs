//! PR#3 Task 3.1 — RED phase: minimal-valid EPUB3.0 in-memory zip parse.
//!
//! Builds a complete (though tiny) EPUB3 zip entirely in-test using the
//! `zip` crate's writer, then asserts `EpubReader::from_bytes` parses it
//! and yields the expected metadata + chapter count.

use epub_lib::EpubReader;
use std::io::{Cursor, Write};

/// Build a minimal-valid EPUB3.0 archive entirely in memory.
/// Returns the raw zip bytes.
fn build_minimal_epub3() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();

        // 1. mimetype (must be first, stored, no compression)
        zip.start_file(
            "mimetype",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        // 2. META-INF/container.xml — points to OPF
        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles>
    <rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/>
  </rootfiles>
</container>"#,
        )
        .unwrap();

        // 3. OEBPS/content.opf — package + metadata + spine (2 items)
        zip.start_file("OEBPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Minimal Test Book</dc:title>
    <dc:creator>Test Author</dc:creator>
    <dc:identifier id="bookid">test-id-001</dc:identifier>
    <dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="ch1" href="ch1.xhtml" media-type="application/xhtml+xml"/>
    <item id="ch2" href="ch2.xhtml" media-type="application/xhtml+xml"/>
    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>
  </manifest>
  <spine>
    <itemref idref="ch1"/>
    <itemref idref="ch2"/>
  </spine>
</package>"#,
        )
        .unwrap();

        // 4. Two chapter content docs
        zip.start_file("OEBPS/ch1.xhtml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Ch1</title></head>
<body><h1>Chapter One</h1><p>Hello world.</p></body></html>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/ch2.xhtml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml"><head><title>Ch2</title></head>
<body><h1>Chapter Two</h1><p>Good morning.</p></body></html>"#,
        )
        .unwrap();

        // 5. nav.xhtml (required by EPUB3)
        zip.start_file("OEBPS/nav.xhtml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
<head><title>Table of Contents</title></head>
<body><nav epub:type="toc"><ol><li><a href="ch1.xhtml">Chapter One</a></li>
<li><a href="ch2.xhtml">Chapter Two</a></li></ol></nav></body></html>"#,
        )
        .unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn parse_minimal_epub3_yields_correct_metadata() {
    let bytes = build_minimal_epub3();
    let reader = EpubReader::from_bytes(&bytes).expect("EPUB3 should parse");
    assert_eq!(reader.metadata.title, "Minimal Test Book");
    assert_eq!(reader.metadata.author, "Test Author");
}

#[test]
fn parse_minimal_epub3_reports_two_spine_chapters() {
    let bytes = build_minimal_epub3();
    let reader = EpubReader::from_bytes(&bytes).expect("EPUB3 should parse");
    assert_eq!(
        reader.metadata.total_chapters, 2,
        "spine has 2 itemrefs → total_chapters must be 2"
    );
    assert_eq!(reader.spine.len(), 2);
}

#[test]
fn parse_invalid_bytes_returns_err_not_panic() {
    let result = EpubReader::from_bytes(b"definitely not a zip");
    assert!(result.is_err(), "garbage bytes must error, not panic");
}
