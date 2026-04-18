//! `/api/projects` endpoints.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use kulisawit_core::ProjectId;
use kulisawit_db::project::{self, NewProject};

use crate::wire::{NewProjectRequest, ProjectResponse};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/projects", post(create))
        .route("/api/projects/:id", get(get_by_id))
}

async fn create(
    State(state): State<AppState>,
    Json(req): Json<NewProjectRequest>,
) -> ServerResult<Json<ProjectResponse>> {
    let id = project::create(
        state.orch.pool(),
        NewProject {
            name: req.name.clone(),
            repo_path: req.repo_path.clone(),
        },
    )
    .await?;
    let row = project::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::Internal("project vanished after insert".into()))?;
    Ok(Json(row.into()))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<ProjectId>,
) -> ServerResult<Json<ProjectResponse>> {
    let row = project::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "project",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}
