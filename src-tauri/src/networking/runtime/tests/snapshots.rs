use std::{
    collections::BTreeMap,
    sync::{atomic::AtomicU64, Arc, Mutex},
};

use super::super::build_snapshot_envelope;
use crate::{
    crypto::{DefaultCryptoProvider, ProtocolCryptoProvider},
    domain::{
        ConnectionState, HandCyclePhase, ParticipantState, SeatOccupancyState, SeatState,
        TournamentPhase, TournamentSeatState,
    },
};

use super::support::*;

#[test]
fn recipient_snapshots_strip_other_players_private_data_from_serialized_json() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let server_sequence = Arc::new(AtomicU64::new(0));
    let join_payload = sample_join_payload_for_tests(
        "table-private-snapshot",
        80,
        host_signing_keys.public_key_base64(),
    );
    let mut state = sample_tournament_state("table-private-snapshot", 80);
    let player_a_keys = provider.generate_signing_keypair();
    let player_b_keys = provider.generate_signing_keypair();
    state.phase = TournamentPhase::Running;
    state.participants.insert(
        "player-a".to_string(),
        sample_participant_entry(
            "player-a",
            "Alice",
            player_a_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(0),
            "token-a",
        ),
    );
    state.participants.insert(
        "player-b".to_string(),
        sample_participant_entry(
            "player-b",
            "Bob",
            player_b_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(1),
            "token-b",
        ),
    );
    state.seats = vec![
        SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("player-a".to_string()),
            display_name: Some("Alice".to_string()),
            chip_count: Some(1200),
            is_ready: true,
            marker: None,
        },
        SeatState {
            seat_index: 1,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("player-b".to_string()),
            display_name: Some("Bob".to_string()),
            chip_count: Some(1300),
            is_ready: true,
            marker: None,
        },
    ];
    state.current_hand = Some(crate::domain::HandState {
        hand_number: 4,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: crate::domain::StreetPhase::Turn,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: Vec::new(),
        hole_cards_by_player_id: [
            (
                "player-a".to_string(),
                vec![
                    crate::domain::Card {
                        rank: crate::domain::Rank::Ace,
                        suit: crate::domain::Suit::Clubs,
                    },
                    crate::domain::Card {
                        rank: crate::domain::Rank::Ace,
                        suit: crate::domain::Suit::Hearts,
                    },
                ],
            ),
            (
                "player-b".to_string(),
                vec![
                    crate::domain::Card {
                        rank: crate::domain::Rank::Jack,
                        suit: crate::domain::Suit::Diamonds,
                    },
                    crate::domain::Card {
                        rank: crate::domain::Rank::Nine,
                        suit: crate::domain::Suit::Spades,
                    },
                ],
            ),
        ]
        .into_iter()
        .collect(),
        participation_by_player_id: [
            (
                "player-a".to_string(),
                crate::domain::HandParticipationState::Active,
            ),
            (
                "player-b".to_string(),
                crate::domain::HandParticipationState::Active,
            ),
        ]
        .into_iter()
        .collect(),
        betting_round: crate::domain::BettingRoundState {
            street: crate::domain::StreetPhase::Turn,
            current_bet: 40,
            min_raise_to: Some(80),
            max_raise_to: Some(200),
            pot_size: 120,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    });
    let authoritative_state = Arc::new(Mutex::new(state));

    let snapshot = build_snapshot_envelope(
        &provider,
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
        "player-a",
    )
    .expect("snapshot envelope");

    let json = serde_json::to_string(&snapshot).expect("snapshot json");
    assert!(!json.contains("token-b"));
    assert!(!json.contains("\"rank\":\"JACK\""));
    assert_eq!(snapshot.payload.private_hole_cards.len(), 2);
    assert!(snapshot
        .payload
        .state
        .current_hand
        .as_ref()
        .is_some_and(|hand| hand.public_hole_cards_by_player_id.is_empty()));
}

#[test]
fn observer_snapshot_contains_no_private_cards() {
    let provider = DefaultCryptoProvider;
    let host_signing_keys = provider.generate_signing_keypair();
    let host_encryption_keys = Arc::new(Mutex::new(provider.generate_encryption_keypair()));
    let server_sequence = Arc::new(AtomicU64::new(0));
    let join_payload = sample_join_payload_for_tests(
        "table-observer-snapshot",
        81,
        host_signing_keys.public_key_base64(),
    );
    let mut state = sample_tournament_state("table-observer-snapshot", 81);
    let observer_keys = provider.generate_signing_keypair();
    let active_keys = provider.generate_signing_keypair();
    state.phase = TournamentPhase::Running;
    state.participants.insert(
        "observer".to_string(),
        sample_participant_entry(
            "observer",
            "Observer",
            observer_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::EliminatedObserver,
            ConnectionState::Connected,
            Some(0),
            "token-observer",
        ),
    );
    state.participants.insert(
        "active".to_string(),
        sample_participant_entry(
            "active",
            "Active",
            active_keys.public_key_base64(),
            provider.generate_encryption_keypair().public_key_base64(),
            ParticipantState::Active,
            ConnectionState::Connected,
            Some(1),
            "token-active",
        ),
    );
    state.seats = vec![
        SeatState {
            seat_index: 0,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::EliminatedObserver,
            participant_id: Some("observer".to_string()),
            display_name: Some("Observer".to_string()),
            chip_count: Some(0),
            is_ready: true,
            marker: None,
        },
        SeatState {
            seat_index: 1,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: TournamentSeatState::Active,
            participant_id: Some("active".to_string()),
            display_name: Some("Active".to_string()),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        },
    ];
    state.current_hand = Some(crate::domain::HandState {
        hand_number: 1,
        cycle_phase: HandCyclePhase::AwaitingAction,
        street: crate::domain::StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: Vec::new(),
        hole_cards_by_player_id: [(
            "active".to_string(),
            vec![
                crate::domain::Card {
                    rank: crate::domain::Rank::King,
                    suit: crate::domain::Suit::Clubs,
                },
                crate::domain::Card {
                    rank: crate::domain::Rank::Queen,
                    suit: crate::domain::Suit::Clubs,
                },
            ],
        )]
        .into_iter()
        .collect(),
        participation_by_player_id: [(
            "active".to_string(),
            crate::domain::HandParticipationState::Active,
        )]
        .into_iter()
        .collect(),
        betting_round: crate::domain::BettingRoundState {
            street: crate::domain::StreetPhase::Preflop,
            current_bet: 20,
            min_raise_to: Some(40),
            max_raise_to: None,
            pot_size: 30,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    });
    let authoritative_state = Arc::new(Mutex::new(state));

    let snapshot = build_snapshot_envelope(
        &provider,
        &join_payload,
        &authoritative_state,
        &server_sequence,
        &host_signing_keys,
        &host_encryption_keys,
        "observer",
    )
    .expect("observer snapshot envelope");

    assert!(snapshot.payload.private_hole_cards.is_empty());
    assert!(snapshot
        .payload
        .state
        .current_hand
        .as_ref()
        .is_some_and(|hand| hand.public_hole_cards_by_player_id.is_empty()));
}
