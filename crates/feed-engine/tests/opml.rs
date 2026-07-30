use feed_engine::opml::{export_opml, parse_opml};
use progress_db::Feed;

#[test]
fn t01_parse_opml_extracts_xmlurl_attributes() {
    let xml = r#"<?xml version="1.0"?>
<opml version="2.0">
  <head><title>Subs</title></head>
  <body>
    <outline text="HN" xmlUrl="https://hnrss.org/frontpage"/>
    <outline text="LWN" xmlurl="https://lwn.net/headlines/rss"/>
  </body>
</opml>"#;
    let urls = parse_opml(xml);
    assert!(urls.iter().any(|u| u == "https://hnrss.org/frontpage"));
    assert!(urls.iter().any(|u| u == "https://lwn.net/headlines/rss"));
}

#[test]
fn t02_export_opml_roundtrip_through_parse() {
    let feeds = vec![
        Feed {
            id: 1,
            url: "https://a.example.com/rss".into(),
            title: "A".into(),
            site_url: "https://a.example.com".into(),
            last_fetched: None,
            fetch_error: None,
        },
        Feed {
            id: 2,
            url: "https://b.example.com/atom".into(),
            title: "B".into(),
            site_url: "https://b.example.com".into(),
            last_fetched: None,
            fetch_error: None,
        },
    ];
    let out = export_opml(&feeds);
    assert!(out.contains(r#"xmlUrl="https://a.example.com/rss""#));
    assert!(out.contains(r#"xmlUrl="https://b.example.com/atom""#));
    // Round trip: our exported OPML parses back to the same urls.
    let urls = parse_opml(&out);
    assert_eq!(urls.len(), 2);
    assert!(urls.contains(&"https://a.example.com/rss".to_string()));
}
