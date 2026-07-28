#!/usr/bin/env python3
"""Exercise explicit release-mode join and host-loss failure behavior.

This harness complements the lower-level reconnect protocol tests. It proves that
unreachable-host joins fail without creating a client session and that permanent
host loss in both lobby and active-hand states becomes terminal, reports an
error, and cannot leave a usable table/action surface.
"""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from typing import Any

from runtime_multi_instance_smoke import (
    WebDriverClient,
    capture,
    invoke,
    participant_by_id,
    wait_for_command,
)
from runtime_webdriver_smoke import wait_for_route

VIEW_PAYLOAD = {"viewerMode": "local"}


def invoke_result(
    client: WebDriverClient,
    command: str,
    payload: dict[str, Any] | None = None,
) -> dict[str, Any]:
    result = client.session_request(
        "POST",
        "/execute/async",
        {
            "script": """
const done = arguments[arguments.length - 1];
const command = arguments[0];
const payload = arguments[1] || {};
if (!window.__TAURI_INTERNALS__ || typeof window.__TAURI_INTERNALS__.invoke !== 'function') {
  done({ ok: false, error: 'Tauri invoke bridge is unavailable.' });
  return;
}
window.__TAURI_INTERNALS__.invoke(command, payload)
  .then((value) => done({ ok: true, value }))
  .catch((error) => done({ ok: false, error: String(error) }));
""",
            "args": [command, payload or {}],
        },
    )
    if not isinstance(result, dict) or not isinstance(result.get("ok"), bool):
        raise AssertionError(f"Tauri command {command!r} returned malformed result: {result!r}")
    return result


def expect_command_error(
    client: WebDriverClient,
    command: str,
    payload: dict[str, Any] | None = None,
) -> str:
    result = invoke_result(client, command, payload)
    if result.get("ok") is True:
        raise AssertionError(
            f"Tauri command {command!r} unexpectedly succeeded: {result.get('value')!r}"
        )
    error = str(result.get("error") or "").strip()
    if not error:
        raise AssertionError(f"Tauri command {command!r} failed without an error message")
    return error


def start_host(client: WebDriverClient, port: int, label: str) -> dict[str, Any]:
    status = invoke(
        client,
        "start_host_session",
        {
            "request": {
                "hostAddress": invoke(client, "resolve_host_lan_address"),
                "hostPort": port,
                "tournamentName": label,
                "maxPlayers": 2,
                "startingStack": 1000,
                "blindPresetId": "fast",
                "turnTimerSeconds": 15,
                "displayName": "Failure Matrix Host",
            }
        },
    )
    if not isinstance(status, dict) or not str(status.get("invite") or "").startswith("pkr1_"):
        raise AssertionError(f"host did not produce a valid invite: {status!r}")
    return status


def join_client(client: WebDriverClient, invite: str) -> dict[str, Any]:
    status = invoke(
        client,
        "join_host_session",
        {
            "request": {
                "joinPayload": invite,
                "displayName": "Failure Matrix Client",
            }
        },
    )
    if not isinstance(status, dict):
        raise AssertionError(f"join_host_session returned {status!r}")
    return status


def wait_for_terminal(client: WebDriverClient) -> dict[str, Any]:
    status = wait_for_command(
        client,
        "get_client_session_status",
        lambda value: isinstance(value, dict)
        and value.get("terminated") is True
        and value.get("reconnecting") is False
        and bool(str(value.get("lastError") or "").strip()),
        timeout=20.0,
    )
    if not isinstance(status, dict):
        raise AssertionError(f"terminal client status was malformed: {status!r}")
    return status


def assert_terminal_table(client: WebDriverClient, expected_error: str) -> dict[str, str]:
    table_error = expect_command_error(client, "get_table_view", VIEW_PAYLOAD)
    action_error = expect_command_error(
        client,
        "submit_table_action",
        {
            "viewerMode": "local",
            "actionKind": "fold",
            "raiseToAmount": None,
        },
    )
    if "disconnected" not in table_error.lower() and expected_error.lower() not in table_error.lower():
        raise AssertionError(
            f"terminal table error did not identify host loss: {table_error!r}; "
            f"status={expected_error!r}"
        )
    return {"tableError": table_error, "actionError": action_error}


def seat_and_start(
    host: WebDriverClient,
    client: WebDriverClient,
    client_status: dict[str, Any],
) -> tuple[dict[str, Any], dict[str, Any]]:
    host_status = invoke(host, "get_host_session_status")
    if not isinstance(host_status, dict):
        raise AssertionError("host status disappeared before seating")

    host_participant = participant_by_id(host_status, "local-player")
    if host_participant is None:
        raise AssertionError("host participant missing")
    if host_participant.get("seatIndex") is None:
        host_status = invoke(host, "host_claim_lobby_seat", {"request": {"seatIndex": 0}})

    client_player_id = str(client_status.get("localPlayerId") or "")
    client_participant = participant_by_id(client_status, client_player_id)
    if client_participant is None:
        raise AssertionError("client participant missing")
    if client_participant.get("seatIndex") is None:
        occupied = {
            participant.get("seatIndex")
            for participant in host_status.get("participants", [])
            if isinstance(participant, dict) and participant.get("seatIndex") is not None
        }
        open_seat = next(index for index in range(2) if index not in occupied)
        client_status = invoke(
            client,
            "client_claim_lobby_seat",
            {"request": {"seatIndex": open_seat}},
        )

    client_status = invoke(
        client,
        "client_set_lobby_ready_state",
        {"request": {"isReady": True}},
    )
    host_status = invoke(
        host,
        "host_set_lobby_ready_state",
        {"request": {"isReady": True}},
    )
    if host_status.get("phase") != "readyCheck":
        host_status = wait_for_command(
            host,
            "get_host_session_status",
            lambda value: isinstance(value, dict) and value.get("phase") == "readyCheck",
            timeout=15.0,
        )
    host_status = invoke(host, "host_start_tournament")
    if host_status.get("phase") != "running":
        raise AssertionError(f"tournament did not start: {host_status!r}")
    wait_for_command(
        client,
        "get_table_view",
        lambda value: isinstance(value, dict)
        and value.get("tournamentPhase") == "running"
        and isinstance(value.get("currentHandNumber"), int),
        payload=VIEW_PAYLOAD,
        timeout=20.0,
    )
    return host_status, client_status


