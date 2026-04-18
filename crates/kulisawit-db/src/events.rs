//! Per-attempt event log repository.

use chrono::Utc;
use kulisawit_core::{adapter::AgentEvent, AttemptId};

use crate::{DbPool, DbResult};

pub async fn append(pool: &DbPool, attempt_id: &AttemptId, event: &AgentEvent) -> DbResult<i64> {
    let ts = Utc::now().timestamp_millis();
    let payload = serde_json::to_string(event)?;
    let type_name = match event {
        AgentEvent::Stdout { .. } => "stdout",
        AgentEvent::Stderr { .. } => "stderr",
        AgentEvent::ToolCall { .. } => "tool_call",
        AgentEvent::ToolResult { .. } => "tool_result",
        AgentEvent::FileEdit { .. } => "file_edit",
        AgentEvent::Status { .. } => "status",
    };
    let attempt_str = attempt_id.as_str();
    let row = sqlx::query!(
        "INSERT INTO events (attempt_id, timestamp, type, payload) VALUES (?, ?, ?, ?) RETURNING id",
        attempt_str,
        ts,
        type_name,
        payload
    )
    .fetch_one(pool)
    .await?;
    row.id
        .ok_or_else(|| crate::DbError::Invalid("events.id is NULL from RETURNING".into()))
}

pub async fn list_for_attempt(pool: &DbPool, attempt_id: &AttemptId) -> DbResult<Vec<AgentEvent>> {
    let attempt_str = attempt_id.as_str();
    let rows = sqlx::query!(
        "SELECT payload FROM events WHERE attempt_id = ? ORDER BY id ASC",
        attempt_str
    )
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|r| serde_json::from_str::<AgentEvent>(&r.payload).map_err(Into::into))
        .collect()
}
