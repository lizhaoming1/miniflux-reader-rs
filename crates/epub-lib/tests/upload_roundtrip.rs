//! PR#3 Task 3.4 — RED phase: upload → disk → reopen roundtrip.

use epub_lib::{save_upload_to_disk, EpubReader};
use std::io::{Cursor, Write};

fn build_tiny_epub() -> Vec<u8> {
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
            br#"<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
<rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/content.opf", opts).unwrap();
        zip.write_all(
            br#"<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="b">
<metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
<dc:title>Roundtrip Test</dc:title><dc:identifier id="b">r1</dc:identifier><dc:language>en</dc:language>
</metadata>
<manifest><item id="c1" href="c1.xhtml" media-type="application/xhtml+xml"/></manifest>
<spine><itemref idref="c1"/></spine>
</package>"#,
        )
        .unwrap();

        zip.start_file("OEBPS/c1.xhtml", opts).unwrap();
        zip.write_all(b"<html><body><p>Roundtrip content.</p></body></html>")
            .unwrap();

        zip.finish().unwrap();
    }
    buf
}

#[test]
fn save_upload_then_reopen_yields_matching_title() {
    let bytes = build_tiny_epub();
    let safe = save_upload_to_disk(&bytes, "roundtrip-test.epub").expect("save ok");
    assert!(!safe.is_empty(), "safe name must be non-empty");

    // Find the file on disk. save_upload_to_disk should write to a
    // well-known location; we reconstruct the path from the safe name.
    // The safe name retains the original extension (.epub), so no extra
    // suffix is appended.
    let books_dir = std::env::temp_dir().join("rust-epub-books-test");
    let path = books_dir.join(&safe);
    assert!(path.exists(), "file should exist at {path:?}");

    let reader = EpubReader::open(&path).expect("reopen ok");
    assert_eq!(reader.metadata.title, "Roundtrip Test");
}

#[test]
fn save_upload_with_traversal_name_is_rejected_or_sanitised() {
    let bytes = build_tiny_epub();
    // The function must NOT write to `../../etc/evil.epub`. Either it
    // returns Err, or it sanitises to a safe basename that lands inside
    // the books dir. Either way, no traversal file must appear.
    let result = save_upload_to_disk(&bytes, "../../evil.epub");
    match result {
        Ok(safe) => {
            assert!(!safe.contains("..") && !safe.contains('/'));
            assert!(!safe.contains('\\'));
        }
        Err(_) => { /* rejection is also acceptable */ }
    }
}
