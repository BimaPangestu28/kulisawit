//! Sortir (verification) runner.
//!
//! Auto-triggered by [`crate::dispatch::dispatch_one_attempt`] after an
//! attempt reaches agent terminal `Completed`. Loads
//! `<repo_root>/.kulisawit/sortir.toml`, runs each `[[checks]]` entry as a
//! subprocess in the petak (worktree) cwd, aggregates output, and persists
//! the result via [`kulisawit_db::attempt::set_verification`].

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, RunStatus, VerificationStatus};
use kulisawit_db::attempt;
use serde::Deserialize;
use tokio::process::Command;
use tokio::time::timeout;

use crate::orchestrator::Orchestrator;

const PER_CHECK_OUTPUT_CAP: usize = 64 * 1024;
const TOTAL_OUTPUT_CAP: usize = 256 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct SortirConfig {
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Check {
    pub name: String,
    pub command: Vec<String>,
    pub timeout_secs: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse: {0}")]
    Parse(String),
}

/// Read `<repo_root>/.kulisawit/sortir.toml`. Returns `Ok(None)` when the file
/// is absent, `Ok(Some(_))` for a valid config, `Err(_)` for a malformed file.
pub async fn load_config(repo_root: &Path) -> Result<Option<SortirConfig>, ConfigError> {
    let path = repo_root.join(".kulisawit").join("sortir.toml");
    let text = match tokio::fs::read_to_string(&path).await {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(ConfigError::Io(e)),
    };
    let config: SortirConfig =
        toml::from_str(&text).map_err(|e| ConfigError::Parse(e.to_string()))?;
    validate(&config).map_err(ConfigError::Parse)?;
    Ok(Some(config))
}

fn validate(config: &SortirConfig) -> Result<(), String> {
    for check in &config.checks {
        if check.name.trim().is_empty() {
            return Err("check name must be non-empty".into());
        }
        if check.command.is_empty() {
            return Err(format!(
                "check '{}' command array must be non-empty",
                check.name
            ));
        }
        if check.timeout_secs == 0 || check.timeout_secs > 1800 {
            return Err(format!(
                "check '{}' timeout_secs must be between 1 and 1800",
                check.name
            ));
        }
    }
    Ok(())
}

#[derive(Debug)]
enum CheckOutcome {
    Pass,
    Fail,
    Timeout,
}

impl CheckOutcome {
    fn label(&self) -> &'static str {
        match self {
            Self::Pass => "passed",
            Self::Fail => "failed",
            Self::Timeout => "TIMEOUT",
        }
    }
}

struct CheckResult {
    name: String,
    outcome: CheckOutcome,
    output: String,
    duration_ms: u128,
}

async fn run_check(check: &Check, cwd: &Path) -> CheckResult {
    let start = Instant::now();
    let mut cmd = Command::new(&check.command[0]);
    cmd.args(&check.command[1..])
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return CheckResult {
                name: check.name.clone(),
                outcome: CheckOutcome::Fail,
                output: format!("spawn failed: {e}"),
                duration_ms: start.elapsed().as_millis(),
            };
        }
    };

    let wait = child.wait_with_output();
    match timeout(Duration::from_secs(check.timeout_secs), wait).await {
        Ok(Ok(out)) => {
            let stdout = truncate(String::from_utf8_lossy(&out.stdout).into_owned());
            let stderr = truncate(String::from_utf8_lossy(&out.stderr).into_owned());
            let combined = if stderr.is_empty() {
                stdout
            } else if stdout.is_empty() {
                stderr
            } else {
                format!("{stdout}\n{stderr}")
            };
            let outcome = if out.status.success() {
                CheckOutcome::Pass
            } else {
                CheckOutcome::Fail
            };
            CheckResult {
                name: check.name.clone(),
                outcome,
                output: combined,
                duration_ms: start.elapsed().as_millis(),
            }
        }
        Ok(Err(e)) => CheckResult {
            name: check.name.clone(),
            outcome: CheckOutcome::Fail,
            output: format!("io error: {e}"),
            duration_ms: start.elapsed().as_millis(),
        },
        Err(_elapsed) => CheckResult {
            name: check.name.clone(),
            outcome: CheckOutcome::Timeout,
            output: format!("TIMEOUT after {}s", check.timeout_secs),
            duration_ms: start.elapsed().as_millis(),
        },
    }
}

