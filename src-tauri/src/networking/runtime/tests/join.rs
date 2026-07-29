use std::sync::{atomic::AtomicU64, Arc, Mutex};

use super::super::{handle_initial_client_request, handle_join_request};
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{
        ConnectionState, ParticipantState, SeatOccupancyState, SeatState, TournamentPhase,
        TournamentSeatState,
    },
    protocol::{JsonSignedEnvelope, ProtocolMessageType, PROTOCOL_VERSION},
};

use super::support::*;

#[test]
fn join_requests_are_rejected_after_roster_freeze() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-join-ready-check",
        82,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-join-ready-check", 82);
    state.phase = TournamentPhase::ReadyCheck;
    let authoritative_state = Arc::new(Mutex::new(state));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();

    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &player_signing_keys,
            &player_encryption_keys,
            &join_payload,
            "player-ready-check",
            "Ready Check",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("join should fail")
        .to_string()
        .contains("roster freeze"));
}

#[test]
fn joins_allow_up_to_capacity_then_reject_max_plus_one_even_with_open_seats() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-max-capacity",
        87,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-max-capacity", 87);
    state.config.max_players = 2;
    state.seats = (0..4)
        .map(|seat_index| SeatState {
            seat_index,
            occupancy: SeatOccupancyState::Empty,
            tournament_state: TournamentSeatState::Open,
            participant_id: None,
            display_name: None,
            chip_count: None,
            is_ready: false,
            marker: None,
        })
        .collect();
    let authoritative_state = Arc::new(Mutex::new(state));
    let first_signing_keys = provider.generate_signing_keypair();
    let first_encryption_keys = provider.generate_encryption_keypair();
    let second_signing_keys = provider.generate_signing_keypair();
    let second_encryption_keys = provider.generate_encryption_keypair();
    let third_signing_keys = provider.generate_signing_keypair();
    let third_encryption_keys = provider.generate_encryption_keypair();

    let first = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &first_signing_keys,
            &first_encryption_keys,
            &join_payload,
            "player-one",
            "One",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );
    let second = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &second_signing_keys,
            &second_encryption_keys,
            &join_payload,
            "player-two",
            "Two",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );
    let third = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &third_signing_keys,
            &third_encryption_keys,
            &join_payload,
            "player-three",
            "Three",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(first.is_ok());
    assert!(second.is_ok());
    assert!(third.is_err());
    assert!(third
        .expect_err("join should fail")
        .to_string()
        .contains("table is full"));
}

#[test]
fn admitted_unseated_participants_count_toward_join_capacity() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload =
        sample_join_payload_for_tests("table-capacity", 83, host_signing_keys.public_key_base64());
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-capacity", 83);
    state.config.max_players = 2;
    let host_keys = provider.generate_signing_keypair();
    let waiting_keys = provider.generate_signing_keypair();
    state.participants.insert(
        "host".to_string(),
        sample_participant_entry(
            "host",
            "Host",
            host_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(0),
            "token-host",
        ),
    );
    state.participants.insert(
        "waiting".to_string(),
        sample_participant_entry(
            "waiting",
            "Waiting",
            waiting_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Admitted,
            ConnectionState::Disconnected,
            None,
            "token-waiting",
        ),
    );
    state.seats = vec![
        SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("host".to_string()),
            display_name: Some("Host".to_string()),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        },
        SeatState {
            seat_index: 1,
            occupancy: SeatOccupancyState::Empty,
            tournament_state: TournamentSeatState::Open,
            participant_id: None,
            display_name: None,
            chip_count: None,
            is_ready: false,
            marker: None,
        },
    ];
    let authoritative_state = Arc::new(Mutex::new(state));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();

    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &player_signing_keys,
            &player_encryption_keys,
            &join_payload,
            "player-over-capacity",
            "Capacity",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("join should fail")
        .to_string()
        .contains("table is full"));
}

#[test]
fn reconnect_eligible_disconnected_participants_count_toward_join_capacity() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-reconnect-capacity",
        86,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-reconnect-capacity", 86);
    state.config.max_players = 2;
    let host_keys = provider.generate_signing_keypair();
    let reconnecting_keys = provider.generate_signing_keypair();
    state.participants.insert(
        "host".to_string(),
        sample_participant_entry(
            "host",
            "Host",
            host_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(0),
            "token-host",
        ),
    );
    state.participants.insert(
        "reconnecting".to_string(),
        sample_participant_entry(
            "reconnecting",
            "Reconnect",
            reconnecting_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Reconnecting,
            ConnectionState::Disconnected,
            Some(1),
            "token-reconnecting",
        ),
    );
    let authoritative_state = Arc::new(Mutex::new(state));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();

    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &player_signing_keys,
            &player_encryption_keys,
            &join_payload,
            "player-after-reconnect-slot",
            "Reconnect Slot",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("join should fail")
        .to_string()
        .contains("table is full"));
}

#[test]
fn eliminated_observers_do_not_block_join_capacity() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-observer-capacity",
        84,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let mut state = sample_tournament_state("table-observer-capacity", 84);
    state.config.max_players = 2;
    let host_keys = provider.generate_signing_keypair();
    let observer_keys = provider.generate_signing_keypair();
    state.participants.insert(
        "host".to_string(),
        sample_participant_entry(
            "host",
            "Host",
            host_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(0),
            "token-host",
        ),
    );
    state.participants.insert(
        "observer".to_string(),
        sample_participant_entry(
            "observer",
            "Observer",
            observer_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::EliminatedObserver,
            ConnectionState::Connected,
            Some(1),
            "token-observer",
        ),
    );
    state.seats = vec![
        SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("host".to_string()),
            display_name: Some("Host".to_string()),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        },
        SeatState {
            seat_index: 1,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::EliminatedObserver,
            participant_id: Some("observer".to_string()),
            display_name: Some("Observer".to_string()),
            chip_count: Some(0),
            is_ready: true,
            marker: None,
        },
    ];
    let authoritative_state = Arc::new(Mutex::new(state));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();

    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &player_signing_keys,
            &player_encryption_keys,
            &join_payload,
            "player-new",
            "New Player",
            &join_payload.join_token,
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_ok());
}

#[test]
fn join_requests_reject_the_wrong_join_token() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let join_payload = sample_join_payload_for_tests(
        "table-wrong-token",
        85,
        host_signing_keys.public_key_base64(),
    );
    let server_sequence = Arc::new(AtomicU64::new(0));
    let authoritative_state =
        Arc::new(Mutex::new(sample_tournament_state("table-wrong-token", 85)));
    let player_signing_keys = provider.generate_signing_keypair();
    let player_encryption_keys = provider.generate_encryption_keypair();

    let result = handle_join_request(
        &provider,
        signed_join_envelope(
            &provider,
            &player_signing_keys,
            &player_encryption_keys,
            &join_payload,
            "player-wrong-token",
            "Wrong Token",
            "not-the-live-token",
        ),
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
    );

    assert!(result.is_err());
    assert!(result
        .expect_err("join should fail")
        .to_string()
        .contains("join token mismatch"));
}

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
