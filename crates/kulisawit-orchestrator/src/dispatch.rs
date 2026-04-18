//! Per-attempt dispatch lifecycle.
//!
//! Flow of `dispatch_single_attempt`:
//!
//! 1. Acquire a permit from the orchestrator semaphore.
//! 2. Fetch the `Task` row; error if missing.
//! 3. Compose the prompt via [`crate::prompt::compose_prompt`].
//! 4. Look up the adapter by id; error if missing.
//! 5. Insert an `attempt` row in `Queued` with the allocated worktree path
//!    and branch name.
//! 6. Create the git worktree on disk.
//! 7. Transition to `Running`.
//! 8. Run the adapter; consume the returned event stream. For each event,
//!    append to the DB event log and broadcast. When a terminal `Status`
//!    is seen, remember the mapped `AttemptStatus` and break.
//!    Cancellation is polled via `tokio::select!` on the per-attempt
//!    `Notify`; if fired, call `adapter.cancel` and map to
//!    `AttemptStatus::Cancelled`.
//! 9. Commit changes in the worktree.
//! 10. Transition the attempt to its terminal `AttemptStatus`.
//! 11. Close the broadcaster channel and drop the cancel flag.

use std::sync::Arc;

use futures::StreamExt;
use kulisawit_core::{AgentEvent, AttemptId, AttemptStatus, RunContext, RunStatus, TaskId};
use kulisawit_db::{attempt, events, task};
use kulisawit_git::{
    branch::commit_all_in_worktree,
    query::head_commit_sha,
    worktree::{create_worktree, CreateWorktreeRequest},
};
use tracing::{instrument, warn};

use crate::{Orchestrator, OrchestratorError, OrchestratorResult};

/// Short id helper: first 8 chars of a UUID-v7-ish string.
fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

#[instrument(skip(orch), fields(task = %task_id, agent = agent_id))]
pub async fn dispatch_single_attempt(
    orch: &Orchestrator,
    task_id: &TaskId,
    agent_id: &str,
    prompt_variant: Option<String>,
) -> OrchestratorResult<AttemptId> {
    let _permit = orch
        .semaphore()
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| OrchestratorError::Invalid(format!("semaphore closed: {e}")))?;

    let task_row = task::get(orch.pool(), task_id)
        .await?
        .ok_or_else(|| OrchestratorError::Invalid(format!("task not found: {task_id}")))?;

    let prompt = crate::prompt::compose_prompt(&task_row, prompt_variant.as_deref());

    let adapter = orch
        .registry()
        .get(agent_id)
        .ok_or_else(|| OrchestratorError::Invalid(format!("agent not registered: {agent_id}")))?;

    let attempt_id = AttemptId::new();
    let attempt_short = short(attempt_id.as_str());
    let task_short = short(task_id.as_str());
    let branch_name = format!("kulisawit/{task_short}/{attempt_short}");
    let worktree_path = orch
        .worktree_root()
        .join(format!("attempt-{attempt_short}"));

    let base_ref = head_commit_sha(orch.repo_root()).map_err(OrchestratorError::from)?;

    let attempt_id = attempt::create(
        orch.pool(),
        attempt::NewAttempt {
            task_id: task_id.clone(),
            agent_id: agent_id.to_owned(),
            prompt_variant: prompt_variant.clone(),
            worktree_path: worktree_path.display().to_string(),
            branch_name: branch_name.clone(),
        },
    )
    .await?;

    let wt_outcome = create_worktree(CreateWorktreeRequest {
        repo_root: orch.repo_root().to_path_buf(),
        worktree_root: orch.worktree_root().to_path_buf(),
        attempt_short_id: attempt_short.clone(),
        branch_name: branch_name.clone(),
        base_ref,
    })
    .await?;

    let cancel_notify = orch.install_cancel_flag(&attempt_id).await;
    attempt::mark_running(orch.pool(), &attempt_id).await?;

    let run_ctx = RunContext {
        run_id: attempt_id.as_str().to_owned(),
        worktree_path: wt_outcome.worktree_path.clone(),
        prompt,
        prompt_variant,
        env: std::collections::HashMap::new(),
    };

    let mut stream = adapter.run(run_ctx).await?;

    let terminal: AttemptStatus = loop {
        tokio::select! {
            biased;
            _ = cancel_notify.notified() => {
                let _ = adapter.cancel(attempt_id.as_str()).await;
                let evt = AgentEvent::Status {
                    status: RunStatus::Cancelled,
                    detail: Some("cancelled by orchestrator".into()),
                };
                let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                orch.broadcaster().send(&attempt_id, evt);
                break AttemptStatus::Cancelled;
            }
            next = stream.next() => {
                let Some(evt) = next else {
                    warn!(attempt = %attempt_id, "adapter stream ended without terminal status");
                    break AttemptStatus::Failed;
                };
                let _ = events::append(orch.pool(), &attempt_id, &evt).await;
                orch.broadcaster().send(&attempt_id, evt.clone());
                if let AgentEvent::Status { status, .. } = &evt {
                    if let Some(mapped) = AttemptStatus::from_terminal_run_status(*status) {
                        break mapped;
                    }
                }
            }
        }
    };

    let attempt_title = &task_row.title;
    let commit_msg = format!("kulisawit: attempt {attempt_short} for {attempt_title}");
    if let Err(e) = commit_all_in_worktree(&wt_outcome.worktree_path, &commit_msg).await {
        warn!(attempt = %attempt_id, "commit_all_in_worktree failed: {e}");
    }
    attempt::mark_terminal(orch.pool(), &attempt_id, terminal).await?;

    orch.broadcaster().close(&attempt_id);
    orch.remove_cancel_flag(&attempt_id).await;

    Ok(attempt_id)
}

#[instrument(skip(orch), fields(task = %task_id, agent = agent_id, n = batch_size))]
pub async fn dispatch_batch(
    orch: &Orchestrator,
    task_id: &TaskId,
    agent_id: &str,
    batch_size: usize,
    variants: Option<Vec<String>>,
) -> OrchestratorResult<Vec<AttemptId>> {
    if batch_size == 0 {
        return Err(OrchestratorError::Invalid("batch_size must be >= 1".into()));
    }
    if let Some(v) = &variants {
        if v.len() != batch_size {
            return Err(OrchestratorError::Invalid(format!(
                "variants length {} != batch_size {}",
                v.len(),
                batch_size
            )));
        }
    }

    let orch = Arc::new(orch.clone_for_dispatch());

    let mut handles = Vec::with_capacity(batch_size);
    for i in 0..batch_size {
        let orch = Arc::clone(&orch);
        let task_id = task_id.clone();
        let agent_id = agent_id.to_owned();
        let variant = variants.as_ref().and_then(|v| v.get(i).cloned());
        handles.push(tokio::spawn(async move {
            dispatch_single_attempt(&orch, &task_id, &agent_id, variant).await
        }));
    }

    let mut ids = Vec::with_capacity(batch_size);
    for h in handles {
        match h.await {
            Ok(Ok(id)) => ids.push(id),
            Ok(Err(e)) => return Err(e),
            Err(join_err) => {
                return Err(OrchestratorError::Invalid(format!(
                    "dispatch task panicked: {join_err}"
                )))
            }
        }
    }
    Ok(ids)
}
