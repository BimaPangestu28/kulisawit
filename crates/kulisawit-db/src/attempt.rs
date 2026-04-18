//! Attempt (single agent run) repository.

use chrono::Utc;
use kulisawit_core::{AttemptId, AttemptStatus, TaskId, VerificationStatus};
use serde::{Deserialize, Serialize};

use crate::{DbError, DbPool, DbResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewAttempt {
    pub task_id: TaskId,
    pub agent_id: String,
    pub prompt_variant: Option<String>,
    pub worktree_path: String,
    pub branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub id: AttemptId,
    pub task_id: TaskId,
    pub agent_id: String,
    pub prompt_variant: Option<String>,
    pub worktree_path: String,
    pub branch_name: String,
    pub status: AttemptStatus,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
    pub verification_status: Option<VerificationStatus>,
    pub verification_output: Option<String>,
}

#[allow(clippy::too_many_arguments)]
fn row_to_attempt(
    id: Option<String>,
    task_id: String,
    agent_id: String,
    prompt_variant: Option<String>,
    worktree_path: String,
    branch_name: String,
    status: String,
    started_at: Option<i64>,
    completed_at: Option<i64>,
    verification_status: Option<String>,
    verification_output: Option<String>,
) -> DbResult<Attempt> {
    let id = id.ok_or_else(|| DbError::Invalid("attempt.id null".into()))?;
    let status =
        AttemptStatus::try_from(status.as_str()).map_err(|e| DbError::Invalid(e.to_string()))?;
    let verification_status = verification_status
        .as_deref()
        .map(parse_verification_status)
        .transpose()?;
    Ok(Attempt {
        id: AttemptId::from_string(id),
        task_id: TaskId::from_string(task_id),
        agent_id,
        prompt_variant,
        worktree_path,
        branch_name,
        status,
        started_at,
        completed_at,
        verification_status,
        verification_output,
    })
}

fn parse_verification_status(s: &str) -> DbResult<VerificationStatus> {
    Ok(match s {
        "pending" => VerificationStatus::Pending,
        "passed" => VerificationStatus::Passed,
        "failed" => VerificationStatus::Failed,
        "skipped" => VerificationStatus::Skipped,
        other => return Err(DbError::Invalid(format!("verification_status={other}"))),
    })
}

pub async fn create(pool: &DbPool, new: NewAttempt) -> DbResult<AttemptId> {
    let id = AttemptId::new();
    let id_str = id.as_str();
    let task_str = new.task_id.as_str();
    let status = AttemptStatus::Queued.as_str();
    sqlx::query!(
        "INSERT INTO attempt (id, task_id, agent_id, prompt_variant, worktree_path, branch_name, status)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
        id_str,
        task_str,
        new.agent_id,
        new.prompt_variant,
        new.worktree_path,
        new.branch_name,
        status
    )
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn get(pool: &DbPool, id: &AttemptId) -> DbResult<Option<Attempt>> {
    let id_str = id.as_str();
    let row = sqlx::query!(
        "SELECT id, task_id, agent_id, prompt_variant, worktree_path, branch_name, status,
                started_at, completed_at, verification_status, verification_output
         FROM attempt WHERE id = ?",
        id_str
    )
    .fetch_optional(pool)
    .await?;
    row.map(|r| {
        row_to_attempt(
            r.id,
            r.task_id,
            r.agent_id,
            r.prompt_variant,
            r.worktree_path,
            r.branch_name,
            r.status,
            r.started_at,
            r.completed_at,
            r.verification_status,
            r.verification_output,
        )
    })
    .transpose()
}

pub async fn list_for_task(pool: &DbPool, task_id: &TaskId) -> DbResult<Vec<Attempt>> {
    let l = task_id.as_str();
    let rows = sqlx::query!(
        "SELECT id, task_id, agent_id, prompt_variant, worktree_path, branch_name, status,
                started_at, completed_at, verification_status, verification_output
         FROM attempt WHERE task_id = ? ORDER BY id ASC",
        l
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| {
            row_to_attempt(
                r.id,
                r.task_id,
                r.agent_id,
                r.prompt_variant,
                r.worktree_path,
                r.branch_name,
                r.status,
                r.started_at,
                r.completed_at,
                r.verification_status,
                r.verification_output,
            )
        })
        .collect()
}

pub async fn mark_running(pool: &DbPool, id: &AttemptId) -> DbResult<()> {
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    let status = AttemptStatus::Running.as_str();
    sqlx::query!(
        "UPDATE attempt SET status = ?, started_at = ? WHERE id = ?",
        status,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_terminal(pool: &DbPool, id: &AttemptId, status: AttemptStatus) -> DbResult<()> {
    if !status.is_terminal() {
        return Err(DbError::Invalid(format!(
            "mark_terminal called with non-terminal status: {status:?}"
        )));
    }
    let now = Utc::now().timestamp();
    let id_str = id.as_str();
    let status_str = status.as_str();
    sqlx::query!(
        "UPDATE attempt SET status = ?, completed_at = ? WHERE id = ?",
        status_str,
        now,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_verification(
    pool: &DbPool,
    id: &AttemptId,
    status: VerificationStatus,
    output: Option<&str>,
) -> DbResult<()> {
    let status_str = match status {
        VerificationStatus::Pending => "pending",
        VerificationStatus::Passed => "passed",
        VerificationStatus::Failed => "failed",
        VerificationStatus::Skipped => "skipped",
    };
    let id_str = id.as_str();
    sqlx::query!(
        "UPDATE attempt SET verification_status = ?, verification_output = ? WHERE id = ?",
        status_str,
        output,
        id_str
    )
    .execute(pool)
    .await?;
    Ok(())
}
