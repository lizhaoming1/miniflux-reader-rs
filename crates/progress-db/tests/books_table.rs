//! PR#4 Task 4.2 — books_table: insert + list ordered tests.

use progress_db::{run_migrations, Book, BookRepository};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use std::str::FromStr;
use tempfile::TempDir;

async fn fresh_pool() -> (TempDir, SqlitePool) {
    let dir = TempDir::new().expect("tempdir");
    let db_path = dir.path().join("rust-books-test.db");
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
async fn insert_book_then_list_returns_it() {
    let (_dir, pool) = fresh_pool().await;
    let repo = BookRepository::new(pool);

    let book = Book {
        safe_name: "test-book.epub".into(),
        title: "Test Book".into(),
        author: "Author".into(),
        total_chapters: 5,
        file_size: 12345,
    };
    repo.upsert(&book).await.expect("upsert");

    let list = repo.list().await.expect("list");
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].safe_name, "test-book.epub");
    assert_eq!(list[0].title, "Test Book");
    assert_eq!(list[0].author, "Author");
    assert_eq!(list[0].total_chapters, 5);
    assert_eq!(list[0].file_size, 12345);
}

#[tokio::test]
async fn list_multiple_books_ordered_created_at_desc() {
    let (_dir, pool) = fresh_pool().await;
    let repo = BookRepository::new(pool.clone());

    // Insert two books with a small delay so created_at differs.
    let b1 = Book {
        safe_name: "first.epub".into(),
        title: "First".into(),
        author: "A".into(),
        total_chapters: 1,
        file_size: 100,
    };
    repo.upsert(&b1).await.expect("upsert 1");

    // SQLite datetime('now') has second resolution; sleep 1.1s to ensure
    // the second book gets a strictly later timestamp.
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    let b2 = Book {
        safe_name: "second.epub".into(),
        title: "Second".into(),
        author: "B".into(),
        total_chapters: 2,
        file_size: 200,
    };
    repo.upsert(&b2).await.expect("upsert 2");

    let list = repo.list().await.expect("list");
    assert_eq!(list.len(), 2, "should have 2 books");
    // DESC order → second (newer) first.
    assert_eq!(list[0].safe_name, "second.epub");
    assert_eq!(list[1].safe_name, "first.epub");
}

#[tokio::test]
async fn upsert_same_safe_name_updates_does_not_duplicate() {
    let (_dir, pool) = fresh_pool().await;
    let repo = BookRepository::new(pool.clone());

    let b1 = Book {
        safe_name: "upsert.epub".into(),
        title: "Original".into(),
        author: "X".into(),
        total_chapters: 3,
        file_size: 500,
    };
    repo.upsert(&b1).await.expect("upsert 1");

    let b2 = Book {
        safe_name: "upsert.epub".into(),
        title: "Updated Title".into(),
        author: "Y".into(),
        total_chapters: 7,
        file_size: 999,
    };
    repo.upsert(&b2).await.expect("upsert 2 (update)");

    let list = repo.list().await.expect("list");
    assert_eq!(list.len(), 1, "upsert must not duplicate");
    assert_eq!(list[0].title, "Updated Title");
    assert_eq!(list[0].total_chapters, 7);
}
