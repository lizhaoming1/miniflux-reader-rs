//! PR#3 Task 3.3 — RED phase: chapter text extraction from a 2-chapter EPUB.

use epub_lib::EpubReader;
use std::io::{Cursor, Write};

fn build_two_chapter_epub() -> Vec<u8> {
    let mut buf: Vec<u8> = Vec::new();
    {
        let mut zip = zip::ZipWriter::new(Cursor::new(&mut buf));
        let opts = zip::write::SimpleFileOptions::default();

        zip.start_file(
            "mimetype",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(b"application/epub+zip").unwrap();

        zip.start_file("META-INF/container.xml", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<?xml version="1.0" encoding="UTF-8"?>
<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="bookid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Two Chapter Book</dc:title><dc:identifier id="bookid">t2</dc:identifier><dc:language>en</dc:language>
  </metadata>
  <manifest>
    <item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/>
    <item id="c2" href="c2.xhtml" media-type="application/xhtml+xml"/>
  </manifest>
  <spine><itemref idref="c1"/><itemref idref="c2"/></spine>
</package>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/c1.xhtml", opts).unwrap();
        zip.write_all(b"<html><body><p>Sentence alpha. Sentence beta.</p></body></html>")
            .unwrap();

        zip.start_file("OEBPS/c2.xhtml", opts).unwrap();
        zip.write_all(b"<html><body><p>Sentence gamma.</p></body></html>")
            .unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn chapter_zero_contains_exact_text_from_first_spine_item() {
    let bytes = build_two_chapter_epub();
    let reader = EpubReader::from_bytes(&bytes).expect("parse ok");
    let ch0 = reader.chapter_at(0).expect("chapter 0 exists");
    assert_eq!(ch0.idx, 0);
    assert!(
        ch0.html_text.contains("Sentence alpha."),
        "ch0 html must contain exact text; got: {}",
        ch0.html_text
    );
    assert!(ch0.html_text.contains("Sentence beta."));
}

#[test]
fn chapter_one_contains_exact_text_from_second_spine_item() {
    let bytes = build_two_chapter_epub();
    let reader = EpubReader::from_bytes(&bytes).expect("parse ok");
    let ch1 = reader.chapter_at(1).expect("chapter 1 exists");
    assert_eq!(ch1.idx, 1);
    assert!(
        ch1.html_text.contains("Sentence gamma."),
        "ch1 html must contain exact text; got: {}",
        ch1.html_text
    );
}

#[test]
fn chapter_index_out_of_bounds_returns_err() {
    let bytes = build_two_chapter_epub();
    let reader = EpubReader::from_bytes(&bytes).expect("parse ok");
    let result = reader.chapter_at(99);
    assert!(result.is_err(), "OOB chapter index must error, not panic");
}
