#!/usr/bin/env python3
"""Repair tournament-completion event publication and client projection."""

from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    occurrences = content.count(old)
    if occurrences != 1:
        raise RuntimeError(f"expected one match in {path}, found {occurrences}")
    file_path.write_text(content.replace(old, new, 1), encoding="utf-8")


def append_once(path: str, marker: str, addition: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    if marker in content:
        raise RuntimeError(f"repair marker already present in {path}")
    file_path.write_text(content.rstrip() + "\n\n" + addition.rstrip() + "\n", encoding="utf-8")


def main() -> None:
    replace_once(
        "src-tauri/src/networking/runtime/events.rs",
        "    collections::HashMap,\n",
        "    collections::{HashMap, HashSet},\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/events.rs",
        "#[allow(clippy::too_many_arguments)]\n"
        "pub(crate) fn publish_runtime_transition(\n",
        "pub(crate) fn infer_new_elimination_events(\n"
        "    before: &TournamentState,\n"
        "    after: &TournamentState,\n"
        ") -> Vec<EliminationEvent> {\n"
        "    let previously_placed_player_ids = before\n"
        "        .placements\n"
        "        .iter()\n"
        "        .map(|placement| placement.player_id.as_str())\n"
        "        .collect::<HashSet<_>>();\n\n"
        "    after\n"
        "        .placements\n"
        "        .iter()\n"
        "        .filter(|placement| {\n"
        "            placement.place > 1\n"
        "                && !previously_placed_player_ids.contains(placement.player_id.as_str())\n"
        "        })\n"
        "        .map(|placement| EliminationEvent {\n"
        "            player_id: placement.player_id.clone(),\n"
        "            place: placement.place,\n"
        "        })\n"
        "        .collect()\n"
        "}\n\n"
        "#[allow(clippy::too_many_arguments)]\n"
        "pub(crate) fn publish_runtime_transition(\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/events.rs",
        "    if after.placements.len() > before.placements.len() {\n"
        "        for placement in &after.placements[before.placements.len()..] {\n"
        "            broadcast_public_event_to_clients(\n"
        "                join_payload,\n"
        "                clients,\n"
        "                server_sequence,\n"
        "                host_signing_keys,\n"
        "                public_events,\n"
        "                ProtocolMessageType::EliminationEvent,\n"
        "                &EliminationEvent {\n"
        "                    player_id: placement.player_id.clone(),\n"
        "                    place: placement.place,\n"
        "                },\n"
        "                |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),\n"
        "            )?;\n"
        "        }\n"
        "    }\n",
        "    for elimination in infer_new_elimination_events(before, after) {\n"
        "        broadcast_public_event_to_clients(\n"
        "            join_payload,\n"
        "            clients,\n"
        "            server_sequence,\n"
        "            host_signing_keys,\n"
        "            public_events,\n"
        "            ProtocolMessageType::EliminationEvent,\n"
        "            &elimination,\n"
        "            |player_id| mark_participant_reconnect_eligible(authoritative_state, player_id),\n"
        "        )?;\n"
        "    }\n",
    )

    replace_once(
        "src-tauri/src/app_state/live_events.rs",
        "            state.phase = domain::TournamentPhase::Complete;\n"
        "            state.placements = event.placements;\n"
        "            if let Some(hand) = state.current_hand.as_mut() {\n"
        "                hand.action_window = None;\n"
        "                hand.cycle_phase = domain::HandCyclePhase::Settlement;\n"
        "            }\n",
        "            let winner_player_id = event.winner_player_id;\n"
        "            let placements = event.placements;\n"
        "            let final_stacks = state\n"
        "                .hand_results\n"
        "                .last()\n"
        "                .map(|result| result.final_stack_by_player_id.clone())\n"
        "                .unwrap_or_default();\n\n"
        "            state.phase = domain::TournamentPhase::Complete;\n"
        "            state.current_hand = None;\n"
        "            state.placements = placements.clone();\n"
        "            for seat in &mut state.seats {\n"
        "                seat.marker = None;\n"
        "            }\n\n"
        "            for placement in placements {\n"
        "                let is_winner = placement.player_id == winner_player_id;\n"
        "                if let Some(participant) = state.participants.get_mut(&placement.player_id) {\n"
        "                    participant.state = if is_winner {\n"
        "                        domain::ParticipantState::Active\n"
        "                    } else {\n"
        "                        domain::ParticipantState::EliminatedObserver\n"
        "                    };\n"
        "                }\n"
        "                if let Some(seat) = state.seats.iter_mut().find(|seat| {\n"
        "                    seat.participant_id.as_deref() == Some(placement.player_id.as_str())\n"
        "                }) {\n"
        "                    if is_winner {\n"
        "                        seat.tournament_state = domain::TournamentSeatState::Active;\n"
        "                        if let Some(final_stack) = final_stacks.get(&placement.player_id) {\n"
        "                            seat.chip_count = Some(*final_stack);\n"
        "                        }\n"
        "                        seat.marker = Some(domain::SeatMarker::Dealer);\n"
        "                    } else {\n"
        "                        seat.tournament_state =\n"
        "                            domain::TournamentSeatState::EliminatedObserver;\n"
        "                        seat.chip_count = Some(0);\n"
        "                    }\n"
        "                }\n"
        "            }\n",
    )

    replace_once(
        "src-tauri/src/networking/runtime/tests/misc.rs",
        "use super::super::validate_production_host_ip;\n",
        "use super::super::{infer_new_elimination_events, validate_production_host_ip};\n",
    )

    append_once(
        "src-tauri/src/networking/runtime/tests/misc.rs",
        "fn completion_transition_never_emits_winner_as_eliminated()",
        "#[test]\n"
        "fn completion_transition_never_emits_winner_as_eliminated() {\n"
        "    let mut before = sample_tournament_state(\"table-completion-events\", 91);\n"
        "    before.placements.clear();\n"
        "    let mut after = before.clone();\n"
        "    after.placements = vec![\n"
        "        PlacementEntry {\n"
        "            player_id: \"winner\".to_string(),\n"
        "            place: 1,\n"
        "            busted_at_hand_number: None,\n"
        "        },\n"
        "        PlacementEntry {\n"
        "            player_id: \"loser\".to_string(),\n"
        "            place: 2,\n"
        "            busted_at_hand_number: Some(3),\n"
        "        },\n"
        "    ];\n\n"
        "    let events = infer_new_elimination_events(&before, &after);\n"
        "    assert_eq!(events.len(), 1);\n"
        "    assert_eq!(events[0].player_id, \"loser\");\n"
        "    assert_eq!(events[0].place, 2);\n\n"
        "    before.placements = vec![after.placements[1].clone()];\n"
        "    let winner_only_added = infer_new_elimination_events(&before, &after);\n"
        "    assert!(\n"
        "        winner_only_added.is_empty(),\n"
        "        \"adding the first-place placement must not emit an elimination\"\n"
        "    );\n"
        "}\n",
    )

    append_once(
        "src-tauri/src/app_state/tests/units.rs",
        "fn tournament_complete_clears_hand_and_reconciles_final_placements()",
        "#[test]\n"
        "fn tournament_complete_clears_hand_and_reconciles_final_placements() {\n"
        "    let mut state = snapshot_test_state();\n"
        "    let mut final_result = state.hand_results[0].clone();\n"
        "    final_result.hand_number = 9;\n"
        "    final_result.final_stack_by_player_id = [\n"
        "        (\"showdown\".to_string(), 1_500),\n"
        "        (\"folded\".to_string(), 0),\n"
        "    ]\n"
        "    .into_iter()\n"
        "    .collect();\n"
        "    state.hand_results.push(final_result);\n\n"
        "    state.participants.get_mut(\"showdown\").unwrap().state =\n"
        "        ParticipantState::EliminatedObserver;\n"
        "    state.seats[1].tournament_state = TournamentSeatState::EliminatedObserver;\n"
        "    state.seats[1].chip_count = Some(0);\n\n"
        "    let placements = vec![\n"
        "        crate::domain::PlacementEntry {\n"
        "            player_id: \"showdown\".to_string(),\n"
        "            place: 1,\n"
        "            busted_at_hand_number: None,\n"
        "        },\n"
        "        crate::domain::PlacementEntry {\n"
        "            player_id: \"folded\".to_string(),\n"
        "            place: 2,\n"
        "            busted_at_hand_number: Some(9),\n"
        "        },\n"
        "    ];\n"
        "    let payload = serde_json::to_value(protocol::TournamentCompleteEvent {\n"
        "        winner_player_id: \"showdown\".to_string(),\n"
        "        placements: placements.clone(),\n"
        "    })\n"
        "    .expect(\"completion payload\");\n\n"
        "    apply_public_event_to_snapshot(\n"
        "        &mut state,\n"
        "        \"showdown\",\n"
        "        protocol::ProtocolMessageType::TournamentCompleteEvent,\n"
        "        &payload,\n"
        "    );\n\n"
        "    assert_eq!(state.phase, TournamentPhase::Complete);\n"
        "    assert!(state.current_hand.is_none());\n"
        "    assert_eq!(state.placements, placements);\n"
        "    assert_eq!(state.participants[\"showdown\"].state, ParticipantState::Active);\n"
        "    assert_eq!(\n"
        "        state.participants[\"folded\"].state,\n"
        "        ParticipantState::EliminatedObserver\n"
        "    );\n"
        "    assert_eq!(state.seats[1].tournament_state, TournamentSeatState::Active);\n"
        "    assert_eq!(state.seats[1].chip_count, Some(1_500));\n"
        "    assert_eq!(state.seats[1].marker, Some(SeatMarker::Dealer));\n"
        "    assert_eq!(\n"
        "        state.seats[2].tournament_state,\n"
        "        TournamentSeatState::EliminatedObserver\n"
        "    );\n"
        "    assert_eq!(state.seats[2].chip_count, Some(0));\n"
        "    assert_eq!(state.seats[2].marker, None);\n"
        "}\n",
    )


if __name__ == "__main__":
    main()
