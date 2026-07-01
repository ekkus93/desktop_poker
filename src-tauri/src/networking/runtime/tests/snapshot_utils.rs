use std::collections::BTreeMap;

use super::super::{build_recipient_snapshot_state, public_revealed_hole_cards};
use crate::domain::{
    BettingRoundState, BlindLevel, BlindSchedule, Card, ConnectionState, HandCyclePhase,
    HandParticipationState, HandState, ParticipantRegistryEntry, ParticipantState, PlayerIdentity,
    Rank, SeatOccupancyState, SeatState, StreetPhase, Suit, TournamentConfig, TournamentPhase,
    TournamentSeatState, TournamentState,
};

// ——— helpers ——————————————————————————————————————————————————

fn sample_config() -> TournamentConfig {
    TournamentConfig {
        tournament_name: "Snapshot Test".to_string(),
        table_name: None,
        max_players: 2,
        starting_stack: 1000,
        turn_timer_seconds: 10,
        blind_schedule: BlindSchedule {
            levels: vec![BlindLevel {
                level_index: 1,
                label: "L1".to_string(),
                small_blind: 10,
                big_blind: 20,
                ante: 0,
                duration_seconds: 60,
            }],
        },
    }
}

fn sample_participant(
    player_id: &str,
    display_name: &str,
    seat_index: Option<u8>,
    signing_key_suffix: &str,
) -> ParticipantRegistryEntry {
    ParticipantRegistryEntry {
        identity: PlayerIdentity {
            player_id: player_id.to_string(),
            display_name: display_name.to_string(),
            signing_public_key: format!("signing-key-{signing_key_suffix}"),
            encryption_public_key: format!("enc-key-{signing_key_suffix}"),
            signing_key_fingerprint: format!("fp-{signing_key_suffix}"),
        },
        state: ParticipantState::Active,
        connection_state: ConnectionState::Connected,
        seat_index,
        admitted_at_ms: 0,
        reconnect_token: None,
        reconnect_expiry_ms: None,
        is_host: false,
    }
}

fn occupied_seat(index: u8, player_id: &str, display_name: &str) -> SeatState {
    SeatState {
        seat_index: index,
        occupancy: SeatOccupancyState::Occupied,
        tournament_state: TournamentSeatState::Active,
        participant_id: Some(player_id.to_string()),
        display_name: Some(display_name.to_string()),
        chip_count: Some(1000),
        is_ready: true,
        marker: None,
    }
}

fn minimal_hand(cycle_phase: HandCyclePhase) -> HandState {
    HandState {
        hand_number: 1,
        cycle_phase,
        street: StreetPhase::Preflop,
        dealer_seat_index: 0,
        small_blind_seat_index: 0,
        big_blind_seat_index: 1,
        board_cards: vec![],
        hole_cards_by_player_id: BTreeMap::new(),
        participation_by_player_id: BTreeMap::new(),
        betting_round: BettingRoundState {
            street: StreetPhase::Preflop,
            current_bet: 0,
            min_raise_to: None,
            max_raise_to: None,
            pot_size: 0,
            contributions_by_player_id: BTreeMap::new(),
        },
        action_window: None,
    }
}

fn two_card_hand() -> Vec<Card> {
    vec![
        Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
        },
        Card {
            rank: Rank::King,
            suit: Suit::Hearts,
        },
    ]
}

// ——— T6.1 — public_revealed_hole_cards pre-Showdown ——————————————

#[test]
fn public_revealed_hole_cards_returns_empty_before_showdown() {
    let mut hand = minimal_hand(HandCyclePhase::AwaitingAction);
    hand.street = StreetPhase::Flop;
    hand.hole_cards_by_player_id
        .insert("p1".to_string(), two_card_hand());
    hand.hole_cards_by_player_id
        .insert("p2".to_string(), two_card_hand());
    hand.participation_by_player_id
        .insert("p1".to_string(), HandParticipationState::Active);
    hand.participation_by_player_id
        .insert("p2".to_string(), HandParticipationState::Active);

    let revealed = public_revealed_hole_cards(&hand);
    assert!(
        revealed.is_empty(),
        "no cards should be revealed before Showdown"
    );
}

// ——— T6.2 — public_revealed_hole_cards at Showdown ————————————

#[test]
fn public_revealed_hole_cards_at_showdown_includes_active_and_all_in_excludes_folded() {
    let mut hand = minimal_hand(HandCyclePhase::Showdown);
    hand.hole_cards_by_player_id
        .insert("active".to_string(), two_card_hand());
    hand.hole_cards_by_player_id
        .insert("folded".to_string(), two_card_hand());
    hand.hole_cards_by_player_id
        .insert("allin".to_string(), two_card_hand());
    hand.participation_by_player_id
        .insert("active".to_string(), HandParticipationState::Active);
    hand.participation_by_player_id
        .insert("folded".to_string(), HandParticipationState::Folded);
    hand.participation_by_player_id
        .insert("allin".to_string(), HandParticipationState::AllIn);

    let revealed = public_revealed_hole_cards(&hand);
    assert!(
        revealed.contains_key("active"),
        "active player should be revealed"
    );
    assert!(
        revealed.contains_key("allin"),
        "all-in player should be revealed"
    );
    assert!(
        !revealed.contains_key("folded"),
        "folded player should not be revealed"
    );
}