def run(
    application: Path,
    host_url: str,
    client_url: str,
    evidence_dir: Path,
) -> dict[str, Any]:
    evidence_dir.mkdir(parents=True, exist_ok=True)
    host = WebDriverClient(host_url)
    client = WebDriverClient(client_url)
    steps: list[dict[str, str]] = []
    scenarios: dict[str, Any] = {}

    def record(name: str) -> None:
        steps.append({"name": name, "result": "PASS"})
        print(f"[PASS] {name}", flush=True)

    try:
        for label, driver in (("host", host), ("client", client)):
            driver.wait_until_ready()
            driver.start_session(application)
            wait_for_route(driver, "/", "Choose a table")
            record(f"{label} release instance launched")

        # Unreachable-host join: use a cryptographically valid invite whose listener
        # has been stopped, then prove no partial client session is retained.
        unreachable_host = start_host(host, 43858, "Unreachable Host Scenario")
        unreachable_invite = str(unreachable_host["invite"])
        invoke(host, "stop_host_session")
        join_error = expect_command_error(
            client,
            "join_host_session",
            {
                "request": {
                    "joinPayload": unreachable_invite,
                    "displayName": "Failure Matrix Client",
                }
            },
        )
        if invoke(client, "get_client_session_status") is not None:
            raise AssertionError("unreachable join left a partial client session")
        scenarios["unreachableHostJoin"] = {"error": join_error}
        record("unreachable-host join failed explicitly without retaining a client session")

        # Lobby host loss.
        lobby_host = start_host(host, 43859, "Lobby Host Loss Scenario")
        lobby_client = join_client(client, str(lobby_host["invite"]))
        if lobby_client.get("phase") not in {"waitingForPlayers", "readyCheck"}:
            raise AssertionError(f"client did not join lobby: {lobby_client!r}")
        invoke(host, "stop_host_session")
        lobby_terminal = wait_for_terminal(client)
        lobby_errors = assert_terminal_table(client, str(lobby_terminal["lastError"]))
        scenarios["lobbyHostLoss"] = {
            "status": lobby_terminal,
            **lobby_errors,
        }
        capture(client, evidence_dir, "lobby-host-loss")
        record("lobby host loss became terminal and rejected stale table/action access")
        invoke(client, "leave_client_session")

        # Active-hand host loss.
        hand_host = start_host(host, 43860, "Active Hand Host Loss Scenario")
        hand_client = join_client(client, str(hand_host["invite"]))
        seat_and_start(host, client, hand_client)
        capture(client, evidence_dir, "active-hand-before-host-loss")
        invoke(host, "stop_host_session")
        hand_terminal = wait_for_terminal(client)
        hand_errors = assert_terminal_table(client, str(hand_terminal["lastError"]))
        scenarios["activeHandHostLoss"] = {
            "status": hand_terminal,
            **hand_errors,
        }
        capture(client, evidence_dir, "active-hand-host-loss")
        record("active-hand host loss became terminal and rejected stale table/action access")

        return {
            "result": "PASS",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "scenarios": scenarios,
            "steps": steps,
        }
    except Exception as error:  # noqa: BLE001 - preserve full runtime evidence
        steps.append({"name": "release reconnect failure matrix", "result": "FAIL"})
        failure: dict[str, Any] = {
            "result": "FAIL",
            "application": str(application),
            "applicationSha256": os.environ.get("DESKTOP_POKER_BINARY_SHA256"),
            "error": f"{type(error).__name__}: {error}",
            "scenarios": scenarios,
            "steps": steps,
        }
        for name, driver in (("host", host), ("client", client)):
            try:
                if driver.session_id is not None:
                    capture(driver, evidence_dir, f"failure-{name}")
            except Exception as evidence_error:  # noqa: BLE001
                failure[f"{name}EvidenceCaptureError"] = str(evidence_error)
        raise RuntimeError(json.dumps(failure, indent=2)) from error
    finally:
        client.close()
        host.close()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--application", required=True, type=Path)
    parser.add_argument("--host-driver-url", default="http://127.0.0.1:4624")
    parser.add_argument("--client-driver-url", default="http://127.0.0.1:4634")
    parser.add_argument(
        "--evidence-dir",
        type=Path,
        default=Path("reconnect-failure-evidence"),
    )
    args = parser.parse_args()

    application = args.application.resolve()
    if not application.is_file():
        parser.error(f"application does not exist: {application}")

    result_path = args.evidence_dir / "release-reconnect-failure-result.json"
    try:
        result = run(
            application,
            args.host_driver_url,
            args.client_driver_url,
            args.evidence_dir,
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
