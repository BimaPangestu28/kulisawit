//! Task (kanban card) repository.

use chrono::Utc;
use kulisawit_core::{ColumnId, ProjectId, TaskId};
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewTask {
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    pub description: Option<String>,
    pub position: i64,
    pub tags: Vec<String>,
    pub linked_files: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

async fn next_position(pool: &DbPool, column_id: &ColumnId) -> DbResult<i64> {
    let col_str = column_id.as_str();
    let row = sqlx::query!(
        "SELECT COALESCE(MAX(position), -1) + 1 AS next FROM task WHERE column_id = ?",
        col_str
    )
    .fetch_one(pool)
    .await?;
    Ok(row.next)
}

pub async fn create(pool: &DbPool, new: NewTask) -> DbResult<TaskId> {
    let id = TaskId::new();
    let now = Utc::now().timestamp_millis();
    let position = next_position(pool, &new.column_id).await?;
    let tags_json = serde_json::to_string(&new.tags)?;
    let files_json = serde_json::to_string(&new.linked_files)?;
    let id_str = id.as_str();
    let project_str = new.project_id.as_str();
    let col_str = new.column_id.as_str();
    sqlx::query!(
        "INSERT INTO task (id, project_id, column_id, title, description, position, tags, linked_files, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        id_str,
        project_str,
        col_str,
        new.title,
        new.description,
        position,
        tags_json,
        files_json,
        now,
        now
    )
    .execute(pool)
    .await?;
    Ok(id)
}

fn parse_string_list(raw: Option<&str>) -> DbResult<Vec<String>> {
    Ok(match raw {
        None => vec![],
        Some(s) => serde_json::from_str(s).map_err(DbError::from)?,
    })
}

pub async fn get(pool: &DbPool, id: &TaskId) -> DbResult<Option<Task>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, project_id, column_id, title, description, position, tags, linked_files, created_at, updated_at
         FROM task WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        // SQLx infers `id` as Option<String> (rowid alias); `project_id` and `column_id`
        // are plain String — NOT NULL in schema and confirmed non-nullable in .sqlx metadata.
        let id =
            r.id.ok_or_else(|| DbError::Invalid("task.id null".into()))?;
        Ok::<_, DbError>(Task {
            id: TaskId::from_string(id),
            project_id: ProjectId::from_string(r.project_id),
            column_id: ColumnId::from_string(r.column_id),
            title: r.title,
            description: r.description,
            position: r.position,
            tags: parse_string_list(r.tags.as_deref())?,
            linked_files: parse_string_list(r.linked_files.as_deref())?,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    })
    .transpose()
}

pub async fn list_for_column(pool: &DbPool, column_id: &ColumnId) -> DbResult<Vec<Task>> {
    let col_str = column_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, project_id, column_id, title, description, position, tags, linked_files, created_at, updated_at
         FROM task WHERE column_id = ? ORDER BY position ASC",
        col_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            // `id` is Option<String> (rowid alias); `project_id` and `column_id` are plain
            // String — NOT NULL in schema and confirmed non-nullable in .sqlx metadata.
            let id =
                r.id.ok_or_else(|| DbError::Invalid("task.id null".into()))?;
            Ok::<_, DbError>(Task {
                id: TaskId::from_string(id),
                project_id: ProjectId::from_string(r.project_id),
                column_id: ColumnId::from_string(r.column_id),
                title: r.title,
                description: r.description,
                position: r.position,
                tags: parse_string_list(r.tags.as_deref())?,
                linked_files: parse_string_list(r.linked_files.as_deref())?,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect()
}

pub async fn list_for_project(pool: &DbPool, project_id: &ProjectId) -> DbResult<Vec<Task>> {
    let project_str = project_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, project_id, column_id, title, description, position, tags, linked_files, created_at, updated_at
         FROM task WHERE project_id = ? ORDER BY column_id, position ASC",
        project_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            let id =
                r.id.ok_or_else(|| DbError::Invalid("task.id null".into()))?;
            Ok::<_, DbError>(Task {
                id: TaskId::from_string(id),
                project_id: ProjectId::from_string(r.project_id),
                column_id: ColumnId::from_string(r.column_id),
                title: r.title,
                description: r.description,
                position: r.position,
                tags: parse_string_list(r.tags.as_deref())?,
                linked_files: parse_string_list(r.linked_files.as_deref())?,
                created_at: r.created_at,
                updated_at: r.updated_at,
            })
        })
        .collect()
}

pub async fn update_text(
    pool: &DbPool,
    id: &TaskId,
    title: &str,
    description: Option<&str>,
) -> DbResult<()> {
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE task SET title = ?, description = ?, updated_at = ? WHERE id = ?",
        title,
        description,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_metadata(
    pool: &DbPool,
    id: &TaskId,
    tags: &[String],
    linked_files: &[String],
) -> DbResult<()> {
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    let tags_json = serde_json::to_string(tags)?;
    let files_json = serde_json::to_string(linked_files)?;
    sqlx::query!(
        "UPDATE task SET tags = ?, linked_files = ?, updated_at = ? WHERE id = ?",
        tags_json,
        files_json,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn move_to_column(pool: &DbPool, id: &TaskId, column_id: &ColumnId) -> DbResult<()> {
    let position = next_position(pool, column_id).await?;
    let now = Utc::now().timestamp_millis();
    let id_str = id.as_str();
    let col_str = column_id.as_str();
    sqlx::query!(
        "UPDATE task SET column_id = ?, position = ?, updated_at = ? WHERE id = ?",
        col_str,
        position,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
