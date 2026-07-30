//! PR#5 — `MockTranslateService` TDD: fixtures, empty, too-long, call_count,
//! concurrency, clone-shared counter.
//!
//! Tests are numbered T0..T5 to mirror the PR spec.

use services::*;

/// Three fixed zh fixtures used across the tests below.
fn fixtures() -> Vec<String> {
    vec!["你好".to_string(), "世界".to_string(), "今天".to_string()]
}

#[tokio::test]
async fn t0_batch_of_3_returns_3_fixed_zh_1_to_1() {
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    let src = vec![
        "hello".to_string(),
        "world".to_string(),
        "today".to_string(),
    ];
    let out = svc.translate_batch(src).await.expect("translate");
    assert_eq!(out.len(), 3);
    assert_eq!(out[0], "你好");
    assert_eq!(out[1], "世界");
    assert_eq!(out[2], "今天");
}

#[tokio::test]
async fn t1_empty_batch_returns_empty_vec() {
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    let out = svc.translate_batch(Vec::new()).await.expect("translate");
    assert!(out.is_empty());
}

#[tokio::test]
async fn t2_batch_too_long_returns_text_too_long() {
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    let src = vec![
        "a".to_string(),
        "b".to_string(),
        "c".to_string(),
        "d".to_string(),
    ];
    let err = svc.translate_batch(src.clone()).await.unwrap_err();
    match err {
        ServiceError::TextTooLong(n) => assert_eq!(n, src.len()),
        other => panic!("expected TextTooLong, got {other:?}"),
    }
}

#[tokio::test]
async fn t3_call_count_increments_across_two_calls() {
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    assert_eq!(svc.call_count(), 0);
    let _ = svc
        .translate_batch(vec!["hello".to_string()])
        .await
        .expect("translate 1");
    assert_eq!(svc.call_count(), 1);
    let _ = svc
        .translate_batch(vec!["world".to_string()])
        .await
        .expect("translate 2");
    assert_eq!(svc.call_count(), 2);
}

#[tokio::test]
async fn t4_eight_concurrent_spawns_no_panic_no_deadlock() {
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    let mut handles = Vec::new();
    for _ in 0..8 {
        let svc = svc.clone();
        handles.push(tokio::spawn(async move {
            let src = vec!["hello".to_string(), "world".to_string()];
            let out = svc.translate_batch(src).await.expect("translate");
            assert_eq!(out.len(), 2);
        }));
    }
    for h in handles {
        h.await.expect("join");
    }
    assert_eq!(svc.call_count(), 8);
}

#[tokio::test]
async fn t5_clone_shares_call_count_between_instances() {
    // Design choice: `call_count` lives behind an `Arc<AtomicUsize>`, so
    // clones share the same counter. This test asserts that shared
    // semantics: calls on the clone are visible to the original and vice
    // versa, and both report the same total.
    let svc = MockTranslateService::with_fixed_vec(fixtures());
    let clone = svc.clone();
    let _ = svc
        .translate_batch(vec!["hello".to_string()])
        .await
        .expect("translate on original");
    let _ = clone
        .translate_batch(vec!["world".to_string()])
        .await
        .expect("translate on clone");
    assert_eq!(svc.call_count(), 2);
    assert_eq!(clone.call_count(), 2);
}
