//! Compile-time checks: the adapter trait must be dyn-compatible.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::adapter::KuliAdapter;
use std::sync::Arc;

#[test]
fn kuli_adapter_is_dyn_compatible() {
    // If this compiles, the trait stays object-safe for the orchestrator's
    // `Arc<dyn KuliAdapter>` registry.
    fn _assert_object_safe(_: Arc<dyn KuliAdapter>) {}
}

#[test]
fn kuli_event_serializes_with_tag() {
    use kulisawit_core::adapter::KuliEvent;
    let evt = KuliEvent::Stdout { text: "hello".into() };
    let json = serde_json::to_string(&evt).expect("ser");
    assert!(json.contains("\"type\":\"stdout\""));
    assert!(json.contains("\"text\":\"hello\""));
}
