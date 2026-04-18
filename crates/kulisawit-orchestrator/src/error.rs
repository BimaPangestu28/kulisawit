//! Orchestrator-level error type.

use thiserror::Error;

use kulisawit_core::{AgentError, CoreError};
use kulisawit_db::DbError;
use kulisawit_git::GitError;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("core: {0}")]
    Core(#[from] CoreError),

    #[error("db: {0}")]
    Db(#[from] DbError),

    #[error("git: {0}")]
    Git(#[from] GitError),

    #[error("agent: {0}")]
    Agent(#[from] AgentError),

    #[error("invalid: {0}")]
    Invalid(String),

    #[error("cancelled")]
    Cancelled,
}

pub type OrchestratorResult<T> = Result<T, OrchestratorError>;
