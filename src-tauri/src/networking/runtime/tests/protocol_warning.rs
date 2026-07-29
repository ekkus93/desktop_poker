use std::time::{Duration, Instant};

use crate::{
    crypto::DefaultCryptoProvider,
    networking::ClientRuntimeEvent,
    protocol::{ProtocolMessageType, SignedEnvelope, TournamentStartedEvent, PROTOCOL_VERSION},
};

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

#[test]
fn bad_public_signature_emits_protocol_warning_and_runtime_continues() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-protocol-warning-signature", 63);
    let client = connect_test_client(&provider, &host, "player-pw-signature", "Signature");
    let _ = expect_snapshot_event(&client);
    let bad_event = TournamentStartedEvent {
        tournament_name: "Tampered".to_string(),
        starting_stack: 1500,
        blind_schedule_preset: "FAST".to_string(),
        frozen_player_ids: vec!["player-pw-signature".to_string()],
    };
    let bad_envelope = SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::TournamentStartedEvent,
        table_id: "table-protocol-warning-signature".to_string(),
        session_epoch: 63,
        sender_id: "host".to_string(),
        counter: 1,
        message_id: "tampered-public-event".to_string(),
        server_sequence: Some(2),
        payload: serde_json::to_value(bad_event).expect("bad event payload serializes"),
        signature: Some("invalid-signature".to_string()),
    };

    send_frame_to_client(
        &host,
        "player-pw-signature",
        &serde_json::to_value(bad_envelope).expect("bad envelope serializes"),
    );

    match client
        .next_event(Duration::from_secs(2))
        .expect("protocol warning")
    {
        ClientRuntimeEvent::ProtocolWarning {
            player_id,
            reason,
            count,
        } => {
            assert_eq!(player_id, "player-pw-signature");
            assert_eq!(reason, "public envelope signature verification failed");
            assert_eq!(count, 1);
        }
        other => panic!("expected ProtocolWarning, got {other:?}"),
    }

    host.broadcast_public_event(
        ProtocolMessageType::TournamentStartedEvent,
        &TournamentStartedEvent {
            tournament_name: "Valid Followup".to_string(),
            starting_stack: 1500,
            blind_schedule_preset: "FAST".to_string(),
            frozen_player_ids: vec!["player-pw-signature".to_string()],
        },
    )
    .expect("valid followup event sends");
    let payload = assert_public_event(&client, ProtocolMessageType::TournamentStartedEvent);
    assert_eq!(
        payload.get("tournamentName"),
        Some(&serde_json::json!("Valid Followup"))
    );
}
