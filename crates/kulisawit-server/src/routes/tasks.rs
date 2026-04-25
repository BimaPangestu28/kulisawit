//! `/api/tasks` endpoints including `/api/tasks/:id/dispatch`.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use kulisawit_core::TaskId;
use kulisawit_db::{
    columns, project,
    task::{self, NewTask},
};
use kulisawit_orchestrator::dispatch_batch_spawned;

use crate::wire::{DispatchRequest, DispatchResponse, NewTaskRequest, TaskResponse, UpdateTaskRequest};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/tasks", post(create))
        .route("/api/tasks/:id", get(get_by_id).patch(update))
        .route("/api/tasks/:id/dispatch", post(dispatch))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewTaskRequest>,
) -> ServerResult<Json<TaskResponse>> {
    if project::get(state.orch.pool(), &req.project_id)
        .await?
        .is_none()
    {
        return Err(ServerError::InvalidInput(format!(
            "project not found: {}",
            req.project_id.as_str()
        )));
    }
    let cols = columns::list_for_project(state.orch.pool(), &req.project_id).await?;
    if !cols.iter().any(|c| c.id == req.column_id) {
        return Err(ServerError::InvalidInput(format!(
            "column not found in project: {}",
            req.column_id.as_str()
        )));
    }

    let id = task::create(
        state.orch.pool(),
        NewTask {
            project_id: req.project_id.clone(),
            column_id: req.column_id.clone(),
            title: req.title,
            description: req.description,
            tags: req.tags,
            linked_files: req.linked_files,
        },
    )
    .await?;
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("task vanished after insert".into()))?;
    Ok(Json(row.into()))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
) -> ServerResult<Json<TaskResponse>> {
    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}

async fn dispatch(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
    Json(req): Json<DispatchRequest>,
) -> ServerResult<Json<DispatchResponse>> {
    if task::get(state.orch.pool(), &id).await?.is_none() {
        return Err(ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        });
    }

    let orch = Arc::clone(&state.orch);
    let attempt_ids = dispatch_batch_spawned(&orch, &id, &req.agent, req.batch, req.variants)
        .await
        .map_err(|e| match e {
            kulisawit_orchestrator::OrchestratorError::Invalid(msg) => {
                ServerError::InvalidInput(msg)
            }
            other => ServerError::from(other),
        })?;

    Ok(Json(DispatchResponse { attempt_ids }))
}

async fn update(
    State(state): State<AppState>,
    Path(id): Path<TaskId>,
    Json(req): Json<UpdateTaskRequest>,
) -> ServerResult<Json<TaskResponse>> {
    if req.title.is_none() && req.description.is_none() && req.column_id.is_none() {
        return Err(ServerError::InvalidInput(
            "at least one of title, description, column_id is required".into(),
        ));
    }

    let current = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "task",
            id: id.as_str().to_owned(),
        })?;

    if req.title.is_some() || req.description.is_some() {
        let title = req.title.as_deref().unwrap_or(current.title.as_str());
        let description: Option<&str> = match &req.description {
            Some(d) => Some(d.as_str()),
            None => current.description.as_deref(),
        };
        task::update_text(state.orch.pool(), &id, title, description).await?;
    }

    if let Some(col_id) = req.column_id.as_ref() {
        let cols = columns::list_for_project(state.orch.pool(), &current.project_id).await?;
        if !cols.iter().any(|c| &c.id == col_id) {
            return Err(ServerError::InvalidInput(format!(
                "column not found in project: {}",
                col_id.as_str()
            )));
        }
        task::move_to_column(state.orch.pool(), &id, col_id).await?;
    }

    let row = task::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("task vanished after update".into()))?;
    Ok(Json(row.into()))
}
