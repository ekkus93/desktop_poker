use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use base64::Engine as _;
use rand_core::{OsRng, RngCore};

use crate::{
    crypto::{self},
    domain, engine, interop, networking, protocol, storage, tournament,
};

use super::*;

pub(crate) fn client_snapshot_state_from_event(
    event: &protocol::SnapshotEvent,
) -> domain::SnapshotState {
    let state = &event.state;
    let occupied_seat_links = state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == domain::SeatOccupancyState::Occupied)
        .filter_map(|seat| {
            seat.participant_id
                .as_ref()
                .map(|participant_id| (seat.seat_index, participant_id.clone()))
        })
        .collect::<BTreeMap<_, _>>();
    let participants = state
        .participants
        .values()
        .map(|participant| {
            let normalized_seat_index = participant.seat_index.and_then(|seat_index| {
                (occupied_seat_links.get(&seat_index) == Some(&participant.player_id))
                    .then_some(seat_index)
            });
            (
                participant.player_id.clone(),
                domain::ParticipantRegistryEntry {
                    identity: domain::PlayerIdentity {
                        player_id: participant.player_id.clone(),
                        display_name: participant.display_name.clone(),
                        signing_public_key: format!("snapshot-sign-{}", participant.player_id),
                        encryption_public_key: format!("snapshot-enc-{}", participant.player_id),
                        signing_key_fingerprint: format!("snapshot-fp-{}", participant.player_id),
                    },
                    state: participant.participant_state,
                    connection_state: participant.connection_state,
                    seat_index: normalized_seat_index,
                    admitted_at_ms: state.session_epoch,
                    reconnect_token: None,
                    reconnect_expiry_ms: None,
                    is_host: participant.is_host,
                },
            )
        })
        .collect();
    let current_hand = state.current_hand.as_ref().map(|hand| {
        let mut hole_cards_by_player_id = hand.public_hole_cards_by_player_id.clone();
        if !event.private_hole_cards.is_empty() {
            hole_cards_by_player_id.insert(
                event.local_player_id.clone(),
                event.private_hole_cards.clone(),
            );
        }

        domain::HandState {
            hand_number: hand.hand_number,
            cycle_phase: hand.cycle_phase,
            street: hand.street,
            dealer_seat_index: hand.dealer_seat_index,
            small_blind_seat_index: hand.small_blind_seat_index,
            big_blind_seat_index: hand.big_blind_seat_index,
            board_cards: hand.board_cards.clone(),
            hole_cards_by_player_id,
            participation_by_player_id: hand.participation_by_player_id.clone(),
            betting_round: hand.betting_round.clone(),
            action_window: hand.action_window.clone(),
        }
    });

    domain::SnapshotState {
        state: domain::TournamentState {
            table_id: state.table_id.clone(),
            session_epoch: state.session_epoch,
            phase: state.phase,
            config: state.config.clone(),
            blind_schedule: state.blind_schedule.clone(),
            blind_level_index: state.blind_level_index,
            participants,
            seats: state.seats.clone(),
            current_hand,
            hand_results: state.hand_results.clone(),
            placements: state.placements.clone(),
        },
        local_player_id: event.local_player_id.clone(),
        reconnect_token: event.reconnect_token.clone(),
        host_signing_public_key: event.host_signing_public_key.clone(),
        host_encryption_public_key: event.host_encryption_public_key.clone(),
    }
}

pub(crate) fn build_host_tournament_config(
    request: &StartHostSessionRequest,
) -> Result<domain::TournamentConfig, String> {
    let blind_schedule = blind_schedule_for_preset(&request.blind_preset_id)?;

    Ok(domain::TournamentConfig {
        tournament_name: request.tournament_name.trim().to_string(),
        table_name: Some("Main Table".to_string()),
        max_players: request.max_players,
        starting_stack: request.starting_stack,
        turn_timer_seconds: request.turn_timer_seconds,
        blind_schedule,
    })
}

