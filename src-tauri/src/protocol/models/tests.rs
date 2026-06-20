use serde_json::json;

use super::{
    canonical_json_bytes_without_signature, ActionWindowOpened, EncryptedPrivateEnvelope,
    PlayerActionCommitted, PrivateEnvelopeMetadata, ProtocolMessageType,
    ReconnectTournamentRequest, SignedEnvelope,
};
use crate::{
    crypto::{DefaultCryptoProvider, EncryptedPayload, ProtocolCryptoProvider},
    domain::ActionType,
    protocol::PROTOCOL_VERSION,
};

fn sample_signed_envelope() -> SignedEnvelope<serde_json::Value> {
    SignedEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::SnapshotEvent,
        table_id: "table-1".to_string(),
        session_epoch: 4,
        sender_id: "host".to_string(),
        counter: 7,
        message_id: "message-1".to_string(),
        server_sequence: None,
        payload: json!({ "value": 7 }),
        signature: None,
    }
}

fn sample_private_envelope() -> EncryptedPrivateEnvelope {
    EncryptedPrivateEnvelope {
        protocol_version: PROTOCOL_VERSION,
        message_type: ProtocolMessageType::PrivateHoleCardsEvent,
        table_id: "table-1".to_string(),
        session_epoch: 4,
        sender_id: "host".to_string(),
        counter: 8,
        message_id: "private-1".to_string(),
        server_sequence: 9,
        recipient_id: "player-1".to_string(),
        recipient_key_id: "recipient-key".to_string(),
        nonce: "nonce".to_string(),
        ciphertext: "ciphertext".to_string(),
        signature: None,
    }
}

#[test]
fn signed_envelope_sign_populates_signature_and_verify_checks_bytes() {
    let provider = DefaultCryptoProvider;
    let signing_keys = provider.generate_signing_keypair();
    let verifying_key = signing_keys.public_key_base64();
    let mut envelope = sample_signed_envelope();

    envelope
        .sign(&provider, &signing_keys)
        .expect("sign envelope");

    assert!(envelope.signature.is_some());
    envelope
        .verify(&provider, &verifying_key)
        .expect("signature should verify");

    let mut mutated = envelope.clone();
    mutated.payload = json!({ "value": 8 });
    assert!(mutated.verify(&provider, &verifying_key).is_err());
}

#[test]
fn signed_envelope_verify_fails_when_signature_is_missing() {
    let provider = DefaultCryptoProvider;
    let signing_keys = provider.generate_signing_keypair();

    let error = sample_signed_envelope()
        .verify(&provider, &signing_keys.public_key_base64())
        .expect_err("missing signature should fail");

    assert_eq!(error.to_string(), "signed envelope missing signature");
}

#[test]
fn signed_envelope_canonical_bytes_omit_none_server_sequence() {
    let bytes =
        canonical_json_bytes_without_signature(&sample_signed_envelope()).expect("canonical bytes");
    let json = String::from_utf8(bytes).expect("utf8");

    assert!(!json.contains("serverSequence"));
}

#[test]
fn encrypted_private_envelope_associated_data_contains_all_authenticated_fields() {
    let json = String::from_utf8(
        sample_private_envelope()
            .associated_data_json()
            .expect("associated data"),
    )
    .expect("utf8");

    assert_eq!(
        json,
        r#"{"counter":8,"messageId":"private-1","messageType":"PRIVATE_HOLE_CARDS_EVENT","protocolVersion":1,"recipientId":"player-1","recipientKeyId":"recipient-key","senderId":"host","serverSequence":9,"sessionEpoch":4,"tableId":"table-1"}"#,
    );
}

#[test]
fn encrypted_private_envelope_from_payload_copies_metadata_exactly() {
    let envelope = EncryptedPrivateEnvelope::from_encrypted_payload(
        EncryptedPayload {
            nonce_base64: "nonce-123".to_string(),
            ciphertext_base64: "ciphertext-456".to_string(),
            recipient_key_id: "recipient-key".to_string(),
        },
        PrivateEnvelopeMetadata {
            sender_id: "host".to_string(),
            table_id: "table-1".to_string(),
            session_epoch: 4,
            counter: 8,
            message_id: "private-1".to_string(),
            server_sequence: 9,
            recipient_id: "player-1".to_string(),
        },
    );

    assert_eq!(
        envelope.message_type,
        ProtocolMessageType::PrivateHoleCardsEvent
    );
    assert_eq!(envelope.recipient_id, "player-1");
    assert_eq!(envelope.recipient_key_id, "recipient-key");
    assert_eq!(envelope.nonce, "nonce-123");
    assert_eq!(envelope.ciphertext, "ciphertext-456");
}

#[test]
fn encrypted_private_envelope_sign_and_verify_track_authenticated_metadata() {
    let provider = DefaultCryptoProvider;
    let signing_keys = provider.generate_signing_keypair();
    let verifying_key = signing_keys.public_key_base64();
    let mut envelope = sample_private_envelope();

    envelope
        .sign(&provider, &signing_keys)
        .expect("sign envelope");

    assert!(envelope.signature.is_some());
    envelope
        .verify(&provider, &verifying_key)
        .expect("signature should verify");

    let mut mutated = envelope.clone();
    mutated.recipient_id = "player-2".to_string();
    assert!(mutated.verify(&provider, &verifying_key).is_err());

    let missing_signature_error = sample_private_envelope()
        .verify(&provider, &verifying_key)
        .expect_err("missing signature should fail");
    assert_eq!(
        missing_signature_error.to_string(),
        "private envelope missing signature"
    );
}