#[test]
fn public_revealed_hole_cards_at_settlement_same_logic() {
    let mut hand = minimal_hand(HandCyclePhase::Settlement);
    hand.hole_cards_by_player_id
        .insert("active".to_string(), two_card_hand());
    hand.hole_cards_by_player_id
        .insert("out".to_string(), two_card_hand());
    hand.participation_by_player_id
        .insert("active".to_string(), HandParticipationState::Active);
    hand.participation_by_player_id
        .insert("out".to_string(), HandParticipationState::Out);

    let revealed = public_revealed_hole_cards(&hand);
    assert!(revealed.contains_key("active"));
    assert!(!revealed.contains_key("out"));
}

// ——— T6.3 — build_recipient_snapshot_state private cards ——————

#[test]
fn build_recipient_snapshot_state_returns_private_cards_for_target_only() {
    let config = sample_config();
    let blind_schedule = config.blind_schedule.clone();

    let mut participants = BTreeMap::new();
    participants.insert(
        "player-a".to_string(),
        sample_participant("player-a", "Alice", Some(0), "a"),
    );
    participants.insert(
        "player-b".to_string(),
        sample_participant("player-b", "Bob", Some(1), "b"),
    );

    let alice_cards = two_card_hand();
    let bob_cards = vec![
        Card {
            rank: Rank::Two,
            suit: Suit::Clubs,
        },
        Card {
            rank: Rank::Three,
            suit: Suit::Diamonds,
        },
    ];

    let mut hole_cards = BTreeMap::new();
    hole_cards.insert("player-a".to_string(), alice_cards.clone());
    hole_cards.insert("player-b".to_string(), bob_cards.clone());

    let mut participation = BTreeMap::new();
    participation.insert("player-a".to_string(), HandParticipationState::Active);
    participation.insert("player-b".to_string(), HandParticipationState::Active);

    let mut hand = minimal_hand(HandCyclePhase::AwaitingAction);
    hand.hole_cards_by_player_id = hole_cards;
    hand.participation_by_player_id = participation;

    let state = TournamentState {
        table_id: "snap-test".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::Running,
        config: config.clone(),
        blind_schedule,
        blind_level_index: 0,
        participants,
        seats: vec![
            occupied_seat(0, "player-a", "Alice"),
            occupied_seat(1, "player-b", "Bob"),
        ],
        current_hand: Some(hand),
        hand_results: vec![],
        placements: vec![],
    };

    let (snapshot, private_cards) = build_recipient_snapshot_state(&state, "player-a")
        .expect("snapshot should succeed for player-a");

    // Alice's private cards are returned separately
    assert_eq!(private_cards, alice_cards);

    // Public hole cards should be empty (not at Showdown)
    if let Some(hand_snap) = &snapshot.current_hand {
        assert!(
            hand_snap.public_hole_cards_by_player_id.is_empty(),
            "no public hole cards before showdown"
        );
    }
}

#[test]
fn build_recipient_snapshot_state_errors_for_unregistered_player() {
    let config = sample_config();
    let blind_schedule = config.blind_schedule.clone();

    let mut participants = BTreeMap::new();
    participants.insert(
        "player-a".to_string(),
        sample_participant("player-a", "Alice", Some(0), "a"),
    );

    let state = TournamentState {
        table_id: "snap-test-missing".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::Running,
        config,
        blind_schedule,
        blind_level_index: 0,
        participants,
        seats: vec![occupied_seat(0, "player-a", "Alice")],
        current_hand: None,
        hand_results: vec![],
        placements: vec![],
    };

    let result = build_recipient_snapshot_state(&state, "ghost");
    assert!(result.is_err());
    assert!(
        result.unwrap_err().to_string().contains("not registered"),
        "expected not-registered error"
    );
}

// ——— T6.4 — build_recipient_snapshot_state stale seat_index ————

#[test]
fn build_recipient_snapshot_state_normalizes_stale_seat_index_to_none() {
    let config = sample_config();
    let blind_schedule = config.blind_schedule.clone();

    // "real-player" is in seat 0; "ghost" also claims seat 0 (stale link)
    // After normalization, ghost.seat_index should become None
    let ghost = sample_participant("ghost", "Ghost", Some(0), "ghost");

    let mut participants = BTreeMap::new();
    participants.insert(
        "real-player".to_string(),
        sample_participant("real-player", "Real", Some(0), "real"),
    );
    // Ghost claims seat 0 but the seat is owned by real-player → stale
    participants.insert("ghost".to_string(), ghost);

    let state = TournamentState {
        table_id: "snap-stale".to_string(),
        session_epoch: 1,
        phase: TournamentPhase::WaitingForPlayers,
        config,
        blind_schedule,
        blind_level_index: 0,
        participants,
        seats: vec![occupied_seat(0, "real-player", "Real")],
        current_hand: None,
        hand_results: vec![],
        placements: vec![],
    };

    let (snapshot, _) =
        build_recipient_snapshot_state(&state, "real-player").expect("snapshot should succeed");

    let ghost_snap = snapshot
        .participants
        .get("ghost")
        .expect("ghost should be in snapshot");
    assert_eq!(
        ghost_snap.seat_index, None,
        "ghost stale seat_index should be normalized to None"
    );
}
