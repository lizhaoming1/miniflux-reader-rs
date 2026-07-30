//! PR#4 Task 4.1 — upsert_progress: save/get/upsert/edge tests.
//!
//! Each test owns its own TempDir SQLite file + fresh SqlitePool, runs
//! migrations, then exercises the ProgressRepository.

use progress_db::{run_migrations, ProgressRepository, ReadingProgress};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tempfile::TempDir;

async fn fresh_pool() -> (TempDir, SqlitePool) {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("rust-test.db");
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
        .expect("opts")
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(opts)
        .await
        .expect("connect");
    run_migrations(&pool).await.expect("migrate");
    (dir, pool)
}

#[tokio::test]
async fn save_new_row_then_read_back_matches_4_fields() {
    let _dir = TempDir::new().unwrap();
    let (_dir, pool) = fresh_pool().await;
    let repo = ProgressRepository::new(pool);

    let p = ReadingProgress {
        epub_path: "book-alpha.epub".into(),
        chapter_idx: 3,
        scroll_pos: 450,
        percent: 42.5,
        overall: 15.2,
    };
    repo.save(&p).await.expect("save");

    let got = repo.get("book-alpha.epub").await.expect("get");
    assert_eq!(got.epub_path, "book-alpha.epub");
    assert_eq!(got.chapter_idx, 3);
    assert_eq!(got.scroll_pos, 450);
    assert!((got.percent - 42.5).abs() < 0.01);
    assert!((got.overall - 15.2).abs() < 0.01);
}

#[tokio::test]
async fn second_save_with_same_epub_path_upserts_row_count_stays_1() {
    let (_dir, pool) = fresh_pool().await;
    let repo = ProgressRepository::new(pool.clone());

    let p1 = ReadingProgress {
        epub_path: "book-beta.epub".into(),
        chapter_idx: 0,
        scroll_pos: 0,
        percent: 10.0,
        overall: 5.0,
    };
    repo.save(&p1).await.expect("save 1");

    let p2 = ReadingProgress {
        epub_path: "book-beta.epub".into(),
        chapter_idx: 5,
        scroll_pos: 100,
        percent: 80.0,
        overall: 40.0,
    };
    repo.save(&p2).await.expect("save 2 (upsert)");

    // Row count must still be 1.
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM reading_progress")
        .fetch_one(&pool)
        .await
        .expect("count");
    assert_eq!(count, 1, "upsert must not insert a duplicate row");

    let got = repo.get("book-beta.epub").await.expect("get");
    assert_eq!(got.chapter_idx, 5, "chapter_idx must be updated");
    assert!((got.percent - 80.0).abs() < 0.01, "percent must be updated");
}

#[tokio::test]
async fn percent_100_does_not_panic() {
    let (_dir, pool) = fresh_pool().await;
    let repo = ProgressRepository::new(pool);

    let p = ReadingProgress {
        epub_path: "complete.epub".into(),
        chapter_idx: 10,
        scroll_pos: 9999,
        percent: 100.0,
        overall: 100.0,
    };
    repo.save(&p).await.expect("save 100%");
    let got = repo.get("complete.epub").await.expect("get");
    assert!((got.percent - 100.0).abs() < 0.01);
}

#[tokio::test]
async fn chapter_idx_i32_max_edge_does_not_panic() {
    let (_dir, pool) = fresh_pool().await;
    let repo = ProgressRepository::new(pool);

    let p = ReadingProgress {
        epub_path: "edge.epub".into(),
        chapter_idx: i32::MAX,
        scroll_pos: 0,
        percent: 0.0,
        overall: 0.0,
    };
    repo.save(&p).await.expect("save i32::MAX");
    let got = repo.get("edge.epub").await.expect("get");
    assert_eq!(got.chapter_idx, i32::MAX);
}

#[tokio::test]
async fn get_nonexistent_returns_not_found_err() {
    let (_dir, pool) = fresh_pool().await;
    let repo = ProgressRepository::new(pool);
    let result = repo.get("does-not-exist.epub").await;
    assert!(result.is_err());
    match result.unwrap_err() {
        progress_db::DbError::NotFound(s) => assert_eq!(s, "does-not-exist.epub"),
        other => panic!("expected NotFound, got {other:?}"),
    }
}
