use std::sync::{atomic::AtomicU64, Arc, Mutex};

use super::super::handle_reconnect_request;
use super::support::*;
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{ConnectionState, ParticipantState},
    protocol::{test_support::sample_reconnect_request, ReconnectTournamentRequest},
};

#[test]
fn reconnect_rejects_expired_authority_without_restoring_participant_state() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-expired",
        32,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-reconnect-expired", 32);
    let mut participant = sample_participant_entry(
        "player-expired",
        "Expired",
        player_signing_keys.public_key_base64(),
        player_encryption_keys.public_key_base64(),
        ParticipantState::Reconnecting,
        ConnectionState::Reconnecting,
        Some(0),
        "expected-token",
    );
    participant.reconnect_expiry_ms = Some(0);
    state
        .participants
        .insert("player-expired".to_string(), participant);
    let authoritative_state = Arc::new(Mutex::new(state));

    let error = handle_reconnect_request(
        &provider,
        signed_reconnect_envelope(
            &provider,
            &player_signing_keys,
            &join_payload,
            ReconnectTournamentRequest {
                player_id: "player-expired".to_string(),
                reconnect_token: "expected-token".to_string(),
                ..sample_reconnect_request(Some(1))
            },
            1,
            "reconnect-expired",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    )
    .expect_err("expired reconnect authority must be rejected");

    assert!(error.to_string().contains("reconnect token expired"));

    let state = authoritative_state
        .lock()
        .expect("authoritative state should remain readable");
    let participant = state
        .participants
        .get("player-expired")
        .expect("expired participant should remain registered");
    assert_eq!(participant.state, ParticipantState::Reconnecting);
    assert_eq!(participant.connection_state, ConnectionState::Reconnecting);
    assert_eq!(
        participant.reconnect_token.as_deref(),
        Some("expected-token")
    );
    assert_eq!(participant.reconnect_expiry_ms, Some(0));
    assert_eq!(server_sequence.load(std::sync::atomic::Ordering::SeqCst), 0);
}
