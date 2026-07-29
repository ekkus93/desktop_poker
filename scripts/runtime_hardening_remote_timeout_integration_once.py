from pathlib import Path

path = Path("src-tauri/src/networking/runtime/tests/end_to_end/basic.rs")
text = path.read_text(encoding="utf-8")

old_import = '''use std::{
    thread,
    time::{Duration, Instant},
};'''
new_import = '''use std::{
    sync::atomic::Ordering,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};'''
if old_import in text:
    text = text.replace(old_import, new_import, 1)
elif new_import not in text:
    raise SystemExit("expected basic end-to-end import block was not found")

old_support = "use super::super::support::*;"
new_support = '''use super::super::support::*;
use super::super::super::merge_networking_state;'''
if old_support in text:
    text = text.replace(old_support, new_support, 1)
elif new_support not in text:
    raise SystemExit("expected support import was not found")

test = r'''

/// A real remote client submits after its action deadline while the background
/// tick loop is deliberately paused. The action session itself must commit and
/// publish the timeout transition before returning the stale-action rejection.
#[test]
fn d_remote_expired_action_publishes_timeout_before_rejecting_submission() {
    let provider = crate::crypto::DefaultCryptoProvider;
    let mut state = sample_tournament_state("table-expired-remote-action", 84);
    state.config.turn_timer_seconds = 1;
    let mut host = bind_test_host_with_state(
        &provider,
        "table-expired-remote-action",
        84,
        state,
    );

    let alice = connect_test_client(&provider, &host, "player-alice", "Alice");
    let bob = connect_test_client(&provider, &host, "player-bob", "Bob");
    let _ = expect_snapshot_event(&alice);
    let _ = expect_snapshot_event(&bob);

    host.claim_seat("player-alice", 0).expect("alice seat");
    host.claim_seat("player-bob", 1).expect("bob seat");
    host.set_ready_state("player-alice", true)
        .expect("alice ready");
    host.set_ready_state("player-bob", true).expect("bob ready");
    host.start_tournament().expect("start");

    let deadline = Instant::now() + Duration::from_secs(3);
    let window = loop {
        if let Some(window) = host
            .authoritative_state()
            .expect("state")
            .current_hand
            .as_ref()
            .and_then(|hand| hand.action_window.clone())
        {
            break window;
        }
        assert!(Instant::now() < deadline, "a hand should open an action window");
        thread::sleep(Duration::from_millis(20));
    };

    let (actor, observer) = if window.player_id == "player-alice" {
        (&alice, &bob)
    } else {
        (&bob, &alice)
    };
    wait_for_client_command_connection(actor);
    wait_for_client_command_connection(observer);

    // Existing client-session threads do not consume this stop signal. Stopping
    // and joining only the tick worker makes the expired-action path
    // deterministic: the remote action handler, not the periodic tick, owns the
    // timeout transition under test.
    host.stop_signal.store(true, Ordering::SeqCst);
    host.tick_thread
        .take()
        .expect("tick worker is present")
        .join()
        .expect("tick worker stops cleanly");

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after epoch")
        .as_millis() as u64;
    let wait_ms = window.deadline_epoch_ms.saturating_sub(now_ms) + 50;
    thread::sleep(Duration::from_millis(wait_ms));

    actor
        .submit_action(
            window.action_window_id.clone(),
            window.seat_index,
            ActionType::Fold,
            None,
        )
        .expect("expired remote action request is written to the host");

    // A different connected client proves the timeout transition was published
    // over TCP rather than merely committed in host memory.
    let _ = wait_for_public_event(observer, ProtocolMessageType::PlayerActionCommittedEvent);

    // The submitting client must see the publication and then the explicit
    // rejection for the stale attempted action.
    let event_deadline = Instant::now() + Duration::from_secs(5);
    let mut saw_timeout_publication = false;
    let mut saw_stale_rejection = false;
    while !(saw_timeout_publication && saw_stale_rejection) {
        match actor.next_event(Duration::from_millis(200)) {
            Ok(crate::networking::ClientRuntimeEvent::PublicEvent { message_type, .. })
                if message_type == ProtocolMessageType::PlayerActionCommittedEvent =>
            {
                saw_timeout_publication = true;
            }
            Ok(crate::networking::ClientRuntimeEvent::SafeError { message, .. }) => {
                assert!(
                    message.contains("stale action window"),
                    "expired action rejection should remain explicit; got: {message}"
                );
                saw_stale_rejection = true;
            }
            Ok(_) => {}
            Err(crate::networking::ClientRuntimePollError::Timeout) => {}
            Err(crate::networking::ClientRuntimePollError::Disconnected) => {
                panic!("client runtime disconnected before publication and rejection")
            }
        }
        assert!(
            Instant::now() < event_deadline,
            "expired remote action should publish and reject before timeout"
        );
    }

    let authoritative = host.authoritative_state().expect("authoritative state after timeout");
    let advanced = authoritative
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.as_ref())
        .map(|next| next.action_window_id != window.action_window_id)
        .unwrap_or(true)
        || !authoritative.hand_results.is_empty();
    assert!(advanced, "the expired action must advance authoritative state");

    let mut normalized_controller_state = host
        .tournament_runtime
        .lock()
        .expect("runtime")
        .as_ref()
        .expect("controller")
        .state()
        .clone();
    merge_networking_state(&authoritative, &mut normalized_controller_state);
    assert_eq!(
        normalized_controller_state, authoritative,
        "controller gameplay and authoritative state must converge after the timeout rejection"
    );
}
'''
if "d_remote_expired_action_publishes_timeout_before_rejecting_submission" not in text:
    text += test

path.write_text(text, encoding="utf-8")
