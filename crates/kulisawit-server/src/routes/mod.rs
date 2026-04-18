//! Router composition.

pub mod attempts;
pub mod projects;
pub mod tasks;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    Router::new().merge(projects::routes()).with_state(state)
}
