#!/usr/bin/env python3
"""Run a complete release tournament with two in-process GGUF NPC players.

The harness drives the real Tauri release backend through tauri-driver. It configures
an embedded GGUF provider, seats two profile-backed NPCs, starts a three-player
tournament, drives only the human host's turns, and lets the production NPC runner
make every bot decision.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import time
from pathlib import Path
from typing import Any

from runtime_multi_instance_smoke import WebDriverClient, capture, invoke
from runtime_webdriver_smoke import wait_for, wait_for_route

VIEW_PAYLOAD = {"viewerMode": "local"}
BOT_PROFILES = (
    ("Bot Alpha", "aggressive", "aggressive-alice"),
    ("Bot Beta", "conservative", "balanced-sam"),
)


def table_view(client: WebDriverClient) -> dict[str, Any]:
    value = invoke(client, "get_table_view", VIEW_PAYLOAD)
    if not isinstance(value, dict):
        raise AssertionError(f"get_table_view returned {value!r}")
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


def collect_events(
    view: dict[str, Any],
    seen_sequences: set[int],
    npc_actions: list[dict[str, Any]],
) -> None:
    feed = view.get("eventFeed")
    if not isinstance(feed, list):
        raise AssertionError("table view has no eventFeed list")

    for event in reversed(feed):
        if not isinstance(event, dict):
            continue
        sequence = event.get("sequence")
        if not isinstance(sequence, int) or sequence in seen_sequences:
            continue
        seen_sequences.add(sequence)
        message = str(event.get("message") or "")
        kind = str(event.get("kind") or "")
        for display_name, _, profile_id in BOT_PROFILES:
            if kind == "Player action" and message.startswith(f"{display_name} "):
                npc_actions.append(
                    {
                        "sequence": sequence,
                        "displayName": display_name,
                        "profileId": profile_id,
                        "message": message,
                        "handNumber": view.get("currentHandNumber"),
                    }
                )
                break


def submit_host_action(client: WebDriverClient, view: dict[str, Any]) -> str:
    tray = view.get("actionTray")
    if not isinstance(tray, dict):
        raise AssertionError("submit_host_action called without a local action tray")
    legal = tray.get("legalActions")
    if not isinstance(legal, list):
        raise AssertionError(f"action tray has no legalActions list: {tray!r}")

    hand_number = int(view.get("currentHandNumber") or 0)
    # Deliberately fold the host's first opportunity so the tournament must settle
    # at least two hands. After that, prefer all-in to keep CI bounded.
    if hand_number == 1 and "fold" in legal:
        action_kind = "fold"
        raise_to_amount = None
    elif "allIn" in legal:
        action_kind = "allIn"
        raise_to_amount = None
    elif "betOrRaise" in legal:
        maximum = tray.get("maxRaiseTo")
        minimum = tray.get("minRaiseTo")
        raise_to_amount = maximum if isinstance(maximum, int) else minimum
        if not isinstance(raise_to_amount, int):
            raise AssertionError(f"raise action has no legal amount: {tray!r}")
        action_kind = "betOrRaise"
    elif "checkOrCall" in legal:
        action_kind = "checkOrCall"
        raise_to_amount = None
    elif "fold" in legal:
        action_kind = "fold"
        raise_to_amount = None
    else:
        raise AssertionError(f"host has no supported legal action: {tray!r}")

    invoke(
        client,
        "submit_table_action",
        {
            "viewerMode": "local",
            "actionKind": action_kind,
            "raiseToAmount": raise_to_amount,
        },
    )
    return (
        f"hand {hand_number}: {action_kind}"
        if raise_to_amount is None
        else f"hand {hand_number}: {action_kind} to {raise_to_amount}"
    )


def configure_and_start(
    client: WebDriverClient, model_path: Path
) -> tuple[dict[str, Any], dict[str, Any]]:
    bootstrap = invoke(client, "get_bootstrap_state")
    if not isinstance(bootstrap, dict):
        raise AssertionError(f"get_bootstrap_state returned {bootstrap!r}")

    profiles = invoke(client, "list_npc_profiles")
    if not isinstance(profiles, dict):
        raise AssertionError(f"list_npc_profiles returned {profiles!r}")
    available_ids = {
        str(profile.get("id"))
        for profile in profiles.get("profiles", [])
        if isinstance(profile, dict)
    }
    required_ids = {profile_id for _, _, profile_id in BOT_PROFILES}
    missing = sorted(required_ids - available_ids)
    if missing:
        raise AssertionError(f"required built-in NPC profiles are missing: {missing}")

    invoke(
        client,
        "set_llm_provider_config",
        {
            "config": {
                "provider": "embeddedLocal",
                "apiKey": None,
                "endpointUrl": None,
                "model": str(model_path),
            }
        },
    )
    configured = invoke(client, "get_bootstrap_state")
    if not isinstance(configured, dict):
        raise AssertionError(f"configured bootstrap returned {configured!r}")
    if configured.get("llmProviderType") != "embeddedLocal":
        raise AssertionError(f"embedded provider was not selected: {configured!r}")
    if configured.get("llmApiKeyConfigured") is not True:
        raise AssertionError("embedded provider was not reported usable")

    host_address = invoke(client, "resolve_host_lan_address")
    if not isinstance(host_address, str) or not host_address.strip():
        raise AssertionError(f"resolve_host_lan_address returned {host_address!r}")

    status = invoke(
        client,
        "start_host_session",
        {
            "request": {
                "hostAddress": host_address,
                "hostPort": 43838,
                "tournamentName": "Embedded GGUF NPC CI Tournament",
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
                        "profileId": profile_id,
                    }
                    for display_name, style, profile_id in BOT_PROFILES
                ]
            }
        },
    )
    if not isinstance(status, dict):
        raise AssertionError(f"add_npc_players returned {status!r}")

    participants = participant_map(status)
    if set(participants) != {"CI Human Host", "Bot Alpha", "Bot Beta"}:
        raise AssertionError(f"unexpected participant set: {sorted(participants)}")
    for display_name, _, _ in BOT_PROFILES:
        participant = participants[display_name]
        if participant.get("seatIndex") is None or participant.get("isReady") is not True:
            raise AssertionError(f"NPC was not seated and ready: {participant!r}")

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
    model_path: Path,
    evidence_dir: Path,
    timeout_seconds: float,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    client = WebDriverClient(driver_url)
    steps: list[dict[str, str]] = []
    npc_actions: list[dict[str, Any]] = []
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

        bootstrap, start_status = configure_and_start(client, model_path)
        record("embedded provider loaded and two profile-backed NPCs were seated ready")
        if start_status.get("activeSeatCount") != 3:
            raise AssertionError(f"expected 3 active seats: {start_status!r}")

        client.navigate("/table")
        wait_for_route(client, "/table", "Main Table")
        initial_view = wait_for(
            "running embedded-NPC hand",
            lambda: (
                view
                if (view := table_view(client)).get("tournamentPhase") == "running"
                and view.get("currentHandNumber") == 1
                else None
            ),
            timeout=45.0,
        )
        collect_events(initial_view, seen_sequences, npc_actions)
        capture(client, evidence_dir, "embedded-tournament-start")
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
            collect_events(view, seen_sequences, npc_actions)
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
                    "embedded tournament made no observable progress for 35 seconds: "
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
                f"embedded tournament did not complete within {timeout_seconds:.0f} seconds"
            )

        collect_events(final_view, seen_sequences, npc_actions)
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
                1 for action in npc_actions if action["displayName"] == display_name
            )
            for display_name, _, _ in BOT_PROFILES
        }
        missing_actions = [name for name, count in action_counts.items() if count < 1]
        if missing_actions:
            raise AssertionError(
                f"NPCs completed without an observed committed action: {missing_actions}; "
                f"actions={npc_actions!r}"
            )
        if len(npc_actions) < 3:
            raise AssertionError(
                f"expected at least three embedded NPC decisions, observed {len(npc_actions)}"
            )

        capture(client, evidence_dir, "embedded-tournament-complete")
        (evidence_dir / "final-table-view.json").write_text(
            json.dumps(final_view, indent=2) + "\n", encoding="utf-8"
        )
        (evidence_dir / "embedded-npc-actions.json").write_text(
            json.dumps(npc_actions, indent=2) + "\n", encoding="utf-8"
        )
        record(
            f"tournament completed across {len(history)} hands with {len(npc_actions)} committed embedded NPC actions"
        )
        record("both embedded NPC identities and profile assignments produced live actions")
        record("final standings contain the human host and both embedded NPC players")

        return {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "model": str(model_path),
            "modelSha256": os.environ.get("MODEL_SHA256"),
            "instanceId": bootstrap.get("instanceId"),
            "profileDirectory": bootstrap.get("profileDirectory"),
            "completedHands": len(history),
            "npcActionCount": len(npc_actions),
            "npcActionCounts": action_counts,
            "npcActions": npc_actions,
            "hostActions": host_actions,
            "finalStandings": standings,
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001 - preserve complete runtime evidence
        steps.append({"name": "embedded NPC tournament", "result": "FAIL"})
        failure: dict[str, Any] = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "model": str(model_path),
            "error": f"{type(error).__name__}: {error}",
            "npcActions": npc_actions,
            "hostActions": host_actions,
            "steps": steps,
        }
        try:
            if client.session_id is not None:
                capture(client, evidence_dir, "embedded-tournament-failure")
                failure["lastTableView"] = table_view(client)
        except Exception as evidence_error:  # noqa: BLE001
            failure["evidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--driver-url", default="http://127.0.0.1:4584")
    parser.add_argument("--model", required=True, type=Path)
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=Path("embedded-tournament-evidence"),
    )
    parser.add_argument("--timeout-seconds", type=float, default=240.0)
    args = parser.parse_args()

    application = args.application.resolve()
    model_path = args.model.resolve()
    if not application.is_file():
        parser.error(f"application does not exist: {application}")
    if not model_path.is_file() or model_path.suffix.lower() != ".gguf":
        parser.error(f"model is not a GGUF file: {model_path}")

    result_path = args.evidence_dir / "embedded-npc-tournament-result.json"
    try:
        result = run(
            application,
            args.driver_url,
            model_path,
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
