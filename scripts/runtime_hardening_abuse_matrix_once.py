from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    text = file_path.read_text(encoding="utf-8")
    if old not in text:
        if new in text:
            return
        raise SystemExit(f"expected text not found in {path}: {old[:120]!r}")
    file_path.write_text(text.replace(old, new, 1), encoding="utf-8")


replace_once(
    "src-tauri/src/networking/runtime/tests/join.rs",
    "use super::super::handle_join_request;",
    "use super::super::{handle_initial_client_request, handle_join_request};",
)
replace_once(
    "src-tauri/src/networking/runtime/tests/join.rs",
    """    domain::{
        ConnectionState, ParticipantState, SeatOccupancyState, SeatState, TournamentPhase,
        TournamentSeatState,
    },
};""",
    """    domain::{
        ConnectionState, ParticipantState, SeatOccupancyState, SeatState, TournamentPhase,
        TournamentSeatState,
    },
    protocol::{JsonSignedEnvelope, ProtocolMessageType, PROTOCOL_VERSION},
};""",
)

join_path = Path("src-tauri/src/networking/runtime/tests/join.rs")
join_text = join_path.read_text(encoding="utf-8")
join_tests = r'''

#[test]
fn initial_request_rejects_wrong_message_type() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-wrong-initial-message",
        88,
        host_signing_keys.public_key_base64(),
    );
    let authoritative_state = Arc::new(Mutex::new(sample_tournament_state(
        "table-wrong-initial-message",
        88,
    )));
    let server_sequence = Arc::new(AtomicU64::new(0));
    let envelope = JsonSignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::SeatClaimRequest,
        table_id: join_payload.table_id.clone(),
        session_epoch: join_payload.session_epoch,
        sender_id: "wrong-initial-player".to_string(),
        counter: 1,
        message_id: "wrong-initial-message".to_string(),
        server_sequence: None,
        payload: serde_json::json!({ "seatIndex": 0 }),
        signature: None,
    };

    let error = handle_initial_client_request(
        &provider,
        envelope,
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect_err("non-join initial request must be rejected");

    assert!(error
        .to_string()
        .contains("first client message must be JOIN_TOURNAMENT_REQUEST"));
    assert!(authoritative_state
        .lock()
        .expect("authoritative state")
        .participants
        .is_empty());
}

#[test]
fn duplicate_player_id_join_is_rejected_without_replacing_identity() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-duplicate-player",
        89,
        host_signing_keys.public_key_base64(),
    );
    let authoritative_state = Arc::new(Mutex::new(sample_tournament_state(
        "table-duplicate-player",
        89,
    )));
    let server_sequence = Arc::new(AtomicU64::new(0));
    let original_signing = provider.generate_signing_keypair();
    let original_encryption = provider.generate_encryption_keypair();
    let replacement_signing = provider.generate_signing_keypair();
    let replacement_encryption = provider.generate_encryption_keypair();
    let original_public_key = original_signing.public_key_base64();

    handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &original_signing,
            &original_encryption,
            &join_payload,
            "duplicate-player",
            "Original",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect("first join succeeds");

    let error = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &replacement_signing,
            &replacement_encryption,
            &join_payload,
            "duplicate-player",
            "Replacement",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect_err("duplicate player ID must be rejected");

    assert!(error.to_string().contains("playerId already exists"));
    let state = authoritative_state.lock().expect("authoritative state");
    assert_eq!(state.participants.len(), 1);
    let participant = state
        .participants
        .get("duplicate-player")
        .expect("original participant remains");
    assert_eq!(participant.identity.display_name, "Original");
    assert_eq!(participant.identity.signing_public_key, original_public_key);
}
'''
if "initial_request_rejects_wrong_message_type" not in join_text:
    join_path.write_text(join_text + join_tests, encoding="utf-8")

replace_once(
    "src-tauri/src/networking/runtime/tests/session.rs",
    """use std::{
    sync::{Arc, Mutex},
    time::Duration,
};""",
    """use std::{
    net::TcpStream,
    sync::{Arc, Mutex},
    time::Duration,
};""",
)
replace_once(
    "src-tauri/src/networking/runtime/tests/session.rs",
    """    networking::{
        ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent, HostRuntimeConfig, HostRuntimeMode,
        HostServer,
    },
    protocol::{PrivateHoleCardsEvent, ProtocolMessageType, TournamentStartedEvent},""",
    """    networking::{
        read_json_frame, write_json_frame, ClientRuntime, ClientRuntimeConfig, ClientRuntimeEvent,
        HostRuntimeConfig, HostRuntimeMode, HostServer,
    },
    protocol::{
        PrivateHoleCardsEvent, ProtocolErrorMessage, ProtocolMessageType, SignedEnvelope,
        SnapshotEvent, TournamentStartedEvent,
    },""",
)

