//! EPUB archive reader: opens a `.epub` (zip) file, parses the OPF package
//! document, and exposes metadata + chapter list.
//!
//! Implementation strategy:
//! 1. Read zip bytes into memory (`from_bytes`) or from disk (`open`).
//! 2. Read `META-INF/container.xml` → find OPF rootfile path.
//! 3. Parse OPF: `<metadata>` (dc:title, dc:creator), `<manifest>` (id →
//!    href + media-type), `<spine>` (ordered itemref idrefs).
//! 4. `chapter_at(idx)` reads the spine item's href from the zip and
//!    returns the raw XHTML text.

use std::io::{Cursor, Read};
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader as XmlReader;
use zip::ZipArchive;

use crate::error::{EpubError, Result};
use crate::{Chapter, EpubMetadata};

/// Reader for an opened EPUB archive. Holds the parsed zip + OPF metadata
/// so repeated chapter fetches do not re-read disk.
#[derive(Debug)]
pub struct EpubReader {
    /// Parsed metadata — `title`, `author`, `total_chapters`.
    pub metadata: EpubMetadata,
    /// (href, media_type) pairs for each spine item, in spine order.
    pub spine: Vec<(String, String)>,
    /// In-memory copy of the zip bytes so chapter extraction can reuse it
    /// without re-opening the file.
    zip_bytes: Vec<u8>,
}

impl EpubReader {
    /// Open `path`, read the zip into memory, parse `mimetype` +
    /// `META-INF/container.xml` → OPF → metadata + spine.
    pub fn open(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)?;
        Self::from_bytes(&bytes)
    }

    /// Open from in-memory bytes (used by tests that build a minimal zip
    /// without writing to disk).
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor)?;

        // 1. Read META-INF/container.xml → OPF rootfile path.
        let container_xml = read_zip_entry(&mut archive, "META-INF/container.xml")?;
        let opf_path = parse_container_rootfile(&container_xml)?;

        // 2. Read + parse OPF.
        let opf_xml = read_zip_entry(&mut archive, &opf_path)?;
        let (metadata, manifest, spine_idrefs) = parse_opf(&opf_xml)?;

        // 3. Resolve spine idrefs → (href, media_type) pairs.
        let mut spine: Vec<(String, String)> = Vec::with_capacity(spine_idrefs.len());
        for idref in &spine_idrefs {
            let (href, media_type) = manifest
                .iter()
                .find(|(id, _, _)| id == idref)
                .map(|(_, href, mt)| (href.clone(), mt.clone()))
                .ok_or_else(|| {
                    EpubError::MissingEntry(format!("spine idref '{idref}' not in manifest"))
                })?;
            // The href is relative to the OPF directory; join paths.
            let opf_dir = opf_path.rfind('/').map(|i| &opf_path[..=i]).unwrap_or("");
            let full_href = format!("{opf_dir}{href}");
            spine.push((full_href, media_type));
        }

        let total_chapters = spine.len();
        Ok(Self {
            metadata: EpubMetadata {
                title: metadata.0,
                author: metadata.1,
                total_chapters,
            },
            spine,
            zip_bytes: bytes.to_vec(),
        })
    }

    /// Extract the chapter at spine index `idx`. Returns the raw HTML text
    /// of the content document.
    pub fn chapter_at(&self, idx: usize) -> Result<Chapter> {
        let (href, _media_type) = self
            .spine
            .get(idx)
            .ok_or_else(|| EpubError::MissingEntry(format!("chapter index {idx} out of bounds")))?;
        let cursor = Cursor::new(&self.zip_bytes);
        let mut archive = ZipArchive::new(cursor)?;
        let html_text = read_zip_entry(&mut archive, href)?;
        Ok(Chapter {
            idx,
            title: format!("Chapter {}", idx + 1),
            html_text,
        })
    }
}

