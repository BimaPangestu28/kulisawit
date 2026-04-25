//! Router composition.

pub mod attempts;
pub mod board;
pub mod projects;
pub mod tasks;

#[cfg(feature = "embed-ui")]
use crate::assets;

use axum::Router;

use crate::AppState;

pub fn router(state: AppState) -> Router {
    let r = Router::new()
        .merge(projects::routes())
        .merge(tasks::routes())
        .merge(attempts::routes())
        .merge(board::routes());

    #[cfg(feature = "embed-ui")]
    let r = r.merge(assets::routes());

    r.with_state(state)
}
