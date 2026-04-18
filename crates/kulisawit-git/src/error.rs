use thiserror::Error;

#[derive(Debug, Error)]
pub enum GitError {
    #[error("git command {command} failed with status {status}: {stderr}")]
    Command {
        command: String,
        status: i32,
        stderr: String,
    },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("libgit2: {0}")]
    Libgit2(#[from] git2::Error),
    #[error("invalid input: {0}")]
    Invalid(String),
}

pub type GitResult<T> = Result<T, GitError>;
