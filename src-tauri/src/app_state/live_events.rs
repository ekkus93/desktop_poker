use crate::{domain, networking, protocol};

use super::*;

pub(crate) fn build_live_event_feed(
    events: &[networking::PublicEventLogEntry],
    state: &domain::TournamentState,
) -> Vec<TableEventView> {
    let mut feed = events
        .iter()
        .rev()
        .take(20)
        .map(|event| {
            let mut entry = TableEventView {
                sequence: event.sequence,
                kind: format!("{:?}", event.message_type),
                message: format!("{:?}", event.message_type),
            };
            update_live_event_entry(&mut entry, event.message_type, &event.payload, state);
            entry
        })
        .collect::<Vec<_>>();
    feed.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
    feed
}

pub(crate) fn push_live_event(
    event_feed: &mut Vec<TableEventView>,
    server_sequence: u64,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
    state: &domain::TournamentState,
) {
    let mut entry = TableEventView {
        sequence: server_sequence,
        kind: format!("{:?}", message_type),
        message: format!("{:?}", message_type),
    };
    update_live_event_entry(&mut entry, message_type, payload, state);
    event_feed.push(entry);
    event_feed.sort_by_key(|entry| std::cmp::Reverse(entry.sequence));
    event_feed.truncate(20);
}

pub(crate) fn update_live_event_entry(
    entry: &mut TableEventView,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
    state: &domain::TournamentState,
) {
    match message_type {
        protocol::ProtocolMessageType::TournamentStartedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::TournamentStartedEvent>(payload.clone())
            {
                entry.kind = "Tournament start".to_string();
                entry.message = format!(
                    "{} is live with {} seated players.",
                    event.tournament_name,
                    event.frozen_player_ids.len()
                );
            }
        }
        protocol::ProtocolMessageType::HandStartingEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::HandStartingEvent>(payload.clone())
            {
                entry.kind = "Hand start".to_string();
                entry.message = format!("Hand {} started.", event.hand_number);
            }
        }
        protocol::ProtocolMessageType::ActionWindowOpenedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::ActionWindowOpened>(payload.clone())
            {
                entry.kind = "Action window".to_string();
                entry.message = format!(
                    "{} to act.",
                    display_name_for_state(state, &event.player_id)
                        .unwrap_or_else(|| event.player_id.clone())
                );
            }
        }
        protocol::ProtocolMessageType::PlayerActionCommittedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::PlayerActionCommitted>(payload.clone())
            {
                entry.kind = "Player action".to_string();
                let actor = display_name_for_state(state, &event.player_id)
                    .unwrap_or_else(|| event.player_id.clone());
                entry.message = match event.raise_to_amount {
                    Some(amount) => format!(
                        "{actor} {} to {}.",
                        format_action(event.action_type),
                        amount
                    ),
                    None => format!("{actor} {}.", format_action(event.action_type)),
                };
            }
        }
        protocol::ProtocolMessageType::StreetRevealedEvent => {
            if let Ok(event) = serde_json::from_value::<protocol::StreetRevealed>(payload.clone()) {
                entry.kind = "Street reveal".to_string();
                entry.message = format!("{} revealed.", event.street);
            }
        }
        protocol::ProtocolMessageType::HandResultCommittedEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::HandResultCommitted>(payload.clone())
            {
                entry.kind = "Hand result".to_string();
                entry.message = format!(
                    "Hand {} settled: {} collected the pot.",
                    event.hand_number,
                    display_names_for_state(state, &event.result.winning_player_ids).join(", ")
                );
            }
        }
        protocol::ProtocolMessageType::EliminationEvent => {
            if let Ok(event) = serde_json::from_value::<protocol::EliminationEvent>(payload.clone())
            {
                entry.kind = "Elimination".to_string();
                entry.message = format!(
                    "{} finished in place {}.",
                    display_name_for_state(state, &event.player_id)
                        .unwrap_or_else(|| event.player_id.clone()),
                    event.place
                );
            }
        }
        protocol::ProtocolMessageType::TournamentCompleteEvent => {
            if let Ok(event) =
                serde_json::from_value::<protocol::TournamentCompleteEvent>(payload.clone())
            {
                entry.kind = "Tournament complete".to_string();
                entry.message = format!(
                    "{} won the tournament.",
                    display_name_for_state(state, &event.winner_player_id)
                        .unwrap_or_else(|| event.winner_player_id.clone())
                );
            }
        }
        _ => {}
    }
}

