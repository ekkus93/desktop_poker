#!/usr/bin/env python3
"""Run a complete release tournament with two rule-based NPC players.

The harness drives the real Tauri release backend through tauri-driver. It seats
NPCs without profile IDs, so the production runner must use the explicit
rule-based strategy rather than an LLM provider or fallback path.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any

from runtime_embedded_npc_tournament import submit_host_action
from runtime_multi_instance_smoke import WebDriverClient, capture, invoke
from runtime_webdriver_smoke import wait_for, wait_for_route

VIEW_PAYLOAD = {"viewerMode": "local"}
BOT_CONFIGS = (
    ("Rule Bot Alpha", "aggressive"),
    ("Rule Bot Beta", "conservative"),
)


def table_view(client: WebDriverClient) -> dict[str, Any]:
    value = invoke(client, "get_table_view", VIEW_PAYLOAD)
    if not isinstance(value, dict):
        raise AssertionError(f"get_table_view returned {value!r}")
    return value


def debug_state(client: WebDriverClient) -> dict[str, Any]:
    value = invoke(client, "get_debug_state", VIEW_PAYLOAD)
    if not isinstance(value, dict):
        raise AssertionError(f"get_debug_state returned {value!r}")
    return value


def participant_map(status: dict[str, Any]) -> dict[str, dict[str, Any]]:
    participants = status.get("participants")
    if not isinstance(participants, list):
        raise AssertionError(f"session status has no participants list: {status!r}")
    return {
        str(participant.get("displayName")): participant
        for participant in participants
        if isinstance(participant, dict)
    }


def collect_bot_actions(
    view: dict[str, Any],
    seen_sequences: set[int],
    bot_actions: list[dict[str, Any]],
) -> None:
    feed = view.get("eventFeed")
    if not isinstance(feed, list):
        raise AssertionError("table view has no eventFeed list")

    bot_names = {display_name for display_name, _ in BOT_CONFIGS}
    for event in reversed(feed):
        if not isinstance(event, dict):
            continue
        sequence = event.get("sequence")
        if not isinstance(sequence, int) or sequence in seen_sequences:
            continue
        seen_sequences.add(sequence)
        message = str(event.get("message") or "")
        if str(event.get("kind") or "") != "Player action":
            continue
        for display_name in bot_names:
            if message.startswith(f"{display_name} "):
                bot_actions.append(
                    {
                        "sequence": sequence,
                        "displayName": display_name,
                        "message": message,
                        "handNumber": view.get("currentHandNumber"),
                    }
                )
                break


def configure_and_start(
    client: WebDriverClient,
) -> tuple[dict[str, Any], dict[str, Any]]:
    bootstrap = invoke(client, "get_bootstrap_state")
    if not isinstance(bootstrap, dict):
        raise AssertionError(f"get_bootstrap_state returned {bootstrap!r}")
    if bootstrap.get("llmApiKeyConfigured") is True:
        raise AssertionError(
            "rule-based test profile unexpectedly has a usable LLM provider configured"
        )

    host_address = invoke(client, "resolve_host_lan_address")
    if not isinstance(host_address, str) or not host_address.strip():
        raise AssertionError(f"resolve_host_lan_address returned {host_address!r}")

    status = invoke(
        client,
        "start_host_session",
        {
            "request": {
                "hostAddress": host_address,
                "hostPort": 43848,
                "tournamentName": "Rule-Based NPC CI Tournament",
                "maxPlayers": 3,
                "startingStack": 100,
                "blindPresetId": "fast",
                "turnTimerSeconds": 15,
                "displayName": "CI Human Host",
            }
        },
    )
    if not isinstance(status, dict):
        raise AssertionError(f"start_host_session returned {status!r}")

    status = invoke(
        client,
        "add_npc_players",
        {
            "request": {
                "npcs": [
                    {
                        "displayName": display_name,
                        "style": style,
                        "profileId": None,
                    }
                    for display_name, style in BOT_CONFIGS
                ]
            }
        },
    )
    if not isinstance(status, dict):
        raise AssertionError(f"add_npc_players returned {status!r}")

    expected_names = {"CI Human Host", *(name for name, _ in BOT_CONFIGS)}
    participants = participant_map(status)
    if set(participants) != expected_names:
        raise AssertionError(f"unexpected participant set: {sorted(participants)}")
    for display_name, _ in BOT_CONFIGS:
        participant = participants[display_name]
        if participant.get("seatIndex") is None or participant.get("isReady") is not True:
            raise AssertionError(f"rule-based NPC was not seated and ready: {participant!r}")

    status = invoke(
        client,
        "host_set_lobby_ready_state",
        {"request": {"isReady": True}},
    )
    if not isinstance(status, dict):
        raise AssertionError(f"host_set_lobby_ready_state returned {status!r}")
    not_ready = [
        participant
        for participant in participant_map(status).values()
        if participant.get("isReady") is not True
    ]
    if not_ready:
        raise AssertionError(f"participants remained unready: {not_ready!r}")

    status = invoke(client, "host_start_tournament")
    if not isinstance(status, dict) or status.get("phase") != "running":
        raise AssertionError(f"host_start_tournament did not enter running: {status!r}")
    return bootstrap, status


def run(
    application: Path,
    driver_url: str,
    evidence_dir: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    client = WebDriverClient(driver_url)
    steps: list[dict[str, str]] = []
    bot_actions: list[dict[str, Any]] = []
    host_actions: list[str] = []
    seen_sequences: set[int] = set()

    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    try:
        client.wait_until_ready()
        record("tauri-driver became ready")
        client.start_session(application)
        wait_for_route(client, "/", "Choose a table")
        record("release binary created a real Tauri/WebKit session")

        bootstrap, start_status = configure_and_start(client)
        if start_status.get("activeSeatCount") != 3:
            raise AssertionError(f"expected 3 active seats: {start_status!r}")
        record("two unprofiled rule-based NPCs were seated and ready")

        client.navigate("/table")
        wait_for_route(client, "/table", "Main Table")
        initial_view = wait_for(
            "running rule-based NPC hand",
            lambda: (
                view
                if (view := table_view(client)).get("tournamentPhase") == "running"
                and view.get("currentHandNumber") == 1
                else None
            ),
            timeout=45.0,
        )
        collect_bot_actions(initial_view, seen_sequences, bot_actions)
        capture(client, evidence_dir, "rule-based-tournament-start")
        (evidence_dir / "initial-table-view.json").write_text(
            json.dumps(initial_view, indent=2) + "\n", encoding="utf-8"
        )
        record("release table entered hand 1 with the production NPC runner active")

        deadline = time.monotonic() + timeout_seconds
        last_progress_at = time.monotonic()
        last_progress_key: tuple[Any, ...] | None = None
        final_view = initial_view

        while time.monotonic() < deadline:
            view = table_view(client)
            collect_bot_actions(view, seen_sequences, bot_actions)
            final_view = view

            progress_key = (
                view.get("tournamentPhase"),
                view.get("currentHandNumber"),
                len(view.get("handHistory") or []),
                max(seen_sequences, default=0),
                view.get("actionOwnerLabel"),
            )
            if progress_key != last_progress_key:
                last_progress_key = progress_key
                last_progress_at = time.monotonic()
            elif time.monotonic() - last_progress_at > 35.0:
                raise AssertionError(
                    "rule-based tournament made no observable progress for 35 seconds: "
                    + json.dumps(view, sort_keys=True)
                )

            if view.get("tournamentPhase") == "complete":
                break
            if isinstance(view.get("actionTray"), dict):
                host_actions.append(submit_host_action(client, view))
                time.sleep(0.1)
            else:
                time.sleep(0.15)
        else:
            raise AssertionError(
                f"rule-based tournament did not complete within {timeout_seconds:.0f} seconds"
            )

        collect_bot_actions(final_view, seen_sequences, bot_actions)
        history = final_view.get("handHistory")
        standings = final_view.get("standings")
        if not isinstance(history, list) or len(history) < 2:
            raise AssertionError(f"expected at least two settled hands: {history!r}")
        if not isinstance(standings, list) or len(standings) != 3:
            raise AssertionError(f"expected three final standings: {standings!r}")
        if final_view.get("actionTray") is not None:
            raise AssertionError("completed tournament retained a local action tray")

        action_counts = {
            display_name: sum(
                1 for action in bot_actions if action["displayName"] == display_name
            )
            for display_name, _ in BOT_CONFIGS
        }
        missing_actions = [name for name, count in action_counts.items() if count < 1]
        if missing_actions:
            raise AssertionError(
                f"rule-based NPCs completed without observed committed actions: {missing_actions}; "
                f"actions={bot_actions!r}"
            )
        if len(bot_actions) < 3:
            raise AssertionError(
                f"expected at least three rule-based NPC decisions, observed {len(bot_actions)}"
            )

        diagnostics = debug_state(client)
        if diagnostics.get("lastNpcActionError") is not None:
            raise AssertionError(
                f"NPC runner reported an action error: {diagnostics['lastNpcActionError']!r}"
            )
        if diagnostics.get("lastLlmFallback") is not None:
            raise AssertionError(
                f"rule-based tournament unexpectedly recorded an LLM fallback: "
                f"{diagnostics['lastLlmFallback']!r}"
            )

        capture(client, evidence_dir, "rule-based-tournament-complete")
        (evidence_dir / "final-table-view.json").write_text(
            json.dumps(final_view, indent=2) + "\n", encoding="utf-8"
        )
        (evidence_dir / "rule-based-npc-actions.json").write_text(
            json.dumps(bot_actions, indent=2) + "\n", encoding="utf-8"
        )
        (evidence_dir / "debug-state.json").write_text(
            json.dumps(diagnostics, indent=2) + "\n", encoding="utf-8"
        )
        record(
            f"tournament completed across {len(history)} hands with "
            f"{len(bot_actions)} committed rule-based NPC actions"
        )
        record("both rule-based NPC identities produced live accepted actions")
        record("NPC diagnostics and LLM fallback state remained clear")
        record("final standings contain the human host and both rule-based NPC players")

        return {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "instanceId": bootstrap.get("instanceId"),
            "profileDirectory": bootstrap.get("profileDirectory"),
            "completedHands": len(history),
            "npcActionCount": len(bot_actions),
            "npcActionCounts": action_counts,
            "npcActions": bot_actions,
            "hostActions": host_actions,
            "finalStandings": standings,
            "lastNpcActionError": diagnostics.get("lastNpcActionError"),
            "lastLlmFallback": diagnostics.get("lastLlmFallback"),
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001 - preserve complete runtime evidence
        steps.append({"name": "rule-based NPC tournament", "result": "FAIL"})
        failure: dict[str, Any] = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "error": f"{type(error).__name__}: {error}",
            "npcActions": bot_actions,
            "hostActions": host_actions,
            "steps": steps,
        }
        try:
            if client.session_id is not None:
                capture(client, evidence_dir, "rule-based-tournament-failure")
                failure["lastTableView"] = table_view(client)
                failure["debugState"] = debug_state(client)
        except Exception as evidence_error:  # noqa: BLE001
            failure["evidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--driver-url", default="http://127.0.0.1:4594")
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=Path("rule-based-tournament-evidence"),
    )
    parser.add_argument("--timeout-seconds", type=float, default=240.0)
    args = parser.parse_args()

    application = args.application.resolve()
    if not application.is_file():
        parser.error(f"application does not exist: {application}")

    result_path = args.evidence_dir / "rule-based-npc-tournament-result.json"
    try:
        result = run(
            application,
            args.driver_url,
            args.evidence_dir,
            args.timeout_seconds,
        )
        result_path.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
        return 0
    except Exception as error:  # noqa: BLE001
        args.evidence_dir.mkdir(parents=True, exist_ok=True)
        result_path.write_text(f"{error}\n", encoding="utf-8")
        print(error, file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
