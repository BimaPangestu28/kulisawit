#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{ColumnId, ProjectId};
use kulisawit_db::{columns, connect, migrate, project, task};

async fn setup() -> (kulisawit_db::DbPool, ProjectId, ColumnId) {
    let pool = connect("sqlite::memory:").await.expect("pool");
    migrate(&pool).await.expect("mig");
    let project_id = project::create(
        &pool,
        project::NewProject {
            name: "K".into(),
            repo_path: "/k".into(),
        },
    )
    .await
    .expect("project");
    let col_ids = columns::seed_defaults(&pool, &project_id)
        .await
        .expect("seed");
    (pool, project_id, col_ids[0].clone())
}

#[tokio::test]
async fn create_task_and_fetch_by_id() {
    let (pool, project_id, col_id) = setup().await;
    let id = task::create(
        &pool,
        task::NewTask {
            project_id: project_id.clone(),
            column_id: col_id.clone(),
            title: "add rate limit to /login".into(),
            description: Some("describe.".into()),
            tags: vec!["auth".into()],
            linked_files: vec!["src/auth.rs".into()],
        },
    )
    .await
    .expect("create");
    let l = task::get(&pool, &id).await.expect("get").expect("row");
    assert_eq!(l.title, "add rate limit to /login");
    assert_eq!(l.tags, vec!["auth".to_string()]);
    assert_eq!(l.linked_files, vec!["src/auth.rs".to_string()]);
    assert_eq!(l.column_id, col_id);
}

#[tokio::test]
async fn list_for_column_returns_in_position_order() {
    let (pool, project_id, col_id) = setup().await;
    for title in ["first", "second", "third"] {
        task::create(
            &pool,
            task::NewTask {
                project_id: project_id.clone(),
                column_id: col_id.clone(),
                title: title.into(),
                description: None,
                tags: vec![],
                linked_files: vec![],
            },
        )
        .await
        .expect("create");
    }
    let rows = task::list_for_column(&pool, &col_id).await.expect("list");
    let titles: Vec<&str> = rows.iter().map(|l| l.title.as_str()).collect();
    assert_eq!(titles, vec!["first", "second", "third"]);
}

#[tokio::test]
async fn update_title_and_description() {
    let (pool, project_id, col_id) = setup().await;
    let id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: col_id,
            title: "old".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("create");
    task::update_text(&pool, &id, "new", Some("fresh desc"))
        .await
        .expect("update");
    let l = task::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(l.title, "new");
    assert_eq!(l.description.as_deref(), Some("fresh desc"));
}

#[tokio::test]
async fn move_task_to_other_column_updates_column_id_and_bumps_position() {
    let (pool, project_id, col_id) = setup().await;
    let cols = columns::list_for_project(&pool, &project_id)
        .await
        .expect("cols");
    let target = &cols[2]; // "Doing"
    let id = task::create(
        &pool,
        task::NewTask {
            project_id,
            column_id: col_id,
            title: "x".into(),
            description: None,
            tags: vec![],
            linked_files: vec![],
        },
    )
    .await
    .expect("create");
    task::move_to_column(&pool, &id, &target.id)
        .await
        .expect("move");
    let l = task::get(&pool, &id).await.expect("ok").expect("row");
    assert_eq!(l.column_id, target.id);
}
