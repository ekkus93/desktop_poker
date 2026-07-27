#!/usr/bin/env python3
"""Apply automatic runout when only one contender remains able to act."""

from __future__ import annotations

from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    occurrences = content.count(old)
    if occurrences != 1:
        raise RuntimeError(
            f"expected exactly one match in {path}, found {occurrences}"
        )
    file_path.write_text(content.replace(old, new, 1), encoding="utf-8")


def append_once(path: str, marker: str, addition: str) -> None:
    file_path = Path(path)
    content = file_path.read_text(encoding="utf-8")
    if marker in content:
        raise RuntimeError(f"repair marker already present in {path}")
    file_path.write_text(content.rstrip() + "\n\n" + addition.rstrip() + "\n", encoding="utf-8")


def main() -> None:
    replace_once(
        "crates/poker-core/src/tournament/controller_core.rs",
        "        if self.remaining_contenders().len() <= 1 {\n"
        "            self.settle_current_hand(now_ms)?;\n"
        "            return Ok(());\n"
        "        }\n\n"
        "        if self.players_who_can_act().is_empty() {\n"
        "            if self.remaining_contenders().iter().all(|player_id| {\n"
        "                self.participation(player_id)\n"
        "                    .is_some_and(|state| state == HandParticipationState::AllIn)\n"
        "            }) {\n"
        "                self.reveal_remaining_board()?;\n"
        "                self.settle_current_hand(now_ms)?;\n"
        "            } else {\n"
        "                self.advance_street_or_showdown(now_ms)?;\n"
        "            }\n"
        "            return Ok(());\n"
        "        }\n",
        "        let remaining_contenders = self.remaining_contenders();\n"
        "        if remaining_contenders.len() <= 1 {\n"
        "            self.settle_current_hand(now_ms)?;\n"
        "            return Ok(());\n"
        "        }\n\n"
        "        if self.players_who_can_act().is_empty() {\n"
        "            let active_contender_count = remaining_contenders\n"
        "                .iter()\n"
        "                .filter(|player_id| {\n"
        "                    self.participation(player_id) == Some(HandParticipationState::Active)\n"
        "                })\n"
        "                .count();\n"
        "            if active_contender_count <= 1 {\n"
        "                self.reveal_remaining_board()?;\n"
        "                self.settle_current_hand(now_ms)?;\n"
        "            } else {\n"
        "                self.advance_street_or_showdown(now_ms)?;\n"
        "            }\n"
        "            return Ok(());\n"
        "        }\n",
    )

    append_once(
        "crates/poker-core/src/tournament/tests/hand.rs",
        "fn matched_all_in_with_one_actionable_player_runs_out_and_settles()",
        "#[test]\n"
        "fn matched_all_in_with_one_actionable_player_runs_out_and_settles() {\n"
        "    let mut controller = started_two_player_controller();\n"
        "    // Preserve the tournament chip total while making p1's all-in smaller\n"
        "    // than p2's remaining stack. After p2 calls, p2 still has chips but no\n"
        "    // opponent can respond to another wager. The board must run out.\n"
        "    controller.set_player_stack(\"p1\", 850).unwrap();\n"
        "    controller.set_player_stack(\"p2\", 1_000).unwrap();\n\n"
        "    let first_window = action_window(&controller);\n"
        "    assert_eq!(first_window.player_id, \"p1\");\n"
        "    controller\n"
        "        .apply_action(\"p1\".to_string(), ActionType::AllIn, None, 0)\n"
        "        .unwrap();\n\n"
        "    let response_window = action_window(&controller);\n"
        "    assert_eq!(response_window.player_id, \"p2\");\n"
        "    assert!(response_window.legal_actions.contains(&ActionType::Call));\n"
        "    controller\n"
        "        .apply_action(\"p2\".to_string(), ActionType::Call, None, 0)\n"
        "        .unwrap();\n\n"
        "    assert_eq!(\n"
        "        controller.state().hand_results.len(),\n"
        "        1,\n"
        "        \"a matched all-in must settle without opening a lone action window\"\n"
        "    );\n"
        "    assert_eq!(controller.state().hand_results[0].board_cards.len(), 5);\n"
        "    assert!(\n"
        "        controller\n"
        "            .state()\n"
        "            .current_hand\n"
        "            .as_ref()\n"
        "            .and_then(|hand| hand.action_window.as_ref())\n"
        "            .is_none(),\n"
        "        \"no action window may remain when only one contender can act\"\n"
        "    );\n"
        "}\n",
    )


if __name__ == "__main__":
    main()
