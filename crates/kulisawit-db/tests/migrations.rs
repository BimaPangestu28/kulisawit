#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_db::pool::{connect, migrate};

#[tokio::test]
async fn migrations_apply_cleanly_to_memory_db() {
    let pool = connect("sqlite::memory:").await.expect("connect");
    migrate(&pool).await.expect("migrate");

    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .fetch_all(&pool)
            .await
            .expect("query");

    let names: Vec<&str> = rows.iter().map(|(n,)| n.as_str()).collect();
    assert!(names.contains(&"project"));
    assert!(names.contains(&"columns"));
    assert!(names.contains(&"task"));
    assert!(names.contains(&"attempt"));
    assert!(names.contains(&"events"));
}
