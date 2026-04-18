#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_db::{columns, connect, migrate, project};

async fn setup() -> (kulisawit_db::DbPool, kulisawit_core::ProjectId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    (pool, id)
}

#[tokio::test]
async fn seed_defaults_creates_five_columns_in_order() {
    let (pool, project_id) = setup().await;
    columns::seed_defaults(&pool, &project_id)
        .await
        .expect("seed");
    let cols = columns::list_for_project(&pool, &project_id)
        .await
        .expect("list");
    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["Backlog", "Todo", "Doing", "Review", "Done"]);
    // Positions are dense starting at 0.
    let positions: Vec<i64> = cols.iter().map(|c| c.position).collect();
    assert_eq!(positions, vec![0, 1, 2, 3, 4]);
}
