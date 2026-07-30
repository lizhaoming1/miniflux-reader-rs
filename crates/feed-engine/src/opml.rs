//! OPML 2.0 import / export.

use std::io::Cursor;

use progress_db::Feed;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::{Reader, Writer};

/// Parse OPML XML and return every `xmlUrl` attribute value found.
///
/// Invalid XML or XML that does not match OPML structure simply returns
/// an empty vec (no feed urls). This is deliberate: the caller is able
/// to surface the count of successfully added vs total urls.
pub fn parse_opml(xml: &str) -> Vec<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut urls = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                if e.local_name().as_ref() == b"outline" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref().eq_ignore_ascii_case(b"xmlurl") {
                            if let Ok(v) = attr.unescape_value() {
                                let s = v.into_owned();
                                if !s.is_empty() {
                                    urls.push(s);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => break, // malformed xml just returns whatever we already found
            _ => {}
        }
        buf.clear();
    }
    urls
}

/// Export a list of `Feed` records into an OPML 2.0 document.
pub fn export_opml(feeds: &[Feed]) -> String {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("utf-8"), None)))
        .ok();
    let mut opml = BytesStart::new("opml");
    opml.push_attribute(("version", "2.0"));
    writer.write_event(Event::Start(opml)).ok();

    writer.write_event(Event::Start(BytesStart::new("head"))).ok();
    writer.write_event(Event::Start(BytesStart::new("title"))).ok();
    writer
        .write_event(Event::Text(BytesText::new(
            "miniflux-reader-rs subscriptions",
        )))
        .ok();
    writer.write_event(Event::End(BytesEnd::new("title"))).ok();
    writer.write_event(Event::End(BytesEnd::new("head"))).ok();

    writer.write_event(Event::Start(BytesStart::new("body"))).ok();
    for f in feeds {
        let mut outline = BytesStart::new("outline");
        outline.push_attribute(("type", "rss"));
        outline.push_attribute(("text", f.title.as_str()));
        outline.push_attribute(("xmlUrl", f.url.as_str()));
        writer.write_event(Event::Empty(outline)).ok();
    }
    writer.write_event(Event::End(BytesEnd::new("body"))).ok();
    writer.write_event(Event::End(BytesEnd::new("opml"))).ok();

    let inner = writer.into_inner().into_inner();
    String::from_utf8(inner).unwrap_or_default()
}
