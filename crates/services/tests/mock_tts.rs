//! PR#5 — `MockTtsService` TDD: MP3 header, highlights, empty, clone, timeout.
//!
//! Tests are numbered T0..T5 to mirror the PR spec, plus a bonus T6 that
//! exercises the simulated-timeout path.

use services::*;
use std::time::Duration;

#[tokio::test]
async fn t0_synthesize_returns_at_least_4_bytes() {
    let svc = MockTtsService::default();
    let bytes = svc.synthesize("你好").await.expect("synthesize");
    assert!(bytes.len() >= 4, "expected >=4 bytes, got {}", bytes.len());
}

#[tokio::test]
async fn t1_highlights_three_words_monotonic_start_end_gt_start() {
    let svc = MockTtsService::default();
    let tokens = svc
        .highlights("word1 word2 word3")
        .await
        .expect("highlights");
    assert_eq!(tokens.len(), 3);
    let mut prev_start: Option<u32> = None;
    for t in &tokens {
        if let Some(p) = prev_start {
            assert!(t.start_ms > p, "start must be strictly monotonic");
        }
        assert!(t.end_ms > t.start_ms, "end must be > start");
        prev_start = Some(t.start_ms);
    }
}

#[tokio::test]
async fn t2_synthesize_empty_text_returns_empty_bytes() {
    let svc = MockTtsService::default();
    let bytes = svc.synthesize("").await.expect("synthesize");
    assert!(bytes.is_empty());
}

#[tokio::test]
async fn t3_highlights_empty_text_returns_empty_vec() {
    let svc = MockTtsService::default();
    let tokens = svc.highlights("").await.expect("highlights");
    assert!(tokens.is_empty());
}

#[tokio::test]
async fn t4_clone_returns_identical_results() {
    let svc = MockTtsService::default();
    let clone = svc.clone();
    let a = svc.synthesize("hello").await.expect("synthesize a");
    let b = clone.synthesize("hello").await.expect("synthesize b");
    assert_eq!(a, b);
    let ha = svc.highlights("hello world").await.expect("highlights a");
    let hb = clone.highlights("hello world").await.expect("highlights b");
    assert_eq!(ha.len(), hb.len());
    for (x, y) in ha.iter().zip(&hb) {
        assert_eq!(x.word, y.word);
        assert_eq!(x.start_ms, y.start_ms);
        assert_eq!(x.end_ms, y.end_ms);
    }
}

#[tokio::test]
async fn t5_synthesize_first_two_bytes_are_mp3_header() {
    let svc = MockTtsService::default();
    let bytes = svc.synthesize("anything").await.expect("synthesize");
    assert_eq!(bytes[0], 0xFF);
    assert_eq!(bytes[1], 0xFB);
}

#[tokio::test]
async fn t6_synthesize_very_long_with_tight_timeout_returns_timeout() {
    let svc = MockTtsService::with_timeout(Duration::from_millis(1));
    let long = "x".repeat(2048);
    let err = svc.synthesize(&long).await.unwrap_err();
    match err {
        ServiceError::Timeout(d) => assert_eq!(d, Duration::from_millis(1)),
        other => panic!("expected Timeout, got {other:?}"),
    }
}
