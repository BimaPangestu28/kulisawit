//! Compile-time checks: the adapter trait must be dyn-compatible.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::adapter::AgentAdapter;
use std::sync::Arc;

#[test]
fn agent_adapter_is_dyn_compatible() {
    // If this compiles, the trait stays object-safe for the orchestrator's
    // `Arc<dyn AgentAdapter>` registry.
    fn _assert_object_safe(_: Arc<dyn AgentAdapter>) {}
}

#[test]
fn agent_event_serializes_with_tag() {
    use kulisawit_core::adapter::AgentEvent;
    let evt = AgentEvent::Stdout {
        text: "hello".into(),
    };
    let json = serde_json::to_string(&evt).expect("ser");
    assert!(json.contains("\"type\":\"stdout\""));
    assert!(json.contains("\"text\":\"hello\""));
}
