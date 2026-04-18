//! Server-level error type and HTTP mapping.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use thiserror::Error;

use kulisawit_db::DbError;
use kulisawit_orchestrator::OrchestratorError;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("not found: {entity} {id}")]
    NotFound { entity: &'static str, id: String },

    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;

impl From<DbError> for ServerError {
    fn from(e: DbError) -> Self {
        Self::Internal(format!("db: {e}"))
    }
}

impl From<OrchestratorError> for ServerError {
    fn from(e: OrchestratorError) -> Self {
        Self::Internal(format!("orchestrator: {e}"))
    }
}

impl From<std::io::Error> for ServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Internal(format!("io: {e}"))
    }
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        match self {
            ServerError::NotFound { entity, id } => (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "not_found",
                    "entity": entity,
                    "id": id,
                })),
            )
                .into_response(),
            ServerError::InvalidInput(message) => (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "error": "invalid_input",
                    "message": message,
                })),
            )
                .into_response(),
            ServerError::Conflict(message) => (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "conflict",
                    "message": message,
                })),
            )
                .into_response(),
            ServerError::Internal(detail) => {
                tracing::error!(detail, "server internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal" })),
                )
                    .into_response()
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;
    use http_body_util::BodyExt;

    async fn body_str(resp: axum::response::Response) -> String {
        let bytes = resp.into_body().collect().await.expect("body").to_bytes();
        String::from_utf8(bytes.to_vec()).expect("utf8")
    }

    #[tokio::test]
    async fn not_found_maps_to_404_with_json_body() {
        let err = ServerError::NotFound {
            entity: "task",
            id: "abc".into(),
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::NOT_FOUND);
        let body = body_str(resp).await;
        assert!(body.contains("\"not_found\""));
        assert!(body.contains("\"task\""));
        assert!(body.contains("\"abc\""));
    }

    #[tokio::test]
    async fn invalid_input_maps_to_400() {
        let err = ServerError::InvalidInput("bad batch".into());
        assert_eq!(
            err.into_response().status(),
            axum::http::StatusCode::BAD_REQUEST
        );
    }

    #[tokio::test]
    async fn internal_maps_to_500_without_leaking_detail() {
        let err = ServerError::Internal("secret db url leak".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        let body = body_str(resp).await;
        assert!(
            !body.contains("secret"),
            "500 body must not leak detail: {body}"
        );
        assert!(body.contains("\"internal\""));
    }
}
