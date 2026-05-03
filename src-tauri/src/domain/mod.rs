mod models;
mod projector;

use std::collections::{BTreeMap, BTreeSet};

use crate::app_state::ModuleDescriptor;
pub use models::*;
pub use projector::{ProjectionBundle, StateProjector};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DomainError {
    message: String,
}

impl DomainError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for DomainError {}

#[must_use]
pub fn descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "domain",
        responsibility:
            "Defines poker, tournament, seat, participant, and projection value types shared across the runtime.",
    }
}

pub fn validate_tournament_config(config: &TournamentConfig) -> Result<(), DomainError> {
    if !(2..=10).contains(&config.max_players) {
        return Err(DomainError::new("max_players must be between 2 and 10"));
    }

    if config.starting_stack == 0 {
        return Err(DomainError::new("starting_stack must be greater than zero"));
    }

    if config.turn_timer_seconds == 0 {
        return Err(DomainError::new(
            "turn_timer_seconds must be greater than zero",
        ));
    }

    validate_blind_schedule(&config.blind_schedule)
}

pub fn validate_blind_schedule(schedule: &BlindSchedule) -> Result<(), DomainError> {
    if schedule.levels.is_empty() {
        return Err(DomainError::new(
            "blind schedule must contain at least one level",
        ));
    }

    let mut previous_small_blind = 0_u32;
    let mut previous_big_blind = 0_u32;

    for level in &schedule.levels {
        if level.small_blind == 0 || level.big_blind == 0 {
            return Err(DomainError::new("blind levels must be greater than zero"));
        }

        if level.small_blind >= level.big_blind {
            return Err(DomainError::new(
                "small blind must be less than the big blind",
            ));
        }

        if level.duration_seconds == 0 {
            return Err(DomainError::new(
                "blind level duration must be greater than zero",
            ));
        }

        if level.small_blind < previous_small_blind || level.big_blind < previous_big_blind {
            return Err(DomainError::new(
                "blind schedule must be ordered monotonically",
            ));
        }

        previous_small_blind = level.small_blind;
        previous_big_blind = level.big_blind;
    }

    Ok(())
}

pub fn validate_tournament_state(state: &TournamentState) -> Result<(), DomainError> {
    validate_tournament_config(&state.config)?;

    let mut occupied_seat_indexes = BTreeSet::new();
    let mut occupied_player_ids = BTreeSet::new();

    for seat in &state.seats {
        if seat.occupancy == SeatOccupancyState::Occupied {
            if !occupied_seat_indexes.insert(seat.seat_index) {
                return Err(DomainError::new("duplicate occupied seat index"));
            }

            let participant_id = seat.participant_id.as_ref().ok_or_else(|| {
                DomainError::new("occupied seats must reference a participant registry entry")
            })?;

            if !occupied_player_ids.insert(participant_id.clone()) {
                return Err(DomainError::new(
                    "participant cannot occupy multiple seats at once",
                ));
            }
        }
    }

    let mut signing_keys = BTreeSet::new();

    for (player_id, participant) in &state.participants {
        if player_id != &participant.identity.player_id {
            return Err(DomainError::new(
                "participant registry key must match identity.player_id",
            ));
        }

        if !signing_keys.insert(participant.identity.signing_public_key.clone()) {
            return Err(DomainError::new("duplicate signing key binding detected"));
        }

        if let Some(seat_index) = participant.seat_index {
            let linked_seat = state
                .seats
                .iter()
                .find(|seat| {
                    seat.seat_index == seat_index && seat.occupancy == SeatOccupancyState::Occupied
                })
                .ok_or_else(|| {
                    DomainError::new(
                        "participant seat_index must refer to an occupied seat in the seat map",
                    )
                })?;

            if linked_seat.participant_id.as_deref() != Some(player_id.as_str()) {
                return Err(DomainError::new(
                    "participant seat link and seat participant_id must agree",
                ));
            }
        }
    }

    if let Some(hand) = &state.current_hand {
        for player_id in hand.hole_cards_by_player_id.keys() {
            if !state.participants.contains_key(player_id) {
                return Err(DomainError::new(
                    "hand hole cards cannot reference an unknown participant",
                ));
            }
        }

        if let Some(action_window) = &hand.action_window {
            let participant = state
                .participants
                .get(&action_window.player_id)
                .ok_or_else(|| DomainError::new("action window references unknown participant"))?;

            if participant.seat_index != Some(action_window.seat_index) {
                return Err(DomainError::new(
                    "action window seat_index must match the participant seat",
                ));
            }
        }
    }

    Ok(())
}

#[must_use]
pub fn counted_capacity(participants: &BTreeMap<String, ParticipantRegistryEntry>) -> usize {
    participants
        .values()
        .filter(|participant| {
            matches!(
                participant.state,
                ParticipantState::Admitted
                    | ParticipantState::Seated
                    | ParticipantState::Active
                    | ParticipantState::Reconnecting
            )
        })
        .count()
}

