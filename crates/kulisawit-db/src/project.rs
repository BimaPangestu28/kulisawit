//! Project repository functions.

use chrono::Utc;
use kulisawit_core::ProjectId;
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewProject {
    pub name: String,
    pub repo_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub repo_path: String,
    pub created_at: i64,
}

fn parse_id(raw: Option<String>) -> DbResult<ProjectId> {
    raw.map(ProjectId::from_string)
        .ok_or_else(|| DbError::Invalid("project.id is NULL".into()))
}

pub async fn create(pool: &DbPool, new: NewProject) -> DbResult<ProjectId> {
    let id = ProjectId::new();
    let created_at = Utc::now().timestamp();
    let id_str = id.as_str();
    sqlx::query!(
        "INSERT INTO project (id, name, repo_path, created_at) VALUES (?, ?, ?, ?)",
        id_str,
        new.name,
        new.repo_path,
        created_at
    )
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn get(pool: &DbPool, id: &ProjectId) -> DbResult<Option<Project>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, name, repo_path, created_at FROM project WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        Ok(Project {
            id: parse_id(r.id)?,
            name: r.name,
            repo_path: r.repo_path,
            created_at: r.created_at,
        })
    })
    .transpose()
}

pub async fn list(pool: &DbPool) -> DbResult<Vec<Project>> {
    let rows = sqlx::query!(
        "SELECT id, name, repo_path, created_at FROM project ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            Ok(Project {
                id: parse_id(r.id)?,
                name: r.name,
                repo_path: r.repo_path,
                created_at: r.created_at,
            })
        })
        .collect()
}
