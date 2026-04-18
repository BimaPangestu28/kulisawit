//! `/api/attempts` endpoints including SSE.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::{Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use futures::{stream, Stream, StreamExt};

use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, RunStatus};
use kulisawit_db::attempt;

use crate::wire::{AttemptResponse, EventEnvelope};
use crate::{AppState, ServerError, ServerResult};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/attempts/:id", get(get_by_id))
        .route("/api/attempts/:id/events", get(events))
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

async fn events(
    State(state): State<AppState>,
    Path(id): Path<AttemptId>,
) -> Result<axum::response::Response, ServerError> {
    let row = attempt::get(state.orch.pool(), &id)
        .await?
        .ok_or_else(|| ServerError::NotFound {
            entity: "attempt",
            id: id.as_str().to_owned(),
        })?;

    let id_owned = id.clone();
    let stream: std::pin::Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> =
        if is_terminal(row.status) {
            let run_status = attempt_to_run_status(row.status);
            let evt = AgentEvent::Status {
                status: run_status,
                detail: None,
            };
            let envelope = EventEnvelope {
                attempt_id: id_owned,
                event: evt,
                ts_ms: Utc::now().timestamp_millis(),
            };
            let data = serde_json::to_string(&envelope).unwrap_or_default();
            let event = Event::default().data(data);
            Box::pin(stream::iter(vec![Ok(event)]))
        } else {
            let rx = state.orch.broadcaster().subscribe(&id_owned);
            let id_for_map = id_owned.clone();
            Box::pin(
                tokio_stream::wrappers::BroadcastStream::new(rx).filter_map(move |res| {
                    let id = id_for_map.clone();
                    async move {
                        match res {
                            Ok(evt) => {
                                let envelope = EventEnvelope {
                                    attempt_id: id,
                                    event: evt,
                                    ts_ms: Utc::now().timestamp_millis(),
                                };
                                let json = serde_json::to_string(&envelope).ok()?;
                                Some(Ok(Event::default().data(json)))
                            }
                            Err(
                                tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(_),
                            ) => None,
                        }
                    }
                }),
            )
        };

    let sse = Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keepalive"),
    );
    Ok(sse.into_response())
}

fn is_terminal(s: AttemptStatus) -> bool {
    matches!(
        s,
        AttemptStatus::Completed | AttemptStatus::Failed | AttemptStatus::Cancelled
    )
}

fn attempt_to_run_status(s: AttemptStatus) -> RunStatus {
    match s {
        AttemptStatus::Completed => RunStatus::Succeeded,
        AttemptStatus::Failed => RunStatus::Failed,
        AttemptStatus::Cancelled => RunStatus::Cancelled,
        _ => RunStatus::Failed,
    }
}
