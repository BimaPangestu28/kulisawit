//! Status enums for runs, attempts, and verification.

use serde::{Deserialize, Serialize};

/// Lifecycle of an attempt from an orchestrator perspective.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl AttemptStatus {
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

impl AttemptStatus {
    /// Map a terminal adapter `RunStatus` to its database counterpart.
    /// Non-terminal states (`Starting`, `InProgress`) map to `None`.
    pub fn from_terminal_run_status(run: RunStatus) -> Option<Self> {
        match run {
            RunStatus::Succeeded => Some(Self::Completed),
            RunStatus::Failed => Some(Self::Failed),
            RunStatus::Cancelled => Some(Self::Cancelled),
            RunStatus::Starting | RunStatus::InProgress => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown attempt status: {0}")]
pub struct UnknownAttemptStatus(pub String);

impl TryFrom<&str> for AttemptStatus {
    type Error = UnknownAttemptStatus;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(UnknownAttemptStatus(other.to_owned())),
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

/// Result of running verification commands against an attempt.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum VerificationStatus {
    #[default]
    Pending,
    Passed,
    Failed,
    Skipped,
}

impl VerificationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("unknown verification status: {0}")]
pub struct UnknownVerificationStatus(pub String);

impl TryFrom<&str> for VerificationStatus {
    type Error = UnknownVerificationStatus;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "passed" => Ok(Self::Passed),
            "failed" => Ok(Self::Failed),
            "skipped" => Ok(Self::Skipped),
            other => Err(UnknownVerificationStatus(other.to_owned())),
        }
    }
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
    fn attempt_status_parses_from_db_string() {
        assert_eq!(
            AttemptStatus::try_from("queued").expect("q"),
            AttemptStatus::Queued
        );
        assert_eq!(
            AttemptStatus::try_from("running").expect("r"),
            AttemptStatus::Running
        );
        assert_eq!(
            AttemptStatus::try_from("completed").expect("c"),
            AttemptStatus::Completed
        );
        assert_eq!(
            AttemptStatus::try_from("failed").expect("f"),
            AttemptStatus::Failed
        );
        assert_eq!(
            AttemptStatus::try_from("cancelled").expect("x"),
            AttemptStatus::Cancelled
        );
        assert!(AttemptStatus::try_from("banana").is_err());
    }

    #[test]
    fn attempt_status_round_trips_to_str() {
        for status in [
            AttemptStatus::Queued,
            AttemptStatus::Running,
            AttemptStatus::Completed,
            AttemptStatus::Failed,
            AttemptStatus::Cancelled,
        ] {
            let s = status.as_str();
            let back = AttemptStatus::try_from(s).expect("roundtrip");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn verification_status_default_is_pending() {
        assert_eq!(VerificationStatus::default(), VerificationStatus::Pending);
    }

    #[test]
    fn attempt_status_terminal_flag() {
        assert!(!AttemptStatus::Queued.is_terminal());
        assert!(!AttemptStatus::Running.is_terminal());
        assert!(AttemptStatus::Completed.is_terminal());
        assert!(AttemptStatus::Failed.is_terminal());
        assert!(AttemptStatus::Cancelled.is_terminal());
    }

    #[test]
    fn run_status_terminal_maps_to_attempt_status() {
        assert_eq!(
            AttemptStatus::from_terminal_run_status(RunStatus::Succeeded),
            Some(AttemptStatus::Completed)
        );
        assert_eq!(
            AttemptStatus::from_terminal_run_status(RunStatus::Failed),
            Some(AttemptStatus::Failed)
        );
        assert_eq!(
            AttemptStatus::from_terminal_run_status(RunStatus::Cancelled),
            Some(AttemptStatus::Cancelled)
        );
    }

    #[test]
    fn run_status_non_terminal_maps_to_none() {
        assert!(AttemptStatus::from_terminal_run_status(RunStatus::Starting).is_none());
        assert!(AttemptStatus::from_terminal_run_status(RunStatus::InProgress).is_none());
    }

    #[test]
    fn verification_status_round_trips_to_str() {
        for status in [
            VerificationStatus::Pending,
            VerificationStatus::Passed,
            VerificationStatus::Failed,
            VerificationStatus::Skipped,
        ] {
            let s = status.as_str();
            let back = VerificationStatus::try_from(s).expect("roundtrip");
            assert_eq!(back, status);
        }
    }

    #[test]
    fn verification_status_rejects_unknown() {
        assert!(VerificationStatus::try_from("banana").is_err());
    }

    #[test]
    fn verification_status_default_is_still_pending() {
        assert_eq!(VerificationStatus::default(), VerificationStatus::Pending);
    }
}
