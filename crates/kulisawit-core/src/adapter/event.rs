//! Events emitted by a kuli while running, plus run-time context types.

use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

use crate::status::RunStatus;

/// Context handed to a kuli for a single run.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunContext {
    pub run_id: String,
    pub petak_path: PathBuf,
    pub prompt: String,
    pub prompt_variant: Option<String>,
    pub env: HashMap<String, String>,
}

/// Result of an adapter health check.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CheckResult {
    pub ok: bool,
    pub message: Option<String>,
    pub version: Option<String>,
}

/// Structured events streamed from a running kuli.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KuliEvent {
    Stdout {
        text: String,
    },
    Stderr {
        text: String,
    },
    ToolCall {
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        name: String,
        output: serde_json::Value,
    },
    FileEdit {
        path: String,
        diff: Option<String>,
    },
    Status {
        status: RunStatus,
        detail: Option<String>,
    },
}
