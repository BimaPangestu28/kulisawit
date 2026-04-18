#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_agent::MockAgent;
use kulisawit_core::{AgentAdapter, AgentError, AgentEvent, CheckResult, RunContext};
use kulisawit_orchestrator::AgentRegistry;
use std::sync::Arc;

#[test]
fn register_and_get_roundtrip() {
    let mut reg = AgentRegistry::new();
    reg.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    let adapter = reg.get("mock").expect("registered");
    assert_eq!(adapter.id(), "mock");
}

#[test]
fn get_missing_returns_none() {
    let reg = AgentRegistry::new();
    assert!(reg.get("not-there").is_none());
}

#[test]
fn ids_sorted_alphabetically() {
    let mut reg = AgentRegistry::new();
    reg.register(Arc::new(MockAgent::default()) as Arc<dyn AgentAdapter>);
    reg.register(Arc::new(NamedAgent("zeta")) as Arc<dyn AgentAdapter>);
    reg.register(Arc::new(NamedAgent("alpha")) as Arc<dyn AgentAdapter>);
    let ids = reg.ids();
    assert_eq!(ids, vec!["alpha", "mock", "zeta"]);
}

#[test]
fn register_overwrites_on_duplicate_id() {
    let mut reg = AgentRegistry::new();
    reg.register(Arc::new(NamedAgent("alpha")) as Arc<dyn AgentAdapter>);
    reg.register(Arc::new(NamedAgent("alpha")) as Arc<dyn AgentAdapter>);
    assert_eq!(reg.ids(), vec!["alpha"]);
}

/// Helper that pretends to be a distinct adapter with a custom id.
#[derive(Debug)]
struct NamedAgent(&'static str);

#[async_trait::async_trait]
impl AgentAdapter for NamedAgent {
    fn id(&self) -> &str {
        self.0
    }
    fn display_name(&self) -> &str {
        self.0
    }
    fn version(&self) -> &str {
        "0"
    }
    async fn check(&self) -> Result<CheckResult, AgentError> {
        Ok(CheckResult {
            ok: true,
            message: None,
            version: None,
        })
    }
    async fn run(
        &self,
        _ctx: RunContext,
    ) -> Result<futures::stream::BoxStream<'static, AgentEvent>, AgentError> {
        Ok(Box::pin(futures::stream::empty()))
    }
    async fn cancel(&self, _run_id: &str) -> Result<(), AgentError> {
        Ok(())
    }
}
