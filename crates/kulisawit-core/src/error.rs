//! Core-level errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("adapter error: {0}")]
    Adapter(String),

    #[error("invalid configuration: {0}")]
    Config(String),

    #[error("invariant violated: {0}")]
    Invariant(&'static str),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serialization error: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("unknown buah status: {0}")]
    UnknownBuahStatus(#[from] crate::status::UnknownBuahStatus),
}

pub type CoreResult<T> = Result<T, CoreError>;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn error_display_contains_reason() {
        let err = CoreError::Config("missing default_kuli".into());
        assert!(format!("{err}").contains("missing default_kuli"));
    }

    #[test]
    fn io_error_converts_via_from() {
        let io: std::io::Error = std::io::Error::new(std::io::ErrorKind::Other, "boom");
        let err: CoreError = io.into();
        assert!(matches!(err, CoreError::Io(_)));
    }
}