/// Read a single entry from the zip archive into a `String`.
fn read_zip_entry<R: std::io::Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String> {
    let mut file = archive
        .by_name(name)
        .map_err(|e| EpubError::MissingEntry(format!("entry '{name}': {e}")))?;
    let mut buf = String::new();
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Parse `META-INF/container.xml` and return the OPF rootfile full-path.
fn parse_container_rootfile(xml: &str) -> Result<String> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.name().as_ref() == b"rootfile" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"full-path" {
                            return Ok(String::from_utf8_lossy(attr.value.as_ref()).into_owned());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EpubError::Xml(e.to_string())),
            _ => {}
        }
    }
    Err(EpubError::MissingEntry(
        "no <rootfile> in container.xml".to_string(),
    ))
}

/// Parsed OPF result: ((title, author), manifest items, spine idrefs).
type OpfParsed = ((String, String), Vec<ManifestItem>, Vec<String>);

/// A manifest item: (id, href, media_type).
type ManifestItem = (String, String, String);

/// Parse OPF XML → (title, author), manifest [(id, href, media_type)], spine [idref].
fn parse_opf(xml: &str) -> Result<OpfParsed> {
    let mut reader = XmlReader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();

    let mut title = String::new();
    let mut author = String::new();
    let mut manifest: Vec<(String, String, String)> = Vec::new();
    let mut spine: Vec<String> = Vec::new();
    // Track current element for text collection (dc:title / dc:creator).
    let mut current_text_elem: Option<String> = None;
    let mut text_buf = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let local = local_name(&name);
                if local == "title" || local == "creator" {
                    current_text_elem = Some(local.clone());
                    text_buf.clear();
                } else if local == "item" {
                    collect_manifest_item(&mut manifest, &e);
                } else if local == "itemref" {
                    collect_spine_idref(&mut spine, &e);
                }
            }
            Ok(Event::Empty(e)) => {
                let name = String::from_utf8_lossy(e.name().as_ref()).into_owned();
                let local = local_name(&name);
                if local == "item" {
                    collect_manifest_item(&mut manifest, &e);
                } else if local == "itemref" {
                    collect_spine_idref(&mut spine, &e);
                }
            }
            Ok(Event::Text(t)) => {
                if current_text_elem.is_some() {
                    text_buf.push_str(&t.unescape().unwrap_or_default());
                }
            }
            Ok(Event::End(_e)) => {
                if let Some(elem) = current_text_elem.take() {
                    if elem == "title" && title.is_empty() {
                        title = text_buf.trim().to_string();
                    } else if elem == "creator" && author.is_empty() {
                        author = text_buf.trim().to_string();
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(EpubError::Xml(e.to_string())),
            _ => {}
        }
    }

    Ok(((title, author), manifest, spine))
}

/// Extract the local name (after `:` prefix) from a qualified XML name.
fn local_name(qname: &str) -> String {
    qname.rsplit(':').next().unwrap_or(qname).to_string()
}

/// Collect attributes from a `<item>` start/empty event into the manifest.
fn collect_manifest_item(manifest: &mut Vec<ManifestItem>, e: &quick_xml::events::BytesStart<'_>) {
    let mut id = String::new();
    let mut href = String::new();
    let mut mt = String::new();
    for attr in e.attributes().flatten() {
        match attr.key.as_ref() {
            b"id" => id = String::from_utf8_lossy(attr.value.as_ref()).into_owned(),
            b"href" => href = String::from_utf8_lossy(attr.value.as_ref()).into_owned(),
            b"media-type" => mt = String::from_utf8_lossy(attr.value.as_ref()).into_owned(),
            _ => {}
        }
    }
    if !id.is_empty() {
        manifest.push((id, href, mt));
    }
}

/// Collect the `idref` attribute from an `<itemref>` start/empty event.
fn collect_spine_idref(spine: &mut Vec<String>, e: &quick_xml::events::BytesStart<'_>) {
    for attr in e.attributes().flatten() {
        if attr.key.as_ref() == b"idref" {
            spine.push(String::from_utf8_lossy(attr.value.as_ref()).into_owned());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_rootfile_parse_extracts_path() {
        let xml = r#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#;
        let p = parse_container_rootfile(xml).expect("parse");
        assert_eq!(p, "OEBPS/content.opf");
    }
}
