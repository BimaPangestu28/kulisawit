//! Registry of `AgentAdapter` implementations keyed by `id()`.
//!
//! Agents are registered at orchestrator construction time and looked up by
//! string id when a caller asks to dispatch an attempt.

use kulisawit_core::AgentAdapter;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Default)]
pub struct AgentRegistry {
    agents: HashMap<String, Arc<dyn AgentAdapter>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
        }
    }

    /// Register an adapter. If an adapter with the same id already exists,
    /// it is replaced.
    pub fn register(&mut self, adapter: Arc<dyn AgentAdapter>) {
        let id = adapter.id().to_owned();
        self.agents.insert(id, adapter);
    }

    /// Look up an adapter by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn AgentAdapter>> {
        self.agents.get(id).cloned()
    }

    /// All registered ids, alphabetically sorted.
    pub fn ids(&self) -> Vec<String> {
        let mut v: Vec<String> = self.agents.keys().cloned().collect();
        v.sort();
        v
    }
}
