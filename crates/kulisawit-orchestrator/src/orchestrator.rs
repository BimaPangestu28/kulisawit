//! The `Orchestrator` struct: shared state for dispatching attempts.
//!
//! The type is `Send + Sync` and cheap to clone: all mutable state sits
//! behind `Arc`-wrapped interior-mutability primitives so a single
//! `Orchestrator` value can be shared across spawned dispatch tasks and HTTP
//! handlers without a surrounding lock.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use kulisawit_core::AttemptId;
use kulisawit_db::DbPool;
use tokio::sync::{Mutex, Notify, Semaphore};

use crate::{AgentRegistry, EventBroadcaster, RuntimeConfig};

#[derive(Debug)]
pub struct Orchestrator {
    pool: Arc<DbPool>,
    registry: Arc<AgentRegistry>,
    broadcaster: Arc<EventBroadcaster>,
    worktree_root: PathBuf,
    repo_root: PathBuf,
    semaphore: Arc<Semaphore>,
    config: RuntimeConfig,
    cancel_flags: Arc<Mutex<HashMap<String, Arc<Notify>>>>,
}

impl Orchestrator {
    pub fn new(
        pool: DbPool,
        registry: AgentRegistry,
        repo_root: PathBuf,
        worktree_root: PathBuf,
        config: RuntimeConfig,
    ) -> Self {
        let permits = config.max_concurrent_attempts.max(1);
        Self {
            pool: Arc::new(pool),
            registry: Arc::new(registry),
            broadcaster: Arc::new(EventBroadcaster::default()),
            worktree_root,
            repo_root,
            semaphore: Arc::new(Semaphore::new(permits)),
            config,
            cancel_flags: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn pool(&self) -> &Arc<DbPool> {
        &self.pool
    }

    pub fn registry(&self) -> &Arc<AgentRegistry> {
        &self.registry
    }

    pub fn broadcaster(&self) -> &Arc<EventBroadcaster> {
        &self.broadcaster
    }

    pub fn worktree_root(&self) -> &std::path::Path {
        &self.worktree_root
    }

    pub fn repo_root(&self) -> &std::path::Path {
        &self.repo_root
    }

    pub fn semaphore(&self) -> &Arc<Semaphore> {
        &self.semaphore
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// Install a cancel `Notify` for the given attempt id. Called by the
    /// dispatcher at the start of each attempt so `cancel_attempt` can
    /// reach it later. Idempotent.
    pub async fn install_cancel_flag(&self, id: &AttemptId) -> Arc<Notify> {
        let mut g = self.cancel_flags.lock().await;
        g.entry(id.as_str().to_owned())
            .or_insert_with(|| Arc::new(Notify::new()))
            .clone()
    }

    /// Remove the cancel `Notify` for the given attempt id. Called on
    /// terminal transition.
    pub async fn remove_cancel_flag(&self, id: &AttemptId) {
        let mut g = self.cancel_flags.lock().await;
        g.remove(id.as_str());
    }

    /// Look up (but do not install) an existing cancel `Notify`.
    pub async fn cancel_flag(&self, id: &AttemptId) -> Option<Arc<Notify>> {
        let g = self.cancel_flags.lock().await;
        g.get(id.as_str()).cloned()
    }

    /// Request cancellation of a live attempt. Returns `Ok(())` whether or
    /// not the attempt is currently running — the dispatcher checks the
    /// flag on its next event poll.
    pub async fn cancel_attempt(&self, id: &AttemptId) -> crate::OrchestratorResult<()> {
        if let Some(n) = self.cancel_flag(id).await {
            n.notify_one();
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use kulisawit_agent::MockAgent;
    use kulisawit_core::AgentAdapter;
    use kulisawit_db::{connect, migrate};
    use std::sync::Arc;
    use tempfile::tempdir;

    #[tokio::test]
    async fn constructor_smoke_works_with_real_pool_and_registry() {
        let pool = connect("sqlite::memory:").await.expect("pool");
        migrate(&pool).await.expect("mig");
        let mut registry = crate::AgentRegistry::new();
        registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
        let base = tempdir().expect("tmp");
        let cfg = crate::RuntimeConfig::default();
        let orch = Orchestrator::new(
            pool,
            registry,
            base.path().to_path_buf(),
            base.path().join(".kulisawit/worktrees"),
            cfg,
        );
        assert_eq!(orch.config().default_agent_id, "mock");
        assert_eq!(orch.config().max_concurrent_attempts, 8);
        assert!(orch.registry().get("mock").is_some());
    }
}
