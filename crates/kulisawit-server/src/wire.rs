//! JSON wire DTOs.
//!
//! All fields are `snake_case`. Responses are thin re-serializations of the
//! DB-layer structs; no additional projection logic lives here. Requests
//! mirror the `New*` struct shapes from `kulisawit-db`.

use serde::{Deserialize, Serialize};

use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, ColumnId, ProjectId, TaskId};

// ---- Requests ----

#[derive(Debug, Deserialize)]
pub struct NewProjectRequest {
    pub name: String,
    pub repo_path: String,
}

#[derive(Debug, Deserialize)]
pub struct NewTaskRequest {
    pub project_id: ProjectId,
    pub column_id: ColumnId,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub linked_files: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct DispatchRequest {
    pub agent: String,
    pub batch: usize,
    #[serde(default)]
    pub variants: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub column_id: Option<ColumnId>,
}

// ---- Responses ----

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: ProjectId,
    pub name: String,
    pub repo_path: String,
    pub created_at: i64,
    /// Filled with seeded column IDs only on the POST /api/projects response.
    /// Always an empty Vec on GET /api/projects (list) responses; clients that
    /// need column IDs after listing should call GET /api/projects/:id/board.
    /// Keep in sync with ui/src/types/api.ts.
    #[serde(default)]
    pub column_ids: Vec<ColumnId>,
}

#[derive(Debug, Serialize)]
pub struct TaskResponse {
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

#[derive(Debug, Serialize)]
pub struct AttemptResponse {
    pub id: AttemptId,
    pub task_id: TaskId,
    pub agent_id: String,
    pub status: AttemptStatus,
    pub prompt_variant: Option<String>,
    pub worktree_path: String,
    pub branch_name: String,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DispatchResponse {
    pub attempt_ids: Vec<AttemptId>,
}

#[derive(Debug, Serialize)]
pub struct EventEnvelope {
    pub attempt_id: AttemptId,
    pub event: AgentEvent,
    pub ts_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct BoardResponse {
    pub project: ProjectResponse,
    pub columns: Vec<BoardColumn>,
}

#[derive(Debug, Serialize)]
pub struct BoardColumn {
    pub id: ColumnId,
    pub name: String,
    pub position: i64,
    pub tasks: Vec<TaskResponse>,
}

// ---- Conversions from DB structs ----

impl From<kulisawit_db::project::Project> for ProjectResponse {
    fn from(p: kulisawit_db::project::Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            repo_path: p.repo_path,
            created_at: p.created_at,
            column_ids: vec![],
        }
    }
}

impl From<kulisawit_db::task::Task> for TaskResponse {
    fn from(t: kulisawit_db::task::Task) -> Self {
        Self {
            id: t.id,
            project_id: t.project_id,
            column_id: t.column_id,
            title: t.title,
            description: t.description,
            position: t.position,
            tags: t.tags,
            linked_files: t.linked_files,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }
    }
}

impl From<kulisawit_db::attempt::Attempt> for AttemptResponse {
    fn from(a: kulisawit_db::attempt::Attempt) -> Self {
        Self {
            id: a.id,
            task_id: a.task_id,
            agent_id: a.agent_id,
            status: a.status,
            prompt_variant: a.prompt_variant,
            worktree_path: a.worktree_path,
            branch_name: a.branch_name,
            started_at: a.started_at,
            completed_at: a.completed_at,
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_core::{AttemptId, AttemptStatus, ProjectId, TaskId};

    #[test]
    fn project_response_serializes_snake_case() {
        let r = ProjectResponse {
            id: ProjectId::new(),
            name: "Demo".into(),
            repo_path: "/tmp/demo".into(),
            created_at: 1_700_000_000_000,
            column_ids: vec![],
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(json.contains("\"repo_path\""));
        assert!(json.contains("\"created_at\":1700000000000"));
    }

    #[test]
    fn attempt_response_omits_verification_fields() {
        let r = AttemptResponse {
            id: AttemptId::new(),
            task_id: TaskId::new(),
            agent_id: "mock".into(),
            status: AttemptStatus::Queued,
            prompt_variant: None,
            worktree_path: "/tmp/wt".into(),
            branch_name: "b".into(),
            started_at: None,
            completed_at: None,
        };
        let json = serde_json::to_string(&r).expect("ser");
        assert!(
            !json.contains("verification"),
            "no verification fields: {json}"
        );
    }

    #[test]
    fn dispatch_request_accepts_no_variants() {
        let body = r#"{"agent":"mock","batch":3}"#;
        let r: DispatchRequest = serde_json::from_str(body).expect("de");
        assert_eq!(r.agent, "mock");
        assert_eq!(r.batch, 3);
        assert!(r.variants.is_none());
    }
}
