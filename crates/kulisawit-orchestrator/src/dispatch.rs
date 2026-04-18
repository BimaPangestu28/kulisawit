//! Per-attempt dispatch — implemented in Task 2.1.7.

use crate::{Orchestrator, OrchestratorError, OrchestratorResult};
use kulisawit_core::{AttemptId, TaskId};

#[allow(dead_code, clippy::needless_pass_by_value)]
pub async fn dispatch_single_attempt(
    _orch: &Orchestrator,
    _task_id: &TaskId,
    _agent_id: &str,
    _prompt_variant: Option<String>,
) -> OrchestratorResult<AttemptId> {
    Err(OrchestratorError::Invalid(
        "dispatch_single_attempt not yet implemented".into(),
    ))
}

#[allow(dead_code, clippy::needless_pass_by_value)]
pub async fn dispatch_batch(
    _orch: &Orchestrator,
    _task_id: &TaskId,
    _agent_id: &str,
    _batch_size: usize,
    _variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>> {
    Err(OrchestratorError::Invalid(
        "dispatch_batch not yet implemented".into(),
    ))
}
