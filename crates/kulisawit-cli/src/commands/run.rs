//! `kulisawit run` — dispatch a batch of attempts for a task.

use anyhow::{Context, Result};
use std::sync::Arc;

use kulisawit_agent::MockAgent;
use kulisawit_core::AgentAdapter;
use kulisawit_db::{attempt, connect, migrate};
use kulisawit_orchestrator::{dispatch_batch, AgentRegistry, Orchestrator, RuntimeConfig};

use crate::RunArgs;

pub async fn run(args: RunArgs) -> Result<()> {
    let db_str = args
        .db
        .to_str()
        .context("--db path is not valid UTF-8")?
        .to_owned();
    let pool = connect(&db_str).await.context("open db")?;
    migrate(&pool).await.context("migrate")?;

    let mut registry = AgentRegistry::new();
    // Phase 2 ships only the MockAgent adapter.
    registry.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);

    let worktree_root = args.repo.join(".kulisawit/worktrees");
    let cfg = RuntimeConfig::default();
    let orch = Orchestrator::new(pool, registry, args.repo.clone(), worktree_root, cfg);

    let ids = dispatch_batch(&orch, &args.task, &args.agent, args.batch, None)
        .await
        .context("dispatch_batch")?;

    println!("{:<36}  {:<10}", "attempt_id", "status");
    println!("{:-<36}  {:-<10}", "", "");
    for id in &ids {
        let row = attempt::get(orch.pool(), id)
            .await
            .context("attempt::get")?
            .context("attempt row missing after dispatch")?;
        println!("{:<36}  {:<10}", id.as_str(), row.status.as_str());
    }
    Ok(())
}
