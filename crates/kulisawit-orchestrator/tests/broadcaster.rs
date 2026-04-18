#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use kulisawit_core::{AgentEvent, AttemptId};
use kulisawit_orchestrator::EventBroadcaster;

#[tokio::test]
async fn subscriber_before_send_receives_event() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx = bc.subscribe(&id);
    bc.send(&id, AgentEvent::Stdout { text: "hi".into() });
    let evt = rx.recv().await.expect("recv");
    assert!(matches!(evt, AgentEvent::Stdout { .. }));
}

#[tokio::test]
async fn two_subscribers_both_receive() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx1 = bc.subscribe(&id);
    let mut rx2 = bc.subscribe(&id);
    bc.send(&id, AgentEvent::Stdout { text: "a".into() });
    bc.send(&id, AgentEvent::Stdout { text: "b".into() });
    for rx in [&mut rx1, &mut rx2] {
        let e1 = rx.recv().await.expect("1");
        let e2 = rx.recv().await.expect("2");
        match (e1, e2) {
            (AgentEvent::Stdout { text: a }, AgentEvent::Stdout { text: b }) => {
                assert_eq!(a, "a");
                assert_eq!(b, "b");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

#[tokio::test]
async fn close_drops_channel() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    let mut rx = bc.subscribe(&id);
    bc.close(&id);
    let res = rx.recv().await;
    assert!(res.is_err(), "expected closed channel, got {res:?}");
}

#[tokio::test]
async fn send_to_unknown_attempt_is_no_op() {
    let bc = EventBroadcaster::new(16);
    let id = AttemptId::new();
    bc.send(
        &id,
        AgentEvent::Stdout {
            text: "lost".into(),
        },
    );
    let mut rx = bc.subscribe(&id);
    bc.send(
        &id,
        AgentEvent::Stdout {
            text: "kept".into(),
        },
    );
    let evt = rx.recv().await.expect("recv");
    match evt {
        AgentEvent::Stdout { text } => assert_eq!(text, "kept"),
        other => panic!("unexpected: {other:?}"),
    }
}
