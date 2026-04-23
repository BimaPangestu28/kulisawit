//! Router composition.

pub mod attempts;
pub mod board;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(projects::routes())
        .merge(tasks::routes())
        .merge(attempts::routes())
        .merge(board::routes())
        .with_state(state)
}
