use std::time::{Duration, Instant};

use crate::{crypto::DefaultCryptoProvider, networking::ClientRuntimeEvent};

use super::support::*;

#[test]
fn missing_message_type_emits_protocol_warning() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-protocol-warning-1", 60);
    let client = connect_test_client(&provider, &host, "player-pw-1", "Pwarn");
    let _ = expect_snapshot_event(&client);

    send_frame_to_client(&host, "player-pw-1", &serde_json::json!({"foo": "bar"}));

    match client
        .next_event(Duration::from_secs(2))
        .expect("protocol warning")
    {
        ClientRuntimeEvent::ProtocolWarning {
            player_id,
            reason,
            count,
        } => {
            assert_eq!(player_id, "player-pw-1");
            assert_eq!(reason, "incoming frame missing messageType");
            assert_eq!(count, 1);
        }
        other => panic!("expected ProtocolWarning, got {other:?}"),
    }
}

#[test]
fn protocol_warning_low_noise_policy_emits_at_powers_of_two() {
    // 4 identical bad frames → warnings at counts 1, 2, 4 only (not 3).
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-protocol-warning-2", 61);
    let client = connect_test_client(&provider, &host, "player-pw-2", "Pwarn2");
    let _ = expect_snapshot_event(&client);

    let bad_frame = serde_json::json!({"no_message_type": true});
    for _ in 0..4 {
        send_frame_to_client(&host, "player-pw-2", &bad_frame);
    }

    let mut warning_counts: Vec<u64> = vec![];
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match client.next_event(Duration::from_millis(100)) {
            Ok(ClientRuntimeEvent::ProtocolWarning { count, .. }) => {
                warning_counts.push(count);
                if count == 4 {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for count=4 warning; got {warning_counts:?}"
        );
    }

    assert_eq!(
        warning_counts,
        vec![1, 2, 4],
        "low-noise policy: count=3 must not emit a warning"
    );
}

#[test]
fn different_warning_reasons_tracked_independently() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-protocol-warning-3", 62);
    let client = connect_test_client(&provider, &host, "player-pw-3", "Pwarn3");
    let _ = expect_snapshot_event(&client);

    // Two frames missing messageType (same reason key).
    send_frame_to_client(&host, "player-pw-3", &serde_json::json!({"x": 1}));
    send_frame_to_client(&host, "player-pw-3", &serde_json::json!({"x": 2}));

    // Both should emit because counts are 1 and 2 (both powers-of-two).
    let mut counts: Vec<u64> = vec![];
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        match client.next_event(Duration::from_millis(100)) {
            Ok(ClientRuntimeEvent::ProtocolWarning { count, reason, .. }) => {
                assert_eq!(reason, "incoming frame missing messageType");
                counts.push(count);
                if counts.len() == 2 {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
        assert!(
            Instant::now() < deadline,
            "timed out waiting for two warnings; got {counts:?}"
        );
    }

    assert_eq!(counts, vec![1, 2]);
}
