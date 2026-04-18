//! Kanban column repository.

use kulisawit_core::{ColumnId, ProjectId};
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

pub const DEFAULT_COLUMN_NAMES: [&str; 5] = ["Backlog", "Todo", "Doing", "Review", "Done"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub id: ColumnId,
    pub project_id: ProjectId,
    pub name: String,
    pub position: i64,
}

fn parse_column_id(raw: Option<String>) -> DbResult<ColumnId> {
    raw.map(ColumnId::from_string)
        .ok_or_else(|| DbError::Invalid("columns.id is NULL".into()))
}

pub async fn seed_defaults(pool: &DbPool, project_id: &ProjectId) -> DbResult<Vec<ColumnId>> {
    let mut ids = Vec::with_capacity(DEFAULT_COLUMN_NAMES.len());
    for (idx, name) in DEFAULT_COLUMN_NAMES.iter().enumerate() {
        let id = ColumnId::new();
        let id_str = id.as_str();
        let project_str = project_id.as_str();
        let pos = idx as i64;
        sqlx::query!(
            "INSERT INTO columns (id, project_id, name, position) VALUES (?, ?, ?, ?)",
            id_str,
            project_str,
            name,
            pos
        )
        .execute(pool)
        .await?;
        ids.push(id);
    }
    Ok(ids)
}

pub async fn list_for_project(pool: &DbPool, project_id: &ProjectId) -> DbResult<Vec<Column>> {
    let project_str = project_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, project_id, name, position FROM columns WHERE project_id = ? ORDER BY position ASC",
        project_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Column {
                id: parse_column_id(r.id)?,
                project_id: ProjectId::from_string(r.project_id),
                name: r.name,
                position: r.position,
            })
        })
        .collect()
}