session_path = Path("src-tauri/src/networking/runtime/tests/session.rs")
session_text = session_path.read_text(encoding="utf-8")
session_test = r'''

#[test]
fn connected_session_rejects_unsupported_request_with_protocol_error() {
    let provider = DefaultCryptoProvider;
    let host = bind_test_host(&provider, "table-unsupported-connected", 90);
    let signing_keys = provider.generate_signing_keypair();
    let encryption_keys = provider.generate_encryption_keypair();
    let mut stream = TcpStream::connect(host.listener_addr()).expect("connect raw client");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .expect("set raw client timeout");
    let join_payload = host.join_payload().clone();
    let join_envelope = signed_join_envelope(
        &provider,
        &signing_keys,
        &encryption_keys,
        &join_payload,
        "unsupported-connected-player",
        "Unsupported",
        &join_payload.join_token,
    );

    write_json_frame(&mut stream, &join_envelope).expect("write initial join");
    let _: SignedEnvelope<SnapshotEvent> =
        read_json_frame(&mut stream).expect("read accepted snapshot");

    let mut unsupported = signed_join_envelope(
        &provider,
        &signing_keys,
        &encryption_keys,
        &join_payload,
        "unsupported-connected-player",
        "Unsupported",
        &join_payload.join_token,
    );
    unsupported.counter = 2;
    unsupported.message_id = "unsupported-post-connect".to_string();
    write_json_frame(&mut stream, &unsupported).expect("write unsupported request");

    let rejection: SignedEnvelope<ProtocolErrorMessage> =
        read_json_frame(&mut stream).expect("read protocol rejection");
    assert_eq!(rejection.message_type, ProtocolMessageType::ProtocolError);
    assert_eq!(rejection.payload.code, "UNSUPPORTED_REQUEST");
    assert_eq!(
        rejection.payload.rejected_message_id.as_deref(),
        Some("unsupported-post-connect")
    );
    assert!(rejection
        .payload
        .message
        .contains("only supports RESYNC_REQUEST"));
}
'''
if "connected_session_rejects_unsupported_request_with_protocol_error" not in session_text:
    session_path.write_text(session_text + session_test, encoding="utf-8")

coverage = Path("docs/runtime-validation/runtime-hardening-abuse-coverage.md")
coverage.write_text(
    """# Runtime hardening abuse-test coverage

This index maps hostile or malformed peer behavior to deterministic tests in the normal Rust suite. Every **Covered** row names an exact runnable test.

| Hostile input or condition | Coverage | Runnable test |
|---|---|---|
| Oversized frame prefix | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Truncated frame body | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Malformed JSON | Covered | `networking::runtime::tests::abuse::host_accept_loop_survives_oversized_truncated_and_malformed_join_frames` |
| Wrong first-message `messageType` | Covered | `networking::runtime::tests::join::initial_request_rejects_wrong_message_type` |
| Invalid join token | Covered | `networking::runtime::tests::join::join_requests_reject_the_wrong_join_token` |
| Duplicate player ID join | Covered | `networking::runtime::tests::join::duplicate_player_id_join_is_rejected_without_replacing_identity` |
| Already-connected reconnect | Covered | `networking::runtime::tests::reconnect::already_connected_reconnect_is_rejected` |
| Reconnect after host-side disconnect | Covered | `networking::runtime::tests::reconnect::reconnect_succeeds_after_host_side_disconnect` |
| Resync after stale server sequence | Covered | `networking::runtime::tests::resync::resync_request_returns_authoritative_snapshot_after_stale_sequence` |
| Unsupported post-connect request | Covered | `networking::runtime::tests::session::connected_session_rejects_unsupported_request_with_protocol_error` |
| Bad public-event signature | Covered | `networking::runtime::tests::protocol_warning::bad_public_signature_emits_protocol_warning_and_runtime_continues` |
| Remote stale action-window ID | Covered | `networking::runtime::tests::action_outcomes::remote_stale_window_rejection_does_not_mutate_state` |
| Remote invalid raise amount | Covered | `networking::runtime::tests::action_outcomes::remote_invalid_raise_rejection_does_not_mutate_state` |

## Explicitly deferred

- Physical-LAN packet loss, router isolation, and cross-device firewall behavior remain release/manual validation concerns rather than deterministic protocol-unit tests.
- Sustained distributed resource-exhaustion/load testing remains outside normal CI; configured connection limits and counters are covered by deterministic unit tests.
""",
    encoding="utf-8",
)
