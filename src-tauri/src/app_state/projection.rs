use crate::domain;

use super::*;

pub(crate) fn build_table_view_snapshot(
    state: &domain::TournamentState,
    local_player_id: &str,
    viewer_mode: TableViewerMode,
    include_action_tray: bool,
    event_feed: Vec<TableEventView>,
) -> Result<TableViewSnapshot, String> {
    let projection = domain::StateProjector::project(state).map_err(|error| error.to_string())?;
    let public_state = projection.public_state.clone();
    let private_projection = projection
        .private_states
        .get(local_player_id)
        .cloned()
        .ok_or_else(|| "local player projection missing".to_string())?;
    let current_hand = state.current_hand.as_ref();
    let action_window = current_hand.and_then(|hand| hand.action_window.clone());
    let action_owner_label = public_state
        .action_window_player_id
        .as_deref()
        .and_then(|player_id| display_name_for_state(state, player_id))
        .unwrap_or_else(|| "Waiting for settlement".to_string());
    let pot_total = current_hand
        .as_ref()
        .map(|hand| hand.betting_round.pot_size)
        .unwrap_or_default();

    Ok(TableViewSnapshot {
        viewer_mode,
        // Callers that wrap this function set session_connection to the
        // appropriate value for their transport (normal/reconnecting/terminated).
        session_connection: "normal".to_string(),
        tournament_name: public_state.tournament_name,
        table_name: public_state
            .table_name
            .unwrap_or_else(|| "Main Table".to_string()),
        table_id: public_state.table_id,
        phase_label: format_phase(public_state.phase),
        street_label: current_hand
            .map(|hand| format_street(hand.street))
            .unwrap_or_else(|| "Waiting".to_string()),
        blind_level_label: public_state
            .blind_level_label
            .unwrap_or_else(|| "Blind level pending".to_string()),
        current_hand_number: public_state.current_hand_number,
        board_cards: public_state.board_cards.iter().map(card_view).collect(),
        pot_total,
        action_owner_label: action_owner_label.clone(),
        elimination_summary: build_elimination_summary(state, public_state.phase),
        observer_banner: matches!(viewer_mode, TableViewerMode::Observer).then(|| {
            "Observer mode uses the public projector only: no private hole cards and no actions."
                .to_string()
        }),
        seats: build_table_seats_for_state(
            state,
            viewer_mode,
            local_player_id,
            &private_projection,
        )?,
        standings: build_table_standings_for_state(state, local_player_id),
        hand_history: build_table_history_for_state(state),
        event_feed,
        action_tray: if include_action_tray
            && matches!(viewer_mode, TableViewerMode::Local)
            && private_projection.can_act
        {
            action_window.map(|window| TableActionTrayView {
                owner_label: action_owner_label,
                check_or_call_label: if window.call_amount == 0 {
                    "Check".to_string()
                } else {
                    format!("Call {}", window.call_amount)
                },
                bet_or_raise_label: if window.call_amount == 0 {
                    "Bet / Raise".to_string()
                } else {
                    format!(
                        "Raise to {}+",
                        window.min_raise_to.unwrap_or(window.call_amount)
                    )
                },
                call_amount: window.call_amount,
                current_bet: current_hand
                    .as_ref()
                    .map(|hand| hand.betting_round.current_bet)
                    .unwrap_or_default(),
                pot_total,
                min_raise_to: window.min_raise_to,
                max_raise_to: window.max_raise_to,
                deadline_epoch_ms: window.deadline_epoch_ms,
                legal_actions: window
                    .legal_actions
                    .iter()
                    .map(|action| format_action(*action))
                    .collect(),
            })
        } else {
            None
        },
    })
}

