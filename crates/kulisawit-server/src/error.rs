//! `ServerError` — placeholder; Task 3.1.2 replaces.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("internal: {0}")]
    Internal(String),
}

pub type ServerResult<T> = Result<T, ServerError>;