pub(crate) fn parse_protocol_street(value: &str) -> Option<domain::StreetPhase> {
    match value.trim().to_ascii_uppercase().as_str() {
        "PREFLOP" => Some(domain::StreetPhase::Preflop),
        "FLOP" => Some(domain::StreetPhase::Flop),
        "TURN" => Some(domain::StreetPhase::Turn),
        "RIVER" => Some(domain::StreetPhase::River),
        "SHOWDOWN" => Some(domain::StreetPhase::Showdown),
        _ => None,
    }
}

pub(crate) fn apply_public_event_to_snapshot(
    state: &mut domain::TournamentState,
    local_player_id: &str,
    message_type: protocol::ProtocolMessageType,
    payload: &serde_json::Value,
) {
    match message_type {
        protocol::ProtocolMessageType::TournamentStartedEvent => {
            state.phase = domain::TournamentPhase::Running;
            for seat in &mut state.seats {
                if seat.occupancy == domain::SeatOccupancyState::Occupied {
                    seat.tournament_state = domain::TournamentSeatState::Active;
                    seat.is_ready = false;
                    seat.chip_count.get_or_insert(state.config.starting_stack);
                }
            }
            for participant in state.participants.values_mut() {
                if participant.state == domain::ParticipantState::Seated {
                    participant.state = domain::ParticipantState::Active;
                }
            }
        }
        protocol::ProtocolMessageType::HandStartingEvent => {
            let Ok(event) = serde_json::from_value::<protocol::HandStartingEvent>(payload.clone())
            else {
                return;
            };
            let blind_level = state
                .blind_schedule
                .levels
                .get(state.blind_level_index)
                .cloned();
            let small_blind = blind_level
                .as_ref()
                .map(|level| level.small_blind)
                .unwrap_or(0);
            let big_blind = blind_level
                .as_ref()
                .map(|level| level.big_blind)
                .unwrap_or(0);
            let mut contributions = std::collections::BTreeMap::new();
            let mut participation = std::collections::BTreeMap::new();
            for seat in &mut state.seats {
                seat.marker = None;
                if seat.occupancy != domain::SeatOccupancyState::Occupied {
                    continue;
                }
                seat.tournament_state =
                    if seat.tournament_state == domain::TournamentSeatState::EliminatedObserver {
                        domain::TournamentSeatState::EliminatedObserver
                    } else {
                        domain::TournamentSeatState::Active
                    };
                let Some(player_id) = seat.participant_id.clone() else {
                    continue;
                };
                if seat.tournament_state != domain::TournamentSeatState::EliminatedObserver {
                    participation.insert(player_id.clone(), domain::HandParticipationState::Active);
                } else {
                    participation.insert(
                        player_id.clone(),
                        domain::HandParticipationState::EliminatedObserver,
                    );
                }
                let contribution = if seat.seat_index == event.small_blind_seat_index {
                    seat.marker = Some(domain::SeatMarker::SmallBlind);
                    small_blind
                } else if seat.seat_index == event.big_blind_seat_index {
                    seat.marker = Some(domain::SeatMarker::BigBlind);
                    big_blind
                } else {
                    0
                };
                if seat.seat_index == event.dealer_seat_index {
                    seat.marker = Some(domain::SeatMarker::Dealer);
                }
                if contribution > 0 {
                    let stack = seat.chip_count.get_or_insert(state.config.starting_stack);
                    *stack = stack.saturating_sub(contribution);
                } else {
                    seat.chip_count.get_or_insert(state.config.starting_stack);
                }
                contributions.insert(player_id, contribution);
            }
            state.current_hand = Some(domain::HandState {
                hand_number: event.hand_number,
                cycle_phase: domain::HandCyclePhase::AwaitingAction,
                street: domain::StreetPhase::Preflop,
                dealer_seat_index: event.dealer_seat_index,
                small_blind_seat_index: event.small_blind_seat_index,
                big_blind_seat_index: event.big_blind_seat_index,
                board_cards: event.board_cards,
                hole_cards_by_player_id: std::collections::BTreeMap::new(),
                participation_by_player_id: participation,
                betting_round: domain::BettingRoundState {
                    street: domain::StreetPhase::Preflop,
                    current_bet: big_blind,
                    min_raise_to: Some(big_blind.saturating_mul(2)),
                    max_raise_to: None,
                    pot_size: small_blind.saturating_add(big_blind),
                    contributions_by_player_id: contributions,
                },
                action_window: None,
            });
        }
        protocol::ProtocolMessageType::ActionWindowOpenedEvent => {
            let Ok(event) = serde_json::from_value::<protocol::ActionWindowOpened>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            let contribution = hand
                .betting_round
                .contributions_by_player_id
                .get(&event.player_id)
                .copied()
                .unwrap_or_default();
            hand.cycle_phase = domain::HandCyclePhase::AwaitingAction;
            hand.action_window = Some(domain::ActionWindow {
                action_window_id: event.action_window_id,
                player_id: event.player_id,
                seat_index: event.seat_index,
                legal_actions: event.legal_actions,
                call_amount: event.call_amount,
                min_raise_to: event.min_raise_to,
                max_raise_to: event.max_raise_to,
                deadline_epoch_ms: event.deadline_epoch_ms,
            });
            hand.betting_round.current_bet = hand
                .betting_round
                .current_bet
                .max(contribution.saturating_add(event.call_amount));
            hand.betting_round.min_raise_to = event.min_raise_to;
            hand.betting_round.max_raise_to = event.max_raise_to;
        }
        protocol::ProtocolMessageType::PlayerActionCommittedEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::PlayerActionCommitted>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            let previous_call_amount = hand
                .action_window
                .as_ref()
                .filter(|window| window.player_id == event.player_id)
                .map(|window| window.call_amount)
                .unwrap_or_default();
            let previous_contribution = hand
                .betting_round
                .contributions_by_player_id
                .get(&event.player_id)
                .copied()
                .unwrap_or_default();
            let next_contribution = match event.action_type {
                domain::ActionType::Fold | domain::ActionType::Check => previous_contribution,
                domain::ActionType::Call => {
                    previous_contribution.saturating_add(previous_call_amount)
                }
                domain::ActionType::Bet | domain::ActionType::Raise | domain::ActionType::AllIn => {
                    event
                        .raise_to_amount
                        .unwrap_or(previous_contribution.saturating_add(previous_call_amount))
                }
            };
            let additional = next_contribution.saturating_sub(previous_contribution);
            hand.betting_round
                .contributions_by_player_id
                .insert(event.player_id.clone(), next_contribution);
            hand.betting_round.pot_size = hand.betting_round.pot_size.saturating_add(additional);
            hand.betting_round.current_bet = hand.betting_round.current_bet.max(next_contribution);
            if let Some(participation) = hand.participation_by_player_id.get_mut(&event.player_id) {
                *participation = match event.action_type {
                    domain::ActionType::Fold => domain::HandParticipationState::Folded,
                    domain::ActionType::AllIn => domain::HandParticipationState::AllIn,
                    _ => domain::HandParticipationState::Active,
                };
            }
            if let Some(seat) = state
                .seats
                .iter_mut()
                .find(|seat| seat.participant_id.as_deref() == Some(event.player_id.as_str()))
            {
                if let Some(chips) = seat.chip_count.as_mut() {
                    *chips = chips.saturating_sub(additional);
                    if *chips == 0 && event.action_type != domain::ActionType::Fold {
                        if let Some(participation) =
                            hand.participation_by_player_id.get_mut(&event.player_id)
                        {
                            *participation = domain::HandParticipationState::AllIn;
                        }
                    }
                }
            }
            hand.action_window = None;
        }
        protocol::ProtocolMessageType::StreetRevealedEvent => {
            let Ok(event) = serde_json::from_value::<protocol::StreetRevealed>(payload.clone())
            else {
                return;
            };
            let Some(hand) = state.current_hand.as_mut() else {
                return;
            };
            hand.board_cards = event.board_cards;
            if let Some(street) = parse_protocol_street(&event.street) {
                hand.street = street;
                hand.betting_round.street = street;
            }
            hand.cycle_phase = domain::HandCyclePhase::AwaitingAction;
            hand.betting_round.current_bet = 0;
            hand.betting_round.min_raise_to = None;
            hand.betting_round.max_raise_to = None;
            hand.action_window = None;
        }
        protocol::ProtocolMessageType::HandResultCommittedEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::HandResultCommitted>(payload.clone())
            else {
                return;
            };
            if !state
                .hand_results
                .iter()
                .any(|result| result.hand_number == event.hand_number)
            {
                state.hand_results.push(event.result.clone());
            }
            if event.result.final_stack_by_player_id.is_empty() {
                let mut payouts = std::collections::BTreeMap::<String, u32>::new();
                for pot in &event.result.pot_summaries {
                    let Some(split_winner_count) = (!pot.winner_player_ids.is_empty())
                        .then_some(pot.winner_player_ids.len() as u32)
                    else {
                        continue;
                    };
                    let split_amount = pot.amount / split_winner_count;
                    for winner in &pot.winner_player_ids {
                        *payouts.entry(winner.clone()).or_default() += split_amount;
                    }
                    if let Some(odd_chip_player_id) = pot.odd_chip_awarded_to.as_ref() {
                        *payouts.entry(odd_chip_player_id.clone()).or_default() +=
                            pot.odd_chip_count;
                    }
                }
                for seat in &mut state.seats {
                    let Some(player_id) = seat.participant_id.as_ref() else {
                        continue;
                    };
                    if let Some(payout) = payouts.get(player_id) {
                        let chips = seat.chip_count.get_or_insert(0);
                        *chips = chips.saturating_add(*payout);
                    }
                }
            } else {
                for seat in &mut state.seats {
                    let Some(player_id) = seat.participant_id.as_ref() else {
                        continue;
                    };
                    if let Some(final_stack) = event.result.final_stack_by_player_id.get(player_id)
                    {
                        seat.chip_count = Some(*final_stack);
                    }
                }
            }
            if let Some(hand) = state.current_hand.as_mut() {
                if hand.hand_number == event.hand_number {
                    hand.board_cards = event.result.board_cards.clone();
                    for (player_id, cards) in &event.result.revealed_hands_by_player_id {
                        hand.hole_cards_by_player_id
                            .insert(player_id.clone(), cards.clone());
                    }
                    hand.cycle_phase = domain::HandCyclePhase::Settlement;
                    hand.action_window = None;
                }
            }
        }
        protocol::ProtocolMessageType::EliminationEvent => {
            let Ok(event) = serde_json::from_value::<protocol::EliminationEvent>(payload.clone())
            else {
                return;
            };
            if !state
                .placements
                .iter()
                .any(|entry| entry.player_id == event.player_id)
            {
                state.placements.push(domain::PlacementEntry {
                    player_id: event.player_id.clone(),
                    place: event.place,
                    busted_at_hand_number: state.current_hand.as_ref().map(|hand| hand.hand_number),
                });
            }
            if let Some(participant) = state.participants.get_mut(&event.player_id) {
                participant.state = domain::ParticipantState::EliminatedObserver;
            }
            if let Some(seat) = state
                .seats
                .iter_mut()
                .find(|seat| seat.participant_id.as_deref() == Some(event.player_id.as_str()))
            {
                seat.tournament_state = domain::TournamentSeatState::EliminatedObserver;
                seat.chip_count = Some(0);
            }
        }
        protocol::ProtocolMessageType::TournamentCompleteEvent => {
            let Ok(event) =
                serde_json::from_value::<protocol::TournamentCompleteEvent>(payload.clone())
            else {
                return;
            };
            let winner_player_id = event.winner_player_id;
            let placements = event.placements;
            let final_stacks = state
                .hand_results
                .last()
                .map(|result| result.final_stack_by_player_id.clone())
                .unwrap_or_default();

            state.phase = domain::TournamentPhase::Complete;
            state.current_hand = None;
            state.placements = placements.clone();
            for seat in &mut state.seats {
                seat.marker = None;
            }

            for placement in placements {
                let is_winner = placement.player_id == winner_player_id;
                if let Some(participant) = state.participants.get_mut(&placement.player_id) {
                    participant.state = if is_winner {
                        domain::ParticipantState::Active
                    } else {
                        domain::ParticipantState::EliminatedObserver
                    };
                }
                if let Some(seat) = state.seats.iter_mut().find(|seat| {
                    seat.participant_id.as_deref() == Some(placement.player_id.as_str())
                }) {
                    if is_winner {
                        seat.tournament_state = domain::TournamentSeatState::Active;
                        if let Some(final_stack) = final_stacks.get(&placement.player_id) {
                            seat.chip_count = Some(*final_stack);
                        }
                        seat.marker = Some(domain::SeatMarker::Dealer);
                    } else {
                        seat.tournament_state = domain::TournamentSeatState::EliminatedObserver;
                        seat.chip_count = Some(0);
                    }
                }
            }
        }
        _ => {
            let _ = local_player_id;
        }
    }
}

pub(crate) fn apply_private_hole_cards_to_snapshot(
    state: &mut domain::TournamentState,
    event: &protocol::PrivateHoleCardsEvent,
) {
    let Some(hand) = state.current_hand.as_mut() else {
        return;
    };
    hand.hole_cards_by_player_id
        .insert(event.recipient_player_id.clone(), event.hole_cards.clone());
}
