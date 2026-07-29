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
    "src-tauri/src/networking/runtime/tests/protocol_warning.rs",
    "use crate::{crypto::DefaultCryptoProvider, networking::ClientRuntimeEvent};",
    """use crate::{
    crypto::DefaultCryptoProvider,
    networking::ClientRuntimeEvent,
    protocol::{ProtocolMessageType, SignedEnvelope, TournamentStartedEvent, PROTOCOL_VERSION},
};""",
)

path = Path("src-tauri/src/networking/runtime/tests/protocol_warning.rs")
text = path.read_text(encoding="utf-8")
test = r'''

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
    assert_eq!(payload.get("tournamentName"), Some(&serde_json::json!("Valid Followup")));
}
'''
if "bad_public_signature_emits_protocol_warning_and_runtime_continues" not in text:
    path.write_text(text + test, encoding="utf-8")

coverage = Path("docs/runtime-validation/runtime-hardening-abuse-coverage.md")
text = coverage.read_text(encoding="utf-8")
text = text.replace(
    "`networking::runtime::tests::reconnect::already_connected_reconnect_is_rejected`",
    "`networking::runtime::tests::reconnect::reconnect_rejects_already_connected_participants`",
)
text = text.replace(
    "`networking::runtime::tests::reconnect::reconnect_succeeds_after_host_side_disconnect`",
    "`networking::runtime::tests::reconnect::reconnect_succeeds_only_with_original_keypair_and_valid_token`",
)
text = text.replace(
    "`networking::runtime::tests::resync::resync_request_returns_authoritative_snapshot_after_stale_sequence`",
    "`networking::runtime::tests::misc::resync_after_a_sequence_gap_allows_followup_public_events_to_continue`",
)
coverage.write_text(text, encoding="utf-8")
