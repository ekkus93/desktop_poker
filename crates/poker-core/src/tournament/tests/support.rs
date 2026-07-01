use super::super::*;
use crate::domain::*;
use crate::engine::Deck;

pub(super) fn sample_config(starting_stack: u32) -> TournamentConfig {
    TournamentConfig {
        tournament_name: "M4 Test".to_string(),
        table_name: Some("Table One".to_string()),
        max_players: 3,
        starting_stack,
        turn_timer_seconds: 10,
        blind_schedule: BlindSchedule {
            levels: vec![
                BlindLevel {
                    level_index: 1,
                    label: "Level 1".to_string(),
                    small_blind: 50,
                    big_blind: 100,
                    ante: 0,
                    duration_seconds: 5,
                },
                BlindLevel {
                    level_index: 2,
                    label: "Level 2".to_string(),
                    small_blind: 100,
                    big_blind: 200,
                    ante: 0,
                    duration_seconds: 5,
                },
            ],
        },
    }
}

pub(super) fn player(player_id: &str, seat_index: u8) -> RegisteredPlayer {
    RegisteredPlayer {
        identity: PlayerIdentity {
            player_id: player_id.to_string(),
            display_name: format!("Player {player_id}"),
            signing_public_key: format!("sign-{player_id}"),
            encryption_public_key: format!("enc-{player_id}"),
            signing_key_fingerprint: format!("fp-{player_id}"),
        },
        seat_index,
        is_host: player_id == "p1",
        is_ready: true,
    }
}

pub(super) fn card(rank: Rank, suit: Suit) -> Card {
    Card { rank, suit }
}

pub(super) fn stacked_deck(prefix: Vec<Card>) -> Deck {
    let mut cards = prefix.clone();
    cards.extend(
        Deck::standard_52()
            .cards()
            .iter()
            .copied()
            .filter(|card| !prefix.contains(card)),
    );
    Deck::from_cards(cards).expect("stacked deck should be unique")
}

pub(super) fn action_window(controller: &TournamentController) -> ActionWindow {
    controller
        .state()
        .current_hand
        .as_ref()
        .and_then(|hand| hand.action_window.clone())
        .expect("expected open action window")
}

pub(super) fn started_two_player_controller() -> TournamentController {
    let mut controller = TournamentController::new(
        "test-table",
        1,
        sample_config(1_000),
        vec![player("p1", 0), player("p2", 1)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");
    controller
}

pub(super) fn started_three_player_controller() -> TournamentController {
    let mut controller = TournamentController::new(
        "test-table-3",
        1,
        sample_config(1_500),
        vec![player("p1", 0), player("p2", 1), player("p3", 2)],
    )
    .expect("controller should build");
    controller
        .start_tournament(0)
        .expect("tournament should start");
    controller
}
