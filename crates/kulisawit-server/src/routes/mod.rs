//! Router composition.
//!
//! Subsequent tasks hang endpoint groups off the root router. For Task 3.1.3
//! the router exists only so `axum::serve` has something to serve.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new().with_state(state)
}
