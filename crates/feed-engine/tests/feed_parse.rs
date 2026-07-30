use feed_engine::parse::parse_feed_bytes;

fn rss_fixture() -> &'static [u8] {
    br#"<?xml version="1.0"?>
<rss version="2.0">
  <channel>
    <title>Example RSS</title>
    <link>https://example.com/</link>
    <description>Test</description>
    <item>
      <guid isPermaLink="false">article-1</guid>
      <title>First Post</title>
      <link>https://example.com/1</link>
      <description>summary 1</description>
      <pubDate>Thu, 30 Jul 2026 00:00:00 GMT</pubDate>
    </item>
  </channel>
</rss>"#
}

fn atom_fixture() -> &'static [u8] {
    br#"<?xml version="1.0" encoding="utf-8"?>
<feed xmlns="http://www.w3.org/2005/Atom">
  <title>Example Atom</title>
  <link href="https://example.com/" rel="alternate"/>
  <entry>
    <id>atom-entry-1</id>
    <title>Atom First</title>
    <link href="https://example.com/a1" rel="alternate"/>
    <updated>2026-07-30T00:00:00Z</updated>
    <summary>summary atom</summary>
  </entry>
</feed>"#
}

#[test]
fn t01_rss_parses_title_and_entries() {
    let f = parse_feed_bytes(rss_fixture()).expect("rss parse");
    assert_eq!(f.title, "Example RSS");
    assert_eq!(f.site_url, "https://example.com/");
    assert_eq!(f.articles.len(), 1);
    assert_eq!(f.articles[0].title, "First Post");
    assert_eq!(f.articles[0].guid, "article-1");
}

#[test]
fn t02_atom_parses_title_and_entries() {
    let f = parse_feed_bytes(atom_fixture()).expect("atom parse");
    assert_eq!(f.title, "Example Atom");
    assert_eq!(f.site_url, "https://example.com/");
    assert_eq!(f.articles.len(), 1);
    assert_eq!(f.articles[0].title, "Atom First");
}

#[test]
fn t03_malformed_xml_returns_parse_err() {
    let err = parse_feed_bytes(b"not xml at all <<<").expect_err("should fail");
    assert!(matches!(err, feed_engine::FeedEngineError::Parse(_)));
}
