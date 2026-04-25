//! Embedded UI assets — compiled only when `embed-ui` feature is on.

use axum::body::Body;
use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use rust_embed::Embed;

use crate::AppState;

#[derive(Embed)]
#[folder = "../../ui/dist/"]
struct Assets;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(serve_index))
        .route("/*path", get(serve_asset))
}

async fn serve_index() -> impl IntoResponse {
    serve_file("index.html")
}

async fn serve_asset(Path(path): Path<String>) -> impl IntoResponse {
    if path.starts_with("api/") {
        return (StatusCode::NOT_FOUND, "Not Found").into_response();
    }
    if Assets::get(&path).is_some() {
        serve_file(&path)
    } else {
        // SPA fallback: any unknown frontend route → index.html
        serve_file("index.html")
    }
}

fn serve_file(path: &str) -> axum::response::Response {
    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            let mut response =
                (StatusCode::OK, Body::from(content.data.into_owned())).into_response();
            response.headers_mut().insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime.as_ref())
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            );
            response
        }
        None => (StatusCode::NOT_FOUND, "Not Found").into_response(),
    }
}
