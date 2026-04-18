//! Runtime configuration loaded from `peta-kebun.toml`.
//!
//! Only a `[runtime]` block is consumed here; other blocks
//! (`[kebun]`, `[agents]`) are handled elsewhere.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeConfig {
    pub max_concurrent_attempts: usize,
    pub worktree_retention_days: u32,
    pub default_agent_id: String,
    pub default_batch_size: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            max_concurrent_attempts: 8,
            worktree_retention_days: 7,
            default_agent_id: "mock".into(),
            default_batch_size: 1,
        }
    }
}

#[derive(Debug, Deserialize)]
struct RawRuntimeFile {
    #[serde(default)]
    runtime: Option<RawRuntimeBlock>,
}

#[derive(Debug, Default, Deserialize)]
struct RawRuntimeBlock {
    max_concurrent_attempts: Option<usize>,
    worktree_retention_days: Option<u32>,
    default_agent_id: Option<String>,
    default_batch_size: Option<usize>,
}

impl RuntimeConfig {
    /// Parse a `peta-kebun.toml` string, extracting only the `[runtime]`
    /// block. Missing keys fall back to [`Default::default`].
    pub fn from_toml_str(s: &str) -> Result<Self, toml::de::Error> {
        let raw: RawRuntimeFile = toml::from_str(s)?;
        let defaults = Self::default();
        let block = raw.runtime.unwrap_or_default();
        Ok(Self {
            max_concurrent_attempts: block
                .max_concurrent_attempts
                .unwrap_or(defaults.max_concurrent_attempts),
            worktree_retention_days: block
                .worktree_retention_days
                .unwrap_or(defaults.worktree_retention_days),
            default_agent_id: block.default_agent_id.unwrap_or(defaults.default_agent_id),
            default_batch_size: block
                .default_batch_size
                .unwrap_or(defaults.default_batch_size),
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn default_values_are_sane() {
        let c = RuntimeConfig::default();
        assert_eq!(c.max_concurrent_attempts, 8);
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }

    #[test]
    fn parses_runtime_block_from_toml() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 4
worktree_retention_days = 14
default_agent_id = "claude-code"
default_batch_size = 3
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 4);
        assert_eq!(c.worktree_retention_days, 14);
        assert_eq!(c.default_agent_id, "claude-code");
        assert_eq!(c.default_batch_size, 3);
    }

    #[test]
    fn missing_runtime_block_returns_default() {
        let toml_src = "[kebun]\nname = \"demo\"\n";
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c, RuntimeConfig::default());
    }

    #[test]
    fn malformed_toml_returns_err() {
        let toml_src = "this is not valid [[[";
        assert!(RuntimeConfig::from_toml_str(toml_src).is_err());
    }

    #[test]
    fn partial_runtime_block_fills_defaults() {
        let toml_src = r#"
[runtime]
max_concurrent_attempts = 2
"#;
        let c = RuntimeConfig::from_toml_str(toml_src).expect("parse");
        assert_eq!(c.max_concurrent_attempts, 2);
        assert_eq!(c.worktree_retention_days, 7);
        assert_eq!(c.default_agent_id, "mock");
        assert_eq!(c.default_batch_size, 1);
    }
}