pub(crate) fn build_table_seats_for_state(
    state: &domain::TournamentState,
    viewer_mode: TableViewerMode,
    local_player_id: &str,
    private_projection: &domain::PrivateState,
) -> Result<Vec<TableSeatView>, String> {
    let current_hand = state.current_hand.as_ref();
    let contributions = current_hand
        .as_ref()
        .map(|hand| hand.betting_round.contributions_by_player_id.clone())
        .unwrap_or_default();

    let mut seats = state
        .seats
        .iter()
        .map(|seat| {
            if seat.occupancy == domain::SeatOccupancyState::Empty {
                return Ok(TableSeatView {
                    seat_index: seat.seat_index + 1,
                    display_name: "Open seat".to_string(),
                    chip_count: None,
                    status_label: "Open".to_string(),
                    marker_label: None,
                    contribution: 0,
                    is_local: false,
                    is_acting: false,
                    is_observer: false,
                    is_eliminated: false,
                    is_compact: true,
                    cards_hidden: true,
                    hole_cards: Vec::new(),
                    detail_lines: vec!["Available for another player to join.".to_string()],
                });
            }

            let player_id = seat
                .participant_id
                .as_deref()
                .ok_or_else(|| "occupied seat missing participant".to_string())?;
            let display_name = seat
                .display_name
                .clone()
                .unwrap_or_else(|| player_id.to_string());
            let participation = current_hand
                .as_ref()
                .and_then(|hand| hand.participation_by_player_id.get(player_id).copied());
            let is_local = player_id == local_player_id;
            let revealed_public_cards = current_hand
                .as_ref()
                .and_then(|hand| {
                    hand.hole_cards_by_player_id
                        .get(player_id)
                        .cloned()
                        .filter(|_| {
                            !is_local
                                && matches!(
                                    viewer_mode,
                                    TableViewerMode::Local | TableViewerMode::Observer
                                )
                                && matches!(
                                    hand.cycle_phase,
                                    domain::HandCyclePhase::Showdown
                                        | domain::HandCyclePhase::Settlement
                                )
                                && hand.participation_by_player_id.get(player_id).is_some_and(
                                    |participation| {
                                        !matches!(
                                            participation,
                                            domain::HandParticipationState::Folded
                                                | domain::HandParticipationState::Out
                                                | domain::HandParticipationState::EliminatedObserver
                                        )
                                    },
                                )
                        })
                })
                .unwrap_or_default();
            let visible_cards = if is_local && matches!(viewer_mode, TableViewerMode::Local) {
                private_projection
                    .private_hole_cards
                    .iter()
                    .map(card_view)
                    .collect::<Vec<_>>()
            } else if !revealed_public_cards.is_empty() {
                revealed_public_cards
                    .iter()
                    .map(card_view)
                    .collect::<Vec<_>>()
            } else {
                Vec::new()
            };
            let cards_hidden = visible_cards.is_empty();
            let contribution = contributions.get(player_id).copied().unwrap_or_default();
            let participant = state
                .participants
                .get(player_id)
                .ok_or_else(|| "participant missing from registry".to_string())?;

            if player_id == RESERVED_PLAYER_ID {
                return Ok(TableSeatView {
                    seat_index: seat.seat_index + 1,
                    display_name: "Waiting for player".to_string(),
                    chip_count: None,
                    status_label: "Reserved".to_string(),
                    marker_label: None,
                    contribution: 0,
                    is_local: false,
                    is_acting: false,
                    is_observer: false,
                    is_eliminated: false,
                    is_compact: true,
                    cards_hidden: true,
                    hole_cards: Vec::new(),
                    detail_lines: vec![
                        "Placeholder seat until another real player joins.".to_string()
                    ],
                });
            }

            Ok(TableSeatView {
                seat_index: seat.seat_index + 1,
                display_name,
                chip_count: seat.chip_count,
                status_label: status_label_for_seat(seat, participation),
                marker_label: seat.marker.map(format_marker),
                contribution,
                is_local,
                is_acting: current_hand
                    .as_ref()
                    .and_then(|hand| hand.action_window.as_ref())
                    .is_some_and(|window| window.player_id == player_id),
                is_observer: participant.state == domain::ParticipantState::EliminatedObserver,
                is_eliminated: seat.tournament_state
                    == domain::TournamentSeatState::EliminatedObserver,
                is_compact: !is_local,
                cards_hidden,
                hole_cards: visible_cards,
                detail_lines: vec![
                    format!(
                        "Connection: {}",
                        format_connection_state(participant.connection_state)
                    ),
                    format!(
                        "Seat state: {}",
                        format_tournament_seat_state(seat.tournament_state)
                    ),
                    format!("Contribution: {contribution}"),
                ],
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    seats.sort_by_key(|seat| seat.seat_index);
    Ok(seats)
}

pub(crate) fn build_table_standings_for_state(
    state: &domain::TournamentState,
    local_player_id: &str,
) -> Vec<TableStandingView> {
    let mut standings = state
        .seats
        .iter()
        .filter(|seat| {
            seat.occupancy == domain::SeatOccupancyState::Occupied
                && seat.participant_id.as_deref() != Some(RESERVED_PLAYER_ID)
        })
        .map(|seat| TableStandingView {
            rank: 0,
            display_name: seat
                .display_name
                .clone()
                .unwrap_or_else(|| "Player".to_string()),
            chip_count: seat.chip_count,
            status_label: format_tournament_seat_state(seat.tournament_state),
            note: seat.marker.map(format_marker),
            is_local: seat.participant_id.as_deref() == Some(local_player_id),
            is_observer: seat.tournament_state == domain::TournamentSeatState::EliminatedObserver,
        })
        .collect::<Vec<_>>();
    standings.sort_by(|left, right| {
        right
            .chip_count
            .unwrap_or_default()
            .cmp(&left.chip_count.unwrap_or_default())
            .then_with(|| left.display_name.cmp(&right.display_name))
    });
    for (index, entry) in standings.iter_mut().enumerate() {
        entry.rank = (index + 1) as u8;
    }
    standings
}

pub(crate) fn build_table_history_for_state(
    state: &domain::TournamentState,
) -> Vec<TableHistoryEntryView> {
    state
        .hand_results
        .iter()
        .rev()
        .map(|result| TableHistoryEntryView {
            hand_number: result.hand_number,
            summary: format!(
                "{} won {} chip(s).",
                display_names_for_state(state, &result.winning_player_ids).join(", "),
                result
                    .pot_summaries
                    .iter()
                    .map(|summary| summary.amount)
                    .sum::<u32>()
            ),
            pot_total: result
                .pot_summaries
                .iter()
                .map(|summary| summary.amount)
                .sum(),
            winning_players: display_names_for_state(state, &result.winning_player_ids),
            eliminated_players: display_names_for_state(state, &result.eliminated_player_ids),
            board_cards: result.board_cards.iter().map(card_view).collect(),
        })
        .collect()
}

pub(crate) fn build_elimination_summary(
    state: &domain::TournamentState,
    phase: domain::TournamentPhase,
) -> String {
    if phase == domain::TournamentPhase::WaitingForPlayers {
        return "Waiting for the first real hand to start.".to_string();
    }
    if phase == domain::TournamentPhase::ReadyCheck {
        return "Waiting for every seated player to be ready.".to_string();
    }

    state
        .hand_results
        .last()
        .map(|result| {
            format!(
                "{} won {} chip(s).",
                display_names_for_state(state, &result.winning_player_ids).join(", "),
                result
                    .pot_summaries
                    .iter()
                    .map(|summary| summary.amount)
                    .sum::<u32>()
            )
        })
        .unwrap_or_else(|| "Table state is live.".to_string())
}

pub(crate) fn card_view(card: &domain::Card) -> TableCardView {
    let (rank_label, compact_label) = match card.rank {
        domain::Rank::Two => ("Two", "2"),
        domain::Rank::Three => ("Three", "3"),
        domain::Rank::Four => ("Four", "4"),
        domain::Rank::Five => ("Five", "5"),
        domain::Rank::Six => ("Six", "6"),
        domain::Rank::Seven => ("Seven", "7"),
        domain::Rank::Eight => ("Eight", "8"),
        domain::Rank::Nine => ("Nine", "9"),
        domain::Rank::Ten => ("Ten", "10"),
        domain::Rank::Jack => ("Jack", "J"),
        domain::Rank::Queen => ("Queen", "Q"),
        domain::Rank::King => ("King", "K"),
        domain::Rank::Ace => ("Ace", "A"),
    };
    let (suit_label, suit_symbol, tone) = match card.suit {
        domain::Suit::Clubs => ("Clubs", "♣", "dark"),
        domain::Suit::Diamonds => ("Diamonds", "♦", "red"),
        domain::Suit::Hearts => ("Hearts", "♥", "red"),
        domain::Suit::Spades => ("Spades", "♠", "dark"),
    };
    TableCardView {
        label: format!("{rank_label} of {suit_label}"),
        compact_label: format!("{compact_label}{suit_symbol}"),
        suit_symbol,
        tone,
    }
}

pub(crate) fn display_name_for_state(
    state: &domain::TournamentState,
    player_id: &str,
) -> Option<String> {
    state
        .participants
        .get(player_id)
        .map(|participant| participant.identity.display_name.clone())
}

pub(crate) fn display_names_for_state(
    state: &domain::TournamentState,
    player_ids: &[String],
) -> Vec<String> {
    player_ids
        .iter()
        .map(|player_id| {
            display_name_for_state(state, player_id).unwrap_or_else(|| player_id.clone())
        })
        .collect()
}

pub(crate) fn status_label_for_seat(
    seat: &domain::SeatState,
    participation: Option<domain::HandParticipationState>,
) -> String {
    match participation {
        Some(domain::HandParticipationState::Folded) => "Folded this hand".to_string(),
        Some(domain::HandParticipationState::AllIn) => "All-in".to_string(),
        Some(domain::HandParticipationState::Active) => {
            format_tournament_seat_state(seat.tournament_state)
        }
        Some(domain::HandParticipationState::EliminatedObserver) => {
            "Eliminated observer".to_string()
        }
        Some(domain::HandParticipationState::Out) => "Out of this hand".to_string(),
        Some(domain::HandParticipationState::Waiting) | None => {
            format_tournament_seat_state(seat.tournament_state)
        }
    }
}

pub(crate) fn format_phase(phase: domain::TournamentPhase) -> String {
    match phase {
        domain::TournamentPhase::WaitingForPlayers => "Waiting for players".to_string(),
        domain::TournamentPhase::ReadyCheck => "Ready check".to_string(),
        domain::TournamentPhase::Running => "Running".to_string(),
        domain::TournamentPhase::Complete => "Complete".to_string(),
        domain::TournamentPhase::Cancelled => "Cancelled".to_string(),
    }
}

pub(crate) fn format_street(street: domain::StreetPhase) -> String {
    match street {
        domain::StreetPhase::Preflop => "Preflop".to_string(),
        domain::StreetPhase::Flop => "Flop".to_string(),
        domain::StreetPhase::Turn => "Turn".to_string(),
        domain::StreetPhase::River => "River".to_string(),
        domain::StreetPhase::Showdown => "Showdown".to_string(),
    }
}

pub(crate) fn format_marker(marker: domain::SeatMarker) -> String {
    match marker {
        domain::SeatMarker::Dealer => "Dealer".to_string(),
        domain::SeatMarker::SmallBlind => "Small blind".to_string(),
        domain::SeatMarker::BigBlind => "Big blind".to_string(),
    }
}

pub(crate) fn format_connection_state(state: domain::ConnectionState) -> &'static str {
    match state {
        domain::ConnectionState::Connected => "Connected",
        domain::ConnectionState::Disconnected => "Disconnected",
        domain::ConnectionState::Reconnecting => "Reconnecting",
    }
}

pub(crate) fn format_tournament_seat_state(state: domain::TournamentSeatState) -> String {
    match state {
        domain::TournamentSeatState::Open => "Open".to_string(),
        domain::TournamentSeatState::Lobby => "Lobby".to_string(),
        domain::TournamentSeatState::Ready => "Ready".to_string(),
        domain::TournamentSeatState::Active => "Active".to_string(),
        domain::TournamentSeatState::EliminatedObserver => "Eliminated observer".to_string(),
        domain::TournamentSeatState::Closed => "Closed".to_string(),
    }
}

pub(crate) fn format_action(action: domain::ActionType) -> String {
    match action {
        domain::ActionType::Fold => "Fold".to_string(),
        domain::ActionType::Check => "Check".to_string(),
        domain::ActionType::Call => "Call".to_string(),
        domain::ActionType::Bet => "Bet".to_string(),
        domain::ActionType::Raise => "Raise".to_string(),
        domain::ActionType::AllIn => "All-in".to_string(),
    }
}
