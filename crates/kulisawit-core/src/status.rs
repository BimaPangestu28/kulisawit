//! Status enums for runs, buah, and sortir.

use serde::{Deserialize, Serialize};

/// Lifecycle of a buah from an orchestrator perspective.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuahStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl BuahStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown buah status: {0}")]
pub struct UnknownBuahStatus(pub String);

impl TryFrom<&str> for BuahStatus {
    type Error = UnknownBuahStatus;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(UnknownBuahStatus(other.to_owned())),
        }
    }
}

/// High-level status emitted by an adapter while running.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Starting,
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
}

/// Result of running sortir (verification) commands against a buah.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SortirStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn run_status_serializes_snake_case() {
        let json = serde_json::to_string(&RunStatus::InProgress).expect("ser");
        assert_eq!(json, "\"in_progress\"");
    }

    #[test]
    fn buah_status_parses_from_db_string() {
        assert_eq!(
            BuahStatus::try_from("queued").expect("q"),
            BuahStatus::Queued
        );
        assert_eq!(
            BuahStatus::try_from("running").expect("r"),
            BuahStatus::Running
        );
        assert_eq!(
            BuahStatus::try_from("completed").expect("c"),
            BuahStatus::Completed
        );
        assert_eq!(
            BuahStatus::try_from("failed").expect("f"),
            BuahStatus::Failed
        );
        assert_eq!(
            BuahStatus::try_from("cancelled").expect("x"),
            BuahStatus::Cancelled
        );
        assert!(BuahStatus::try_from("banana").is_err());
    }

    #[test]
    fn buah_status_round_trips_to_str() {
        for status in [
            BuahStatus::Queued,
            BuahStatus::Running,
            BuahStatus::Completed,
            BuahStatus::Failed,
            BuahStatus::Cancelled,
        ] {
            let s = status.as_str();
            let back = BuahStatus::try_from(s).expect("roundtrip");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn sortir_status_default_is_pending() {
        assert_eq!(SortirStatus::default(), SortirStatus::Pending);
    }

    #[test]
    fn buah_status_terminal_flag() {
        assert!(!BuahStatus::Queued.is_terminal());
        assert!(!BuahStatus::Running.is_terminal());
        assert!(BuahStatus::Completed.is_terminal());
        assert!(BuahStatus::Failed.is_terminal());
        assert!(BuahStatus::Cancelled.is_terminal());
    }
}