#[test]
fn reconnect_request_serialization_omits_or_keeps_last_known_sequence_as_expected() {
    let without_sequence = serde_json::to_value(ReconnectTournamentRequest {
        player_id: "player-1".to_string(),
        reconnect_token: "token".to_string(),
        last_known_server_seq: None,
    })
    .expect("serialize reconnect request");
    let with_sequence = serde_json::to_value(ReconnectTournamentRequest {
        player_id: "player-1".to_string(),
        reconnect_token: "token".to_string(),
        last_known_server_seq: Some(44),
    })
    .expect("serialize reconnect request");

    assert_eq!(
        without_sequence,
        json!({
            "playerId": "player-1",
            "reconnectToken": "token"
        }),
    );
    assert_eq!(with_sequence.get("lastKnownServerSeq"), Some(&json!(44)));
}

#[test]
fn action_window_opened_serialization_omits_null_raise_bounds_and_keeps_action_strings() {
    let payload = serde_json::to_value(ActionWindowOpened {
        hand_number: 12,
        hand_phase: "TURN".to_string(),
        action_window_id: "window-1".to_string(),
        player_id: "player-1".to_string(),
        seat_index: 3,
        legal_actions: vec![ActionType::Fold, ActionType::Raise, ActionType::AllIn],
        call_amount: 40,
        min_raise_to: None,
        max_raise_to: None,
        deadline_epoch_ms: 123_456,
    })
    .expect("serialize action window");

    assert_eq!(payload.get("minRaiseTo"), None);
    assert_eq!(payload.get("maxRaiseTo"), None);
    assert_eq!(
        payload.get("legalActions"),
        Some(&json!(["FOLD", "RAISE", "ALL_IN"])),
    );
}

#[test]
fn player_action_committed_serialization_preserves_raise_amount_contract() {
    let with_raise = serde_json::to_value(PlayerActionCommitted {
        hand_number: 12,
        seat_index: 3,
        player_id: "player-1".to_string(),
        action_type: ActionType::Raise,
        raise_to_amount: Some(180),
    })
    .expect("serialize action with raise");
    let without_raise = serde_json::to_value(PlayerActionCommitted {
        hand_number: 12,
        seat_index: 3,
        player_id: "player-1".to_string(),
        action_type: ActionType::Call,
        raise_to_amount: None,
    })
    .expect("serialize action without raise");

    assert_eq!(with_raise.get("raiseToAmount"), Some(&json!(180)));
    assert_eq!(
        without_raise.get("raiseToAmount"),
        Some(&serde_json::Value::Null)
    );
}

// ---------------------------------------------------------------------------
// Section 11 — canonical JSON round-trip (protocol layer)
// ---------------------------------------------------------------------------

/// Byte-for-byte Android compatibility test.
///
/// TODO: Capture a real fixture by running the Android canonical-JSON test
/// suite and paste it here as the `expected_bytes` literal.  Until then this
/// test is `#[ignore]`d so it does not create a false passing signal.
#[test]
#[ignore = "requires captured Android CanonicalJson fixture — see INT_TEST3_TODO.md §11.1"]
fn canonical_bytes_for_join_request_match_known_android_fixture() {
    // Construct the envelope exactly as the Android test fixture defines it.
    // Replace the values below with the real fixture once captured.
    let envelope = sample_signed_envelope();
    let bytes = canonical_json_bytes_without_signature(&envelope).expect("canonical bytes");

    let expected_json: &str = "REPLACE_WITH_ANDROID_FIXTURE";
    assert_eq!(
        String::from_utf8(bytes).expect("utf8"),
        expected_json,
        "canonical bytes must match the Android CanonicalJson output byte-for-byte"
    );
}

/// Verifies that `None`-valued optional fields on a real protocol struct are
/// omitted from canonical output (not serialised as `"key":null`).
#[test]
fn null_optional_fields_are_omitted_from_canonical_output() {
    // `server_sequence: None` is an optional field on `SignedEnvelope`.
    let envelope = sample_signed_envelope(); // server_sequence is None
    let bytes = canonical_json_bytes_without_signature(&envelope).expect("canonical bytes");
    let json = String::from_utf8(bytes).expect("utf8");

    assert!(
        !json.contains("null"),
        "no `null` values should appear in canonical output; got: {json}"
    );
    assert!(
        !json.contains("serverSequence"),
        "the `serverSequence` field should be omitted when None; got: {json}"
    );
}

#[test]
fn protocol_message_type_strings_stay_stable_for_high_risk_variants() {
    assert_eq!(
        serde_json::to_string(&ProtocolMessageType::ReconnectTournamentRequest)
            .expect("serialize message type"),
        r#""RECONNECT_TOURNAMENT_REQUEST""#,
    );
    assert_eq!(
        serde_json::to_string(&ProtocolMessageType::ActionWindowOpenedEvent)
            .expect("serialize message type"),
        r#""ACTION_WINDOW_OPENED_EVENT""#,
    );
    assert_eq!(
        serde_json::to_string(&ProtocolMessageType::PrivateHoleCardsEvent)
            .expect("serialize message type"),
        r#""PRIVATE_HOLE_CARDS_EVENT""#,
    );
}