fn truncate(mut s: String) -> String {
    if s.len() > PER_CHECK_OUTPUT_CAP {
        s.truncate(PER_CHECK_OUTPUT_CAP);
        s.push_str("\n... (truncated)");
    }
    s
}

fn format_block(result: &CheckResult) -> String {
    format!(
        "=== {} ({}, {}ms) ===\n{}\n\n",
        result.name,
        result.outcome.label(),
        result.duration_ms,
        result.output
    )
}

fn aggregate_blocks(blocks: Vec<String>) -> String {
    let mut total: usize = blocks.iter().map(String::len).sum();
    let mut blocks = blocks;
    let mut dropped = 0usize;
    while total > TOTAL_OUTPUT_CAP && !blocks.is_empty() {
        total -= blocks[0].len();
        blocks.remove(0);
        dropped += 1;
    }
    let mut joined: String = blocks.concat();
    if dropped > 0 {
        joined = format!("--- truncated {dropped} earlier blocks ---\n{joined}");
    }
    joined
}

/// Runs every check and returns the aggregated `(status, output)`.
/// Public for unit tests; production callers use [`run_sortir`].
pub async fn run_checks(config: &SortirConfig, cwd: &Path) -> (VerificationStatus, String) {
    let mut blocks = Vec::with_capacity(config.checks.len());
    let mut all_pass = true;
    for check in &config.checks {
        let result = run_check(check, cwd).await;
        if !matches!(result.outcome, CheckOutcome::Pass) {
            all_pass = false;
        }
        blocks.push(format_block(&result));
    }
    let status = if all_pass {
        VerificationStatus::Passed
    } else {
        VerificationStatus::Failed
    };
    (status, aggregate_blocks(blocks))
}

/// Production entrypoint. Loads config, runs checks, persists, broadcasts.
/// Best-effort: all errors are logged and swallowed (don't poison the
/// orchestrator).
pub async fn run_sortir(orch: Arc<Orchestrator>, attempt_id: AttemptId) {
    let pool = orch.pool();
    let attempt = match attempt::get(pool, &attempt_id).await {
        Ok(Some(a)) => a,
        Ok(None) => return,
        Err(e) => {
            tracing::warn!(?e, ?attempt_id, "sortir: failed to load attempt");
            return;
        }
    };
    let cwd = std::path::PathBuf::from(&attempt.worktree_path);

    let (final_status, output) = match load_config(orch.repo_root()).await {
        Ok(None) => (
            VerificationStatus::Skipped,
            "no .kulisawit/sortir.toml found".to_string(),
        ),
        Err(e) => (
            VerificationStatus::Failed,
            format!("sortir.toml invalid: {e}"),
        ),
        Ok(Some(config)) => run_checks(&config, &cwd).await,
    };

    if let Err(e) =
        attempt::set_verification(pool, &attempt_id, final_status, Some(&output)).await
    {
        tracing::warn!(?e, "sortir: failed to write verification");
    }

    let derived = match attempt.status {
        AttemptStatus::Completed => RunStatus::Succeeded,
        AttemptStatus::Failed => RunStatus::Failed,
        AttemptStatus::Cancelled => RunStatus::Cancelled,
        _ => RunStatus::Failed,
    };
    let detail = match final_status {
        VerificationStatus::Passed => "sortir:passed",
        VerificationStatus::Failed => "sortir:failed",
        VerificationStatus::Skipped => "sortir:skipped",
        VerificationStatus::Pending => "sortir:pending",
    };
    orch.broadcaster().send(
        &attempt_id,
        AgentEvent::Status {
            status: derived,
            detail: Some(detail.to_string()),
        },
    );
}