pub fn validate_projection_bundle(
    authoritative_state: &TournamentState,
    projection: &ProjectionBundle,
) -> Result<(), DomainError> {
    let public_player_ids: BTreeSet<String> = projection
        .public_state
        .seats
        .iter()
        .filter_map(|seat| seat.participant_id.clone())
        .collect();

    let authoritative_player_ids: BTreeSet<String> =
        authoritative_state.participants.keys().cloned().collect();

    if !public_player_ids.is_subset(&authoritative_player_ids) {
        return Err(DomainError::new(
            "public projection references participants outside the authoritative registry",
        ));
    }

    for (player_id, private_state) in &projection.private_states {
        let participant = authoritative_state
            .participants
            .get(player_id)
            .ok_or_else(|| DomainError::new("private projection references unknown participant"))?;

        let expected_cards = authoritative_state
            .current_hand
            .as_ref()
            .and_then(|hand| hand.hole_cards_by_player_id.get(player_id))
            .cloned()
            .unwrap_or_default();

        let is_observer = participant.state == ParticipantState::EliminatedObserver;

        if is_observer {
            if !private_state.private_hole_cards.is_empty() {
                return Err(DomainError::new(
                    "eliminated observers must not receive private hole cards",
                ));
            }

            if private_state.can_act || private_state.action_window_player_id.is_some() {
                return Err(DomainError::new(
                    "eliminated observers must not receive an action window",
                ));
            }
        } else if private_state.private_hole_cards != expected_cards {
            return Err(DomainError::new(
                "private projection must contain only the recipient hole cards",
            ));
        }
    }

    if !projection.observer_projection.private_hole_cards.is_empty() {
        return Err(DomainError::new(
            "observer projection must never include private hole cards",
        ));
    }

    if projection.observer_projection.can_act
        || projection
            .observer_projection
            .action_window_player_id
            .is_some()
    {
        return Err(DomainError::new(
            "observer projection must never include action authority",
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_blind_schedule() -> BlindSchedule {
        BlindSchedule {
            levels: vec![
                BlindLevel {
                    level_index: 1,
                    label: "Level 1".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 180,
                },
                BlindLevel {
                    level_index: 2,
                    label: "Level 2".to_string(),
                    small_blind: 15,
                    big_blind: 30,
                    ante: 0,
                    duration_seconds: 180,
                },
            ],
        }
    }

    fn sample_config() -> TournamentConfig {
        TournamentConfig {
            tournament_name: "Local test".to_string(),
            table_name: Some("Table One".to_string()),
            max_players: 6,
            starting_stack: 1500,
            turn_timer_seconds: 20,
            blind_schedule: sample_blind_schedule(),
        }
    }

    fn participant(
        player_id: &str,
        signing_key: &str,
        state: ParticipantState,
        seat_index: Option<u8>,
    ) -> ParticipantRegistryEntry {
        ParticipantRegistryEntry {
            identity: PlayerIdentity {
                player_id: player_id.to_string(),
                display_name: format!("Player {player_id}"),
                signing_public_key: signing_key.to_string(),
                encryption_public_key: format!("enc-{player_id}"),
                signing_key_fingerprint: format!("fp-{player_id}"),
            },
            state,
            connection_state: ConnectionState::Connected,
            seat_index,
            admitted_at_ms: 1,
            reconnect_token: Some(format!("token-{player_id}")),
            reconnect_expiry_ms: Some(99_999),
            is_host: player_id == "host",
        }
    }

    fn seat(seat_index: u8, participant_id: &str, seat_state: TournamentSeatState) -> SeatState {
        SeatState {
            seat_index,
            occupancy: SeatOccupancyState::Occupied,
            tournament_state: seat_state,
            participant_id: Some(participant_id.to_string()),
            display_name: Some(format!("Player {participant_id}")),
            chip_count: Some(1500),
            is_ready: true,
            marker: None,
        }
    }

    fn sample_state() -> TournamentState {
        let mut participants = BTreeMap::new();
        participants.insert(
            "host".to_string(),
            participant("host", "sign-host", ParticipantState::Active, Some(0)),
        );
        participants.insert(
            "guest".to_string(),
            participant("guest", "sign-guest", ParticipantState::Active, Some(1)),
        );
        participants.insert(
            "observer".to_string(),
            participant(
                "observer",
                "sign-observer",
                ParticipantState::EliminatedObserver,
                Some(2),
            ),
        );
        participants.insert(
            "waiting".to_string(),
            participant("waiting", "sign-waiting", ParticipantState::Admitted, None),
        );

        let seats = vec![
            seat(0, "host", TournamentSeatState::Active),
            seat(1, "guest", TournamentSeatState::Active),
            seat(2, "observer", TournamentSeatState::EliminatedObserver),
        ];

        let mut hole_cards_by_player_id = BTreeMap::new();
        hole_cards_by_player_id.insert(
            "host".to_string(),
            vec![
                Card {
                    rank: Rank::Ace,
                    suit: Suit::Spades,
                },
                Card {
                    rank: Rank::King,
                    suit: Suit::Spades,
                },
            ],
        );
        hole_cards_by_player_id.insert(
            "guest".to_string(),
            vec![
                Card {
                    rank: Rank::Queen,
                    suit: Suit::Hearts,
                },
                Card {
                    rank: Rank::Jack,
                    suit: Suit::Hearts,
                },
            ],
        );
        hole_cards_by_player_id.insert(
            "observer".to_string(),
            vec![
                Card {
                    rank: Rank::Ten,
                    suit: Suit::Clubs,
                },
                Card {
                    rank: Rank::Nine,
                    suit: Suit::Clubs,
                },
            ],
        );

        TournamentState {
            table_id: "table-1".to_string(),
            session_epoch: 1,
            phase: TournamentPhase::Running,
            config: sample_config(),
            blind_schedule: sample_blind_schedule(),
            blind_level_index: 0,
            participants,
            seats,
            current_hand: Some(HandState {
                hand_number: 1,
                cycle_phase: HandCyclePhase::AwaitingAction,
                street: StreetPhase::Preflop,
                dealer_seat_index: 0,
                small_blind_seat_index: 1,
                big_blind_seat_index: 2,
                board_cards: Vec::new(),
                hole_cards_by_player_id,
                participation_by_player_id: BTreeMap::from([
                    ("host".to_string(), HandParticipationState::Active),
                    ("guest".to_string(), HandParticipationState::Active),
                    (
                        "observer".to_string(),
                        HandParticipationState::EliminatedObserver,
                    ),
                ]),
                betting_round: BettingRoundState {
                    street: StreetPhase::Preflop,
                    current_bet: 20,
                    min_raise_to: Some(40),
                    max_raise_to: Some(1500),
                    pot_size: 30,
                    contributions_by_player_id: BTreeMap::from([
                        ("host".to_string(), 10),
                        ("guest".to_string(), 20),
                    ]),
                },
                action_window: Some(ActionWindow {
                    player_id: "host".to_string(),
                    seat_index: 0,
                    legal_actions: vec![
                        ActionType::Fold,
                        ActionType::Call,
                        ActionType::Raise,
                        ActionType::AllIn,
                    ],
                    call_amount: 10,
                    min_raise_to: Some(40),
                    max_raise_to: Some(1500),
                    deadline_epoch_ms: 99_999,
                }),
            }),
            hand_results: Vec::new(),
            placements: Vec::new(),
        }
    }

    #[test]
    fn validates_tournament_config_constraints() {
        let mut config = sample_config();
        config.max_players = 1;
        assert!(validate_tournament_config(&config).is_err());

        config.max_players = 6;
        config.starting_stack = 0;
        assert!(validate_tournament_config(&config).is_err());
    }

    #[test]
    fn rejects_out_of_order_blind_schedules() {
        let schedule = BlindSchedule {
            levels: vec![
                BlindLevel {
                    level_index: 1,
                    label: "Level 1".to_string(),
                    small_blind: 25,
                    big_blind: 50,
                    ante: 0,
                    duration_seconds: 180,
                },
                BlindLevel {
                    level_index: 2,
                    label: "Level 2".to_string(),
                    small_blind: 10,
                    big_blind: 20,
                    ante: 0,
                    duration_seconds: 180,
                },
            ],
        };

        assert!(validate_blind_schedule(&schedule).is_err());
    }

    #[test]
    fn rejects_duplicate_signing_key_bindings() {
        let mut state = sample_state();
        state.participants.insert(
            "dupe".to_string(),
            participant("dupe", "sign-guest", ParticipantState::Admitted, None),
        );

        assert!(validate_tournament_state(&state).is_err());
    }

    #[test]
    fn counts_capacity_with_admitted_players_but_not_eliminated_observers() {
        let state = sample_state();

        assert_eq!(counted_capacity(&state.participants), 3);
    }

    #[test]
    fn projector_preserves_public_state_and_strips_hidden_cards() {
        let state = sample_state();

        let bundle = StateProjector::project(&state).expect("projection should succeed");
        validate_projection_bundle(&state, &bundle).expect("projection should validate");

        let host_private = bundle
            .private_states
            .get("host")
            .expect("host private state should exist");
        let guest_private = bundle
            .private_states
            .get("guest")
            .expect("guest private state should exist");
        let observer_private = bundle
            .private_states
            .get("observer")
            .expect("observer private state should exist");

        assert_eq!(bundle.public_state.current_hand_number, Some(1));
        assert_eq!(host_private.private_hole_cards.len(), 2);
        assert_eq!(guest_private.private_hole_cards.len(), 2);
        assert!(observer_private.private_hole_cards.is_empty());
        assert!(!observer_private.can_act);
        assert!(bundle.observer_projection.private_hole_cards.is_empty());
        assert!(!bundle.observer_projection.can_act);
    }
}
