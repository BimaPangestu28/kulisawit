//! `/api/projects/:id/board` endpoint — full kanban shape in one fetch.

use std::collections::HashMap;

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use kulisawit_core::{ColumnId, ProjectId};
use kulisawit_db::{columns, project, task};

use crate::wire::{BoardColumn, BoardResponse, ProjectResponse, TaskResponse};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/projects/:id/board", get(get_board))
}

async fn get_board(
    State(state): State<AppState>,
    Path(id): Path<ProjectId>,
) -> ServerResult<Json<BoardResponse>> {
    let pool = state.orch.pool();
    let project_row = project::get(pool, &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "project",
            id: id.as_str().to_owned(),
        })?;

    let column_rows = columns::list_for_project(pool, &id).await?;
    let task_rows = task::list_for_project(pool, &id).await?;

    // Group tasks by column_id. Tasks are already sorted by (column_id, position)
    // from list_for_project, so per-column order is correct.
    let mut tasks_by_column: HashMap<ColumnId, Vec<TaskResponse>> = HashMap::new();
    for t in task_rows {
        tasks_by_column
            .entry(t.column_id.clone())
            .or_default()
            .push(t.into());
    }

    let columns_out: Vec<BoardColumn> = column_rows
        .into_iter()
        .map(|c| BoardColumn {
            tasks: tasks_by_column.remove(&c.id).unwrap_or_default(),
            id: c.id,
            name: c.name,
            position: c.position,
        })
        .collect();

    let mut project_resp: ProjectResponse = project_row.into();
    // column_ids stays empty Vec on board response per the listing contract.
    project_resp.column_ids = vec![];

    Ok(Json(BoardResponse {
        project: project_resp,
        columns: columns_out,
    }))
}
