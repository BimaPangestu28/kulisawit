//! `/api/attempts` endpoints. SSE stream lands in Task 3.1.10.

use axum::extract::{Path, State};
use axum::routing::get;
use axum::{Json, Router};

use kulisawit_core::AttemptId;
use kulisawit_db::attempt;

use crate::wire::AttemptResponse;
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/attempts/:id", get(get_by_id))
}

async fn get_by_id(
    State(state): State<AppState>,
    Path(id): Path<AttemptId>,
) -> ServerResult<Json<AttemptResponse>> {
    let row = attempt::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "attempt",
            id: id.as_str().to_owned(),
        })?;
    Ok(Json(row.into()))
}
