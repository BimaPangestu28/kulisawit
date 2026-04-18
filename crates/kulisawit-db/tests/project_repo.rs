#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::ProjectId;
use kulisawit_db::{connect, migrate, project};

async fn fresh_pool() -> kulisawit_db::DbPool {
    let pool = connect("sqlite::memory:").await.expect("connect");
    migrate(&pool).await.expect("migrate");
    pool
}

#[tokio::test]
async fn create_then_get_returns_same_row() {
    let pool = fresh_pool().await;
    let record = project::NewProject {
        name: "Demo".into(),
        repo_path: "/tmp/demo".into(),
    };
    let id = project::create(&pool, record).await.expect("create");
    let fetched = project::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(fetched.name, "Demo");
    assert_eq!(fetched.repo_path, "/tmp/demo");
    assert_eq!(fetched.id, id);
}

#[tokio::test]
async fn list_returns_rows_ordered_by_created_at_desc() {
    let pool = fresh_pool().await;
    let a = project::create(
        &pool,
        project::NewProject {
            name: "A".into(),
            repo_path: "/a".into(),
        },
    )
    .await
    .expect("a");
    // Small sleep to guarantee distinct created_at timestamps (second resolution).
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let b = project::create(
        &pool,
        project::NewProject {
            name: "B".into(),
            repo_path: "/b".into(),
        },
    )
    .await
    .expect("b");
    let rows = project::list(&pool).await.expect("list");
    assert_eq!(rows.len(), 2);
    // b was created later → comes first.
    assert_eq!(rows[0].id, b);
    assert_eq!(rows[1].id, a);
}

#[tokio::test]
async fn get_missing_returns_none() {
    let pool = fresh_pool().await;
    let result = project::get(&pool, &ProjectId::new()).await.expect("ok");
    assert!(result.is_none());
}
