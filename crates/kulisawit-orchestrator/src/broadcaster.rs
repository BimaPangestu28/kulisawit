//! Per-attempt broadcast fanout of `AgentEvent`s.
//!
//! Each attempt gets its own `tokio::sync::broadcast::Sender`. Subscribers
//! (one per SSE client, typically) read from the corresponding `Receiver`.
//! The orchestrator's event loop sends into the channel for every inbound
//! event, and calls `close` when the attempt is terminal — dropping the
//! `Sender` signals receivers that no further events will arrive.

use std::collections::HashMap;
use std::sync::Mutex;

use kulisawit_core::{AgentEvent, AttemptId};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct EventBroadcaster {
    channels: Mutex<HashMap<String, broadcast::Sender<AgentEvent>>>,
    capacity: usize,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        Self {
            channels: Mutex::new(HashMap::new()),
            capacity,
        }
    }

    /// Subscribe to an attempt's event stream. The channel is created on demand.
    #[allow(clippy::expect_used)]
    pub fn subscribe(&self, attempt: &AttemptId) -> broadcast::Receiver<AgentEvent> {
        let mut guard = self.channels.lock().expect("broadcaster mutex poisoned");
        let tx = guard
            .entry(attempt.as_str().to_owned())
            .or_insert_with(|| broadcast::channel::<AgentEvent>(self.capacity).0);
        tx.subscribe()
    }

    /// Fanout an event. Silently creates a channel if one does not exist.
    /// Drops the event (returns Ok) if no receivers are currently attached.
    #[allow(clippy::expect_used)]
    pub fn send(&self, attempt: &AttemptId, event: AgentEvent) {
        let mut guard = self.channels.lock().expect("broadcaster mutex poisoned");
        let tx = guard
            .entry(attempt.as_str().to_owned())
            .or_insert_with(|| broadcast::channel::<AgentEvent>(self.capacity).0);
        let _ = tx.send(event);
    }

    /// Drop the channel for the given attempt. Any live receivers will see
    /// the channel close (`recv().await` returns `Err`).
    #[allow(clippy::expect_used)]
    pub fn close(&self, attempt: &AttemptId) {
        let mut guard = self.channels.lock().expect("broadcaster mutex poisoned");
        guard.remove(attempt.as_str());
    }
}

impl Default for EventBroadcaster {
    fn default() -> Self {
        Self::new(256)
    }
}