pub(crate) fn blind_schedule_for_preset(
    blind_preset_id: &str,
) -> Result<domain::BlindSchedule, String> {
    let duration_seconds = match blind_preset_id.trim() {
        "fast" => 180,
        "normal" => 300,
        "slow" => 480,
        "turbo" => 180,
        "standard" => 300,
        "deep-stack" => 480,
        other => {
            return Err(format!("unsupported blindPresetId: {other}"));
        }
    };

    Ok(domain::BlindSchedule {
        levels: [
            (10, 20),
            (15, 30),
            (25, 50),
            (50, 100),
            (75, 150),
            (100, 200),
            (150, 300),
            (200, 400),
            (300, 600),
            (400, 800),
            (600, 1200),
            (800, 1600),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (small_blind, big_blind))| domain::BlindLevel {
            level_index: (index + 1) as u8,
            label: format!("Level {}", index + 1),
            small_blind,
            big_blind,
            ante: 0,
            duration_seconds,
        })
        .collect(),
    })
}

pub(crate) fn issue_join_token() -> String {
    let mut bytes = [0_u8; 24];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn build_initial_host_state(
    config: &domain::TournamentConfig,
    display_name: &str,
    table_id: &str,
    session_epoch: u64,
    host_signing_keys: &Arc<crypto::SigningKeyMaterial>,
    host_encryption_keys: &Arc<Mutex<crypto::EncryptionKeyMaterial>>,
) -> Result<domain::TournamentState, String> {
    let host_encryption_public_key = host_encryption_keys
        .lock()
        .map_err(|_| "host encryption key lock poisoned".to_string())?
        .public_key_base64();

    let mut participants = BTreeMap::new();
    participants.insert(
        LOCAL_PLAYER_ID.to_string(),
        domain::ParticipantRegistryEntry {
            identity: domain::PlayerIdentity {
                player_id: LOCAL_PLAYER_ID.to_string(),
                display_name: display_name.to_string(),
                signing_public_key: host_signing_keys.public_key_base64(),
                encryption_public_key: host_encryption_public_key,
                signing_key_fingerprint: host_signing_keys.key_id(),
            },
            state: domain::ParticipantState::Seated,
            connection_state: domain::ConnectionState::Connected,
            seat_index: Some(0),
            admitted_at_ms: session_epoch,
            reconnect_token: None,
            reconnect_expiry_ms: None,
            is_host: true,
        },
    );

    let seats = (0..config.max_players)
        .map(|seat_index| {
            if seat_index == 0 {
                domain::SeatState {
                    seat_index,
                    occupancy: domain::SeatOccupancyState::Occupied,
                    tournament_state: domain::TournamentSeatState::Lobby,
                    participant_id: Some(LOCAL_PLAYER_ID.to_string()),
                    display_name: Some(display_name.to_string()),
                    chip_count: None,
                    is_ready: false,
                    marker: None,
                }
            } else {
                domain::SeatState {
                    seat_index,
                    occupancy: domain::SeatOccupancyState::Empty,
                    tournament_state: domain::TournamentSeatState::Open,
                    participant_id: None,
                    display_name: None,
                    chip_count: None,
                    is_ready: false,
                    marker: None,
                }
            }
        })
        .collect::<Vec<_>>();

    Ok(domain::TournamentState {
        table_id: table_id.to_string(),
        session_epoch,
        phase: domain::TournamentPhase::WaitingForPlayers,
        config: config.clone(),
        blind_schedule: config.blind_schedule.clone(),
        blind_level_index: 0,
        participants,
        seats,
        current_hand: None,
        hand_results: Vec::new(),
        placements: Vec::new(),
    })
}

pub(crate) fn format_connection_state_value(connection_state: domain::ConnectionState) -> String {
    match connection_state {
        domain::ConnectionState::Connected => "connected".to_string(),
        domain::ConnectionState::Disconnected => "disconnected".to_string(),
        domain::ConnectionState::Reconnecting => "reconnecting".to_string(),
    }
}

pub(crate) fn format_participant_state_value(
    participant_state: domain::ParticipantState,
) -> String {
    match participant_state {
        domain::ParticipantState::Admitted => "admitted".to_string(),
        domain::ParticipantState::Seated => "seated".to_string(),
        domain::ParticipantState::Active => "active".to_string(),
        domain::ParticipantState::Reconnecting => "reconnecting".to_string(),
        domain::ParticipantState::EliminatedObserver => "eliminatedObserver".to_string(),
        domain::ParticipantState::Removed => "removed".to_string(),
    }
}

pub(crate) fn format_tournament_phase_value(phase: domain::TournamentPhase) -> String {
    match phase {
        domain::TournamentPhase::WaitingForPlayers => "waitingForPlayers".to_string(),
        domain::TournamentPhase::ReadyCheck => "readyCheck".to_string(),
        domain::TournamentPhase::Running => "running".to_string(),
        domain::TournamentPhase::Complete => "complete".to_string(),
        domain::TournamentPhase::Cancelled => "cancelled".to_string(),
    }
}

pub(crate) fn active_seat_count_for_state(authoritative_state: &domain::TournamentState) -> u8 {
    authoritative_state
        .seats
        .iter()
        .filter(|seat| seat.occupancy == domain::SeatOccupancyState::Occupied)
        .count() as u8
}

pub(crate) fn build_session_participants(
    authoritative_state: &domain::TournamentState,
) -> Vec<HostSessionParticipantView> {
    authoritative_state
        .participants
        .values()
        .map(|participant| HostSessionParticipantView {
            player_id: participant.identity.player_id.clone(),
            display_name: participant.identity.display_name.clone(),
            seat_index: participant.seat_index,
            is_host: participant.is_host,
            is_ready: participant
                .seat_index
                .and_then(|seat_index| authoritative_state.seats.get(seat_index as usize))
                .map(|seat| seat.is_ready)
                .unwrap_or(false),
            connection_state: format_connection_state_value(participant.connection_state),
            participant_state: format_participant_state_value(participant.state),
        })
        .collect()
}

#[must_use]
pub fn screen_catalog(debug_tools_enabled: bool) -> Vec<ScreenDescriptor> {
    let mut screens = vec![
        ScreenDescriptor {
            id: "home",
            title: "Home",
            route: "/",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "host-tournament-setup",
            title: "Host Tournament Setup",
            route: "/host",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "join-tournament",
            title: "Join Tournament",
            route: "/join",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "tournament-lobby",
            title: "Tournament Lobby",
            route: "/lobby",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "main-table",
            title: "Main Table",
            route: "/table",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "hand-history",
            title: "Hand History",
            route: "/history",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "tournament-complete",
            title: "Tournament Complete",
            route: "/complete",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "rules-help",
            title: "Rules / Help",
            route: "/rules",
            surface: "screen",
        },
        ScreenDescriptor {
            id: "reconnect-errors",
            title: "Reconnect / Errors",
            route: "/errors",
            surface: "dialog",
        },
    ];

    if debug_tools_enabled {
        screens.push(ScreenDescriptor {
            id: "debug-tools",
            title: "Debug / Internal Tools",
            route: "/debug",
            surface: "panel",
        });
    }

    screens
}

pub(crate) fn backend_modules() -> Vec<ModuleDescriptor> {
    vec![
        app_state_descriptor(),
        domain::descriptor(),
        engine::descriptor(),
        tournament::descriptor(),
        protocol::descriptor(),
        networking::descriptor(),
        crypto::descriptor(),
        storage::descriptor(),
        interop::descriptor(),
    ]
}

pub(crate) fn app_state_descriptor() -> ModuleDescriptor {
    ModuleDescriptor {
        name: "app_state",
        responsibility:
            "Detects launch context, profile namespace, backend module map, screen catalog, and the desktop shell table runtime.",
    }
}
