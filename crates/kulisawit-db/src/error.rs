use thiserror::Error;

#[derive(Debug, Error)]
pub enum DbError {
    #[error("sqlx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("migration: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
    #[error("invalid row: {0}")]
    Invalid(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type DbResult<T> = Result<T, DbError>;
